//! The vault list — reading it, and (new) writing it.
//!
//! This lives here rather than in `fm-core` on purpose: *fm-core stays free of environment
//! and configuration concerns*. The list is read from `$FM_VAULTS` / `$XDG_CONFIG_HOME` —
//! environment by definition — and `fm-cli`, the other `fm-core` consumer, never reads it.
//! `backup_status` is the precedent: fm-core supplies facts, this crate assembles env +
//! config into a DTO.
//!
//! It moved *up* from `fm-serve` when [`crate::dispatch`] was extracted: the vault list is
//! state the command surface owns, not something one transport owns. HTTP was simply the
//! only caller at the time. Nothing about it is HTTP-shaped, and a second transport that
//! had to re-read this file would be the fork the extraction exists to prevent.
//!
//! **The list is now app-managed.** It used to be read-only config that only a text editor
//! ever wrote, which is why creating a vault was impossible from inside the app. Writing it
//! is [`save`], and its whole design is about not damaging a file a human may have edited.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// One configured vault: an audience, where it lives, and where its media backs up to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultConfig {
    pub name: String,
    pub path: PathBuf,
    /// This vault's restic repo, or `None` when its media has nowhere to go.
    ///
    /// Per vault because **a restic repo is per repository** — backing up two vaults
    /// means two repos, so "the" restic repo for a set of vaults is not a thing that
    /// exists. A vault without one is not an error: you may well want your lab's notes
    /// shared over git and its media backed up by the lab, not by you.
    pub restic: Option<String>,
}

/// The vault list as it stands, plus the two facts a writer needs.
pub struct VaultList {
    pub vaults: Vec<VaultConfig>,
    /// The file we would write. `None` when this machine has no home or config dir at
    /// all, in which case there is nowhere to persist a vault and creating one must fail
    /// *before* it makes any directories.
    pub path: Option<PathBuf>,
    /// We understood what is on disk (or there is nothing on disk yet). **False means do
    /// not write** — see [`save`].
    pub writable: bool,
}

/// The vault list — **the first app-level config file**, and a deliberate break with
/// `decisions.md`'s "no new config file" (the backup panel keeps the remote in the
/// vault's own `.git/config`). A *set* of vaults cannot live inside any one vault, and
/// `FM_VAULT` is a single path. Breaking that principle on purpose, in one place, beats
/// breaking it by accident later.
///
/// `FM_VAULTS` points at the file; otherwise this OS's per-user config dir (see
/// [`config_dir`]) — `~/.config/formicaria/vaults.json` on Linux.
///
/// `restic` is optional and **per vault** — see [`VaultConfig::restic`].
///
/// ```json
/// { "vaults": [ { "name": "personal", "path": "/home/you/vault", "restic": "/backup/personal" },
///               { "name": "lab",      "path": "/home/you/lab-notes" } ] }
/// ```
///
/// **Absent, with `FM_VAULT` unset, means zero vaults** — the first-run state. There is no
/// longer a default path: `FM_VAULT` used to fall back to the *relative* `"vault"`, so a
/// typo, or a launcher started from another cwd, silently created a working empty vault
/// named after the mistake while your notes appeared to have vanished. Configuration
/// masquerading as capability.
pub fn load() -> VaultList {
    let config = vault_list_path();

    let Some(config) = config else {
        // No home, no config dir: we can still run on an explicit FM_VAULT, but there is
        // nowhere to save a list, so `writable` is false and `create_vault` will say so.
        return VaultList { vaults: single(), path: None, writable: false };
    };

    let text = match std::fs::read_to_string(&config) {
        Ok(t) => t,
        // Absent is the normal first run — we may create it.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return VaultList { vaults: single(), path: Some(config), writable: true };
        }
        // There but unreadable. Do not offer to overwrite what we could not even read.
        Err(e) => {
            eprintln!("error: {} could not be read: {e}", config.display());
            return VaultList { vaults: single(), path: Some(config), writable: false };
        }
    };

    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            // Loud. A malformed vault list that fell back to the single vault would look
            // exactly like "my other vaults vanished", which is a bad hour.
            eprintln!("error: {} is not valid JSON: {e}", config.display());
            eprintln!("  falling back to FM_VAULT. Fix the file and restart.");
            return VaultList { vaults: single(), path: Some(config), writable: false };
        }
    };

    let entries: Vec<VaultConfig> = parsed
        .get("vaults")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(entry).collect())
        .unwrap_or_default();

    if entries.is_empty() {
        let fallback = single();
        if !fallback.is_empty() {
            eprintln!("warning: {} lists no vaults; using FM_VAULT", config.display());
        }
        // Both empty is not a warning — it is the first run.
        return VaultList { vaults: fallback, path: Some(config), writable: true };
    }
    VaultList { vaults: entries, path: Some(config), writable: true }
}

fn entry(v: &Value) -> Option<VaultConfig> {
    let name = v.get("name")?.as_str()?.to_string();
    let path = v.get("path")?.as_str()?;
    Some(VaultConfig {
        name,
        path: PathBuf::from(expand_home(path)),
        restic: v.get("restic").and_then(Value::as_str).filter(|s| !s.is_empty()).map(expand_home),
    })
}

/// The single-vault install: `FM_VAULT`, named after its own directory. **No default** —
/// unset means zero vaults, which is the first-run state and not a path.
fn single() -> Vec<VaultConfig> {
    let Ok(raw) = std::env::var("FM_VAULT") else { return Vec::new() };
    if raw.is_empty() {
        return Vec::new();
    }
    let path = PathBuf::from(expand_home(&raw));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vault".into());
    // The single-vault install kept its restic repo in FM_RESTIC_REPO. Honour it, so an
    // existing setup keeps backing up without touching anything.
    let restic = std::env::var("FM_RESTIC_REPO").ok().filter(|s| !s.is_empty());
    vec![VaultConfig { name, path, restic }]
}

/// Merge the live list into whatever is on disk and write it back atomically.
///
/// **Never a typed serde round-trip.** A `#[serde(flatten)] extra` would preserve unknown
/// keys but reformat and reorder the user's whole file, and would turn "I don't understand
/// this" into "I silently dropped it". We merge into the parsed `Value` tree instead: every
/// entry we did not create keeps its own bytes, and unknown top-level keys survive.
///
/// **Appends only; rewrites no existing entry.** Creation is the only writer there is.
///
/// **Writes the whole live list, not just the new entry.** The sharp bug that fixes:
/// `FM_VAULT` set and no `vaults.json`, then the user creates vault #2. Write only the new
/// entry and [`load`] — which prefers the file — ignores `FM_VAULT` entirely on the next
/// start, so vault #1 vanishes. Serializing the live list materialises the `FM_VAULT` vault
/// into the file first, in order. It does freeze that path into config, which is honest and
/// worth saying out loud to the user.
pub fn save(list: &[VaultConfig], to: &Path) -> Result<(), String> {
    let mut root: Value = match std::fs::read_to_string(to) {
        Ok(t) => serde_json::from_str(&t)
            .map_err(|e| format!("{} is not valid JSON ({e}) — fix it first", to.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(format!("{} could not be read: {e}", to.display())),
    };

    // Belt to `load`'s braces: never overwrite a shape we did not understand.
    let Some(obj) = root.as_object_mut() else {
        return Err(format!("{} is not a JSON object — fix it first", to.display()));
    };
    let existing = obj.entry("vaults").or_insert_with(|| json!([]));
    let Some(arr) = existing.as_array_mut() else {
        return Err(format!("{}'s \"vaults\" is not an array — fix it first", to.display()));
    };

    for v in list {
        let known = arr
            .iter()
            .any(|e| e.get("name").and_then(Value::as_str) == Some(v.name.as_str()));
        if known {
            continue; // theirs. Leave every byte of it alone.
        }
        let mut e = json!({ "name": v.name, "path": absolute(&v.path).to_string_lossy() });
        if let Some(r) = &v.restic {
            e["restic"] = json!(r);
        }
        arr.push(e);
    }

    let mut text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    text.push('\n');
    write_atomic(to, text.as_bytes())
}

/// Temp file in the **same directory** + rename: atomic on one filesystem, so a crash
/// leaves the old list or the new one, never a truncated one. The same discipline as
/// `FileStore::write_atomic`, for the same reason.
fn write_atomic(to: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    let tmp = to.with_extension("json.tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
        f.write_all(bytes).map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
        f.sync_all().map_err(|e| format!("could not flush {}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, to).map_err(|e| format!("could not replace {}: {e}", to.display()))
}

/// Absolute, and canonical where we can manage it. A **relative** path in a config file
/// resolves against the cwd, which is not a thing a config file should do — it is how
/// `FM_VAULT=vault` meant a different vault depending on where you launched from.
pub fn absolute(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir().map(|d| d.join(path)).unwrap_or_else(|_| path.to_path_buf())
}

pub fn vault_list_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FM_VAULTS") {
        return Some(PathBuf::from(p));
    }
    Some(config_dir()?.join("formicaria").join("vaults.json"))
}

/// Where this OS keeps per-user config. Hand-rolled rather than pulling in `dirs`: it is
/// three env lookups, and this crate's whole stance is to add a dependency only when it
/// solves a problem whole.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute()) // the spec says relative XDG values are ignored
            .or_else(|| home().map(|h| h.join(".config")))
    }
    #[cfg(target_os = "macos")]
    {
        home().map(|h| h.join("Library").join("Application Support"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join("AppData").join("Roaming")))
    }
}

/// The user's home, whatever this OS calls it.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// `~` in a config file is what a human writes; nothing else expands it for us. Left
/// alone when there is no home to expand to — a literal `~/notes` fails loudly as a
/// missing path, which beats silently resolving somewhere unexpected.
pub fn expand_home(path: &str) -> String {
    let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) else {
        return path.to_string();
    };
    match home() {
        Some(h) => h.join(rest).to_string_lossy().into_owned(),
        None => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(name: &str, path: &str) -> VaultConfig {
        VaultConfig { name: name.into(), path: PathBuf::from(path), restic: None }
    }

    fn read(p: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    /// The file belongs to the user. They may have put things in it we know nothing about,
    /// at the top level or inside an entry, and a writer that drops what it doesn't
    /// understand is a writer you cannot trust with a config file.
    #[test]
    fn saving_preserves_hand_added_unknown_keys() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");
        std::fs::write(
            &f,
            r#"{ "note": "hi", "vaults": [ { "name": "personal", "path": "/n", "myKey": 1 } ] }"#,
        )
        .unwrap();

        save(&[cfg("personal", "/n"), cfg("lab", "/l")], &f).unwrap();

        let v = read(&f);
        assert_eq!(v["note"], "hi", "an unknown top-level key must survive");
        assert_eq!(v["vaults"][0]["myKey"], 1, "an unknown key inside an entry must survive");
        assert_eq!(v["vaults"][1]["name"], "lab", "and the new vault is appended");
    }

    /// Append-only. An entry we did not create is not ours to reformat, reorder, or
    /// "correct" — even when its path disagrees with what we hold in memory.
    #[test]
    fn saving_never_rewrites_an_existing_entry() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");
        std::fs::write(&f, r#"{"vaults":[{"name":"personal","path":"~/somewhere-else"}]}"#).unwrap();

        save(&[cfg("personal", "/a/totally/different/path")], &f).unwrap();

        assert_eq!(read(&f)["vaults"][0]["path"], "~/somewhere-else");
        assert_eq!(read(&f)["vaults"].as_array().unwrap().len(), 1, "no duplicate");
    }

    /// Overwriting a hand-edited list we could not parse is the loss `load`'s
    /// malformed-JSON warning exists to shout about. Refuse, and leave the bytes alone.
    #[test]
    fn saving_refuses_a_file_it_did_not_understand() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");

        for bad in [r#"{{"#, r#"[1,2]"#, r#"{"vaults": "not an array"}"#] {
            std::fs::write(&f, bad).unwrap();
            assert!(save(&[cfg("lab", "/l")], &f).is_err(), "must refuse: {bad}");
            assert_eq!(std::fs::read_to_string(&f).unwrap(), bad, "and must not touch it");
        }
    }

    /// The sharp one. An `FM_VAULT`-only install has no `vaults.json`; the moment we write
    /// one, `load` starts preferring it and stops consulting `FM_VAULT`. If we wrote only
    /// the vault being created, the original would vanish on the next start.
    #[test]
    fn an_fm_vault_only_install_is_materialised_into_the_list_on_first_write() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("nested").join("vaults.json"); // parent does not exist yet

        save(&[cfg("personal", "/from-fm-vault"), cfg("lab", "/l")], &f).unwrap();

        let v = read(&f);
        assert_eq!(v["vaults"][0]["name"], "personal", "the FM_VAULT vault is written first");
        assert_eq!(v["vaults"][1]["name"], "lab");
    }

    /// A relative path in a config file resolves against the cwd — exactly the bug that
    /// made `FM_VAULT=vault` mean a different vault depending on where you launched from.
    #[test]
    fn saved_paths_are_absolute() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");

        save(&[cfg("rel", "some/relative/dir")], &f).unwrap();

        let p = read(&f)["vaults"][0]["path"].as_str().unwrap().to_string();
        assert!(Path::new(&p).is_absolute(), "wrote a relative path: {p}");
    }

    #[test]
    fn restic_is_written_only_when_set() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");
        let with = VaultConfig { restic: Some("/backup/lab".into()), ..cfg("lab", "/l") };

        save(&[cfg("personal", "/n"), with], &f).unwrap();

        let v = read(&f);
        assert!(v["vaults"][0].get("restic").is_none(), "no restic → no key, not null");
        assert_eq!(v["vaults"][1]["restic"], "/backup/lab");
    }
}
