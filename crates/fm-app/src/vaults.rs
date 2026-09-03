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
        path: resolve_path(path),
        restic: v.get("restic").and_then(Value::as_str).filter(|s| !s.is_empty()).map(expand_home),
    })
}

/// A stored path, made into a real one: expand `@root/`, then `~`, then — **only on a phone, and
/// only when the result is not there** — look for the vault under the current managed root.
///
/// That last step is the migration, and it has exactly one job: an install written *before*
/// [`ROOT_MARKER`] existed holds an absolute container path that a re-sign has since invalidated.
/// [`save`] never rewrites an entry it did not create (*"theirs. Leave every byte of it alone"*),
/// so those entries would stay absolute and stay broken forever. Healing on read fixes them
/// without touching the file.
///
/// **Deliberately narrow.** It runs only where [`vault_root`] is `Some` — a phone — only when the
/// stored path is missing, and only adopts a **directory** that is actually there. On a desktop it
/// is not reachable at all, so a user whose external drive is unmounted still gets the honest
/// missing path they had before rather than a surprise vault somewhere else.
fn resolve_path(raw: &str) -> PathBuf {
    let stored = PathBuf::from(expand_home(&expand_root(raw)));
    if stored.exists() {
        return stored;
    }
    let (Some(root), Some(leaf)) = (vault_root(), stored.file_name()) else {
        return stored;
    };
    let candidate = root.join(leaf);
    if candidate.is_dir() {
        candidate
    } else {
        stored
    }
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
        let mut e = json!({ "name": v.name, "path": persist_path(&v.path) });
        if let Some(r) = &v.restic {
            e["restic"] = json!(r);
        }
        arr.push(e);
    }

    let mut text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    text.push('\n');
    write_atomic(to, text.as_bytes())
}

/// Set (or clear) **one** key of **one** entry: this vault's restic repository.
///
/// [`save`] deliberately leaves a known entry's every byte alone — *"theirs"* — which is right for
/// a function whose job is appending vaults, and is exactly why it cannot be the one that changes
/// a setting on a vault that already exists. Without this, giving an existing vault a media-backup
/// repo meant editing `vaults.json` by hand, which Settings said in as many words.
///
/// So: a narrow writer, not a relaxed [`save`]. It rewrites the `restic` key of the named entry and
/// nothing else — every other key of that entry, every other entry, and any part of the file this
/// app does not understand are carried through untouched, and the same atomic replace protects a
/// crash mid-write. `None` removes the key rather than writing `null`, so "no repo" reads the same
/// way it always has.
///
/// The entry has to exist: this is a setting on a vault, and inventing one from a name would let a
/// typo create a phantom. Call [`save`] with the live list first — which is what materialises an
/// `FM_VAULT`-only install into the file — and then this.
pub fn set_restic(name: &str, repo: Option<&str>, to: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(to)
        .map_err(|e| format!("{} could not be read: {e}", to.display()))?;
    let mut root: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not valid JSON ({e}) — fix it first", to.display()))?;
    let entry = root
        .as_object_mut()
        .and_then(|o| o.get_mut("vaults"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("{}'s \"vaults\" is not an array — fix it first", to.display()))?
        .iter_mut()
        .find(|e| e.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("no vault named '{name}' in {}", to.display()))?;
    let obj = entry
        .as_object_mut()
        .ok_or_else(|| format!("the '{name}' entry in {} is not an object", to.display()))?;
    match repo.map(str::trim).filter(|r| !r.is_empty()) {
        Some(r) => {
            obj.insert("restic".into(), json!(r));
        }
        None => {
            obj.remove("restic");
        }
    }
    let mut out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    out.push('\n');
    write_atomic(to, out.as_bytes())
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
    // Anywhere else — Android first, and the reason this arm exists at all. Without it the
    // three `cfg`s above are all false, the block has no tail expression, and the crate does
    // not *compile* for `target_os = "android"`. There is no ambient per-user config dir on a
    // phone: the platform hands the app its own data directory at runtime, so the shell passes
    // it in the same way `FM_VAULTS` already overrides this file's location on desktop.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("FM_CONFIG_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute()) // a relative one would follow the cwd, which on a phone means nothing
    }
}

/// The one directory this installation is allowed to put vaults in, or `None` when the user
/// picks their own locations.
///
/// **`Some` is the phone; `None` is the desktop, and that asymmetry is the design.** On a
/// desktop a vault is a folder you chose — `~/notes`, or a project repo you are adopting — and
/// taking that away would break the thing that makes a vault *yours*. A phone has no such
/// place: there is no `$HOME`, no shell to `mkdir` with, no file manager that can reach an
/// app's storage, and no meaningful path a user could type. The platform hands the app one
/// private directory and that is the whole world it may write to.
///
/// So the shell sets `FM_VAULT_ROOT` and everything lands inside it. Nothing else on the device
/// writes there, and it is removed when the app is uninstalled — which is the honest cost of a
/// sandbox, and the reason a phone vault wants a remote or a backup pointed at it.
pub fn vault_root() -> Option<PathBuf> {
    std::env::var_os("FM_VAULT_ROOT")
        .map(PathBuf::from)
        // A relative root would follow the cwd, which on a phone means nothing at all.
        .filter(|p| p.is_absolute())
}

/// Where a vault called `name` goes inside the managed root.
///
/// **The join happens here, never in the UI.** A browser computing `<root>/<name>` is a browser
/// one `../` away from writing outside the sandbox, and "the frontend promised not to" is not
/// containment. The name becomes exactly one path segment or this refuses.
///
/// The mapping is deliberately lossy and deliberately not shown to the user as *the* name: what
/// they typed is the vault's name in the app, and this is only the folder it lives in. Two
/// vaults whose names differ only in punctuation would collide here, which `check_path` catches
/// as a taken path — a clear refusal rather than two vaults quietly sharing a directory.
pub fn contained_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    let slug: String = name
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    // Collapse runs and trim the separator, so "my notes!!" is `my-notes`, not `my-notes--`.
    let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() {
        return Err("that name has no letters or digits in it, so there is no folder to make \
                    from it — try adding some"
            .into());
    }
    Ok(root.join(slug))
}

/// The user's home, whatever this OS calls it.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// The prefix that means *"inside whatever this installation's managed root is right now"*.
///
/// **This exists because an iOS container path is not stable.** A sideloaded app is re-signed
/// every 7 days and reinstalled; the data survives, the container UUID does not, so an absolute
/// `/var/mobile/Containers/Data/Application/<UUID>/…` written last week names nothing this week.
/// The vault is still on disk and the app opens to a first-run screen — the notes are not
/// corrupted, they are *unreferenced*, which is worse because it looks like deletion.
///
/// Desktop is untouched by construction: [`vault_root`] is `Some` only where the shell sets
/// `FM_VAULT_ROOT`, i.e. only on a phone. A desktop vault is a folder the user chose and keeps
/// being written absolute.
pub const ROOT_MARKER: &str = "@root/";

/// `@root/` in a config file means the managed root, resolved at read time. The direct analogue
/// of [`expand_home`], and left alone for the same reason when there is nothing to expand to: a
/// literal `@root/notes` fails loudly as a missing path, which beats resolving somewhere
/// unexpected. A bare relative path would have followed the process working directory — the very
/// bug [`absolute`] exists to prevent — so the marker states what it is relative *to*.
pub fn expand_root(path: &str) -> String {
    let Some(rest) = path.strip_prefix(ROOT_MARKER) else {
        return path.to_string();
    };
    match vault_root() {
        Some(r) => r.join(rest).to_string_lossy().into_owned(),
        None => path.to_string(),
    }
}

/// How a vault's path is written to `vaults.json`.
///
/// Root-relative when this installation has a managed root and the vault lives inside it —
/// which is every phone vault, since [`contained_path`] is the only thing that makes one.
/// Absolute otherwise, which is every desktop vault and any phone vault somehow outside the
/// root. **Both forms are read back by [`entry`], so this is safe to change under an existing
/// file**: nothing rewrites entries it did not create.
fn persist_path(path: &Path) -> String {
    let abs = absolute(path);
    if let Some(root) = vault_root() {
        // `absolute` canonicalises where it can, and the root may be a symlink on a phone, so
        // canonicalise the root the same way before comparing — otherwise every path looks
        // outside a root it is plainly inside.
        let root = absolute(&root);
        if let Ok(rel) = abs.strip_prefix(&root) {
            // An empty tail would mean "the root is the vault", which `contained_path` cannot
            // produce; writing `@root/` for it would round-trip to the root itself, so refuse
            // to be clever and keep the absolute form.
            if rel.as_os_str().len() > 0 {
                return format!("{ROOT_MARKER}{}", rel.to_string_lossy());
            }
        }
    }
    abs.to_string_lossy().into_owned()
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

/// **Serialises every test that touches `FM_VAULT_ROOT`** — and it lives here, beside the code,
/// rather than inside either test module, because **this file has two of them** (`tests` and
/// `contained`) and the variable is process-global to the whole binary. A lock in one module would
/// not protect the other, which is precisely the shape of the bug it exists to prevent.
///
/// `std::env::set_var` is process-global while cargo runs a binary's tests on concurrent threads,
/// so a test that sets the root races every test that reads it — and since [`vault_root`] now
/// decides how a path is *persisted*, that is most of this file. The identical race in
/// `fm-core/tests/git_transport.rs` failed `pixi run ci` about 1 run in 8 until it was locked; this
/// is the same fix applied before the flake rather than after. The repo's other instances of the
/// idiom are `fm-app/tests/backup_records_everything.rs` and `fm-app/src/secrets.rs`.
#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Hold [`ENV_LOCK`], set `FM_VAULT_ROOT`, and always unset it again — including on panic, which a
/// bare `set_var`/`remove_var` pair wrapped around an assertion does not do. Poisoning is recovered
/// from, so one failed assertion reports itself instead of turning every sibling into an unwrap
/// panic on a poisoned mutex.
#[cfg(test)]
pub(crate) struct RootGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

#[cfg(test)]
impl RootGuard {
    fn set(root: &Path) -> Self {
        let g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("FM_VAULT_ROOT", root);
        RootGuard(g)
    }
    /// The desktop: no managed root at all.
    fn none() -> Self {
        let g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("FM_VAULT_ROOT");
        RootGuard(g)
    }
}

#[cfg(test)]
impl Drop for RootGuard {
    fn drop(&mut self) {
        std::env::remove_var("FM_VAULT_ROOT");
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
        let _env = RootGuard::none(); // the desktop, where there is no managed root
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("vaults.json");

        save(&[cfg("rel", "some/relative/dir")], &f).unwrap();

        let p = read(&f)["vaults"][0]["path"].as_str().unwrap().to_string();
        assert!(Path::new(&p).is_absolute(), "wrote a relative path: {p}");
    }

    /// **G5, and the whole point of it.** A phone vault must be found again after the app's
    /// container path changes — which on a sideloaded iOS build happens on every 7-day re-sign.
    #[test]
    fn a_phone_vault_is_found_again_under_a_new_container() {
        let old = tempfile::tempdir().unwrap(); // the container as it was last week
        let new = tempfile::tempdir().unwrap(); // the same app, re-signed, new UUID
        let f = old.path().join("vaults.json");
        std::fs::create_dir_all(old.path().join("notes")).unwrap();

        {
            let _env = RootGuard::set(old.path());
            save(&[cfg("notes", &old.path().join("notes").to_string_lossy())], &f).unwrap();
            let written = read(&f)["vaults"][0]["path"].as_str().unwrap().to_string();
            assert_eq!(written, "@root/notes", "a managed vault must persist root-relative");
        }

        // The container moves, taking the data with it — which is what iOS actually does.
        std::fs::create_dir_all(new.path().join("notes")).unwrap();
        let _env = RootGuard::set(new.path());
        let v = entry(&read(&f)["vaults"][0]).expect("the entry must still parse");
        assert_eq!(v.path, new.path().join("notes"), "the vault must resolve against the new root");
        assert!(v.path.is_dir(), "and it must be the directory that actually exists");
    }

    /// The desktop is untouched: no managed root means the absolute path it always wrote.
    #[test]
    fn a_desktop_vault_still_persists_absolute() {
        let _env = RootGuard::none();
        let d = tempfile::tempdir().unwrap();
        let vault = d.path().join("notes");
        std::fs::create_dir_all(&vault).unwrap();
        let f = d.path().join("vaults.json");

        save(&[cfg("notes", &vault.to_string_lossy())], &f).unwrap();

        let p = read(&f)["vaults"][0]["path"].as_str().unwrap().to_string();
        assert!(!p.starts_with(ROOT_MARKER), "a desktop vault must not be written root-relative");
        assert!(Path::new(&p).is_absolute(), "and it must stay absolute: {p}");
    }

    /// A phone vault that is somehow *outside* the managed root keeps its absolute path — the
    /// marker means "inside the root", and writing it for something else would relocate the vault.
    #[test]
    fn a_path_outside_the_root_is_not_made_relative() {
        let root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(elsewhere.path().join("notes")).unwrap();
        let f = root.path().join("vaults.json");

        let _env = RootGuard::set(root.path());
        save(&[cfg("notes", &elsewhere.path().join("notes").to_string_lossy())], &f).unwrap();

        let p = read(&f)["vaults"][0]["path"].as_str().unwrap().to_string();
        assert!(!p.starts_with(ROOT_MARKER), "only a vault inside the root is root-relative: {p}");
    }

    /// **The migration.** An entry written before the marker existed holds an absolute container
    /// path that no longer resolves. `save` never rewrites an entry it did not create, so such an
    /// entry would stay broken forever — healing on read is what rescues an install made from a
    /// pre-G5 build.
    #[test]
    fn a_stale_absolute_container_path_is_healed_to_the_current_root() {
        let new = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(new.path().join("notes")).unwrap();
        let stale = "/var/mobile/Containers/Data/Application/DEAD-BEEF/vaults/notes";

        let _env = RootGuard::set(new.path());
        let v = entry(&json!({ "name": "notes", "path": stale })).unwrap();

        assert_eq!(v.path, new.path().join("notes"), "a missing container path must be healed");
    }

    /// The heal must not fire on a desktop, where a missing path means a drive is unmounted and
    /// silently substituting a different directory would be worse than the honest failure.
    #[test]
    fn a_missing_desktop_path_is_left_alone() {
        let _env = RootGuard::none();
        let missing = "/definitely/not/here/notes";

        let v = entry(&json!({ "name": "notes", "path": missing })).unwrap();

        assert_eq!(v.path, PathBuf::from(missing), "a desktop path must be reported as it is");
    }

    /// And it must not fire when the stored path is perfectly fine, even on a phone — otherwise a
    /// vault outside the root would be quietly relocated into it.
    #[test]
    fn a_resolvable_path_is_never_healed() {
        let root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let real = elsewhere.path().join("notes");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::create_dir_all(root.path().join("notes")).unwrap(); // a decoy with the same leaf

        let _env = RootGuard::set(root.path());
        let v = entry(&json!({ "name": "notes", "path": real.to_string_lossy() })).unwrap();

        assert_eq!(v.path, real, "a path that resolves must be used as-is, decoy or not");
    }

    /// The narrow writer earns its existence by what it does **not** touch: `save` refuses to
    /// rewrite a known entry at all, so a relaxed `save` would have been the alternative — and
    /// that is the version that quietly reformats a file someone hand-edited.
    #[test]
    fn setting_a_restic_repo_edits_one_key_and_leaves_the_rest_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vaults.json");
        std::fs::write(
            &path,
            r#"{"vaults":[{"name":"personal","path":"/p","note":"mine"},
                          {"name":"lab","path":"/l","restic":"/backup/lab"}],
                "something_we_do_not_understand": 7}"#,
        )
        .unwrap();

        set_restic("personal", Some("/backup/personal"), &path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["vaults"][0]["restic"], "/backup/personal");
        assert_eq!(v["vaults"][0]["note"], "mine", "an unrelated key of that entry survived");
        assert_eq!(v["vaults"][1]["restic"], "/backup/lab", "another vault was not touched");
        assert_eq!(v["something_we_do_not_understand"], 7, "a shape we don't own survived");

        // Clearing removes the key, rather than writing `null` — "no repo" must read the same way
        // it does for a vault that never had one.
        set_restic("lab", None, &path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["vaults"][1].get("restic").is_none());

        // A name that is not there is a typo, not an invitation to invent a vault.
        assert!(set_restic("ghost", Some("/backup/ghost"), &path).is_err());
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

#[cfg(test)]
mod contained {
    use super::*;

    /// The ordinary case: a name becomes one folder inside the root.
    #[test]
    fn a_name_becomes_one_segment_inside_the_root() {
        let root = Path::new("/data/app/vaults");
        assert_eq!(contained_path(root, "notes").unwrap(), root.join("notes"));
        assert_eq!(contained_path(root, "My Notes").unwrap(), root.join("My-Notes"));
        assert_eq!(contained_path(root, "lab-2026").unwrap(), root.join("lab-2026"));
    }

    /// **The whole reason this function exists.** A name is user input, and on a phone it is
    /// the *only* input — so anything that could climb out of the sandbox must become an
    /// ordinary segment instead. Every one of these stays under the root.
    #[test]
    fn no_name_can_escape_the_root() {
        let root = Path::new("/data/app/vaults");
        for hostile in [
            "../../etc",
            "..",
            "../sibling",
            "/absolute",
            "a/b",
            "a\\b",
            "....//....//etc",
            "~/elsewhere",
            "notes/../../..",
        ] {
            // Two acceptable outcomes, and refusal is the stronger one: a name made entirely
            // of traversal (`..`) has nothing left once punctuation is stripped, so it is
            // rejected rather than silently renamed. Anything that *does* produce a folder
            // must land inside the root as exactly one segment.
            let Ok(got) = contained_path(root, hostile) else { continue };
            assert!(
                got.starts_with(root),
                "{hostile:?} escaped to {}",
                got.display()
            );
            assert_eq!(
                got.components().count(),
                root.components().count() + 1,
                "{hostile:?} became more than one segment: {}",
                got.display()
            );
            assert!(
                !got.components().any(|c| c.as_os_str() == ".." || c.as_os_str() == "."),
                "{hostile:?} kept a traversal component: {}",
                got.display()
            );
        }
    }

    /// A name with nothing to make a folder from is refused rather than silently becoming the
    /// root itself — which would put a vault's notes directly among every other vault's.
    #[test]
    fn a_name_with_no_usable_characters_is_refused() {
        let root = Path::new("/data/app/vaults");
        for empty in ["", "   ", "...", "///", "!!!", "..", "/"] {
            assert!(contained_path(root, empty).is_err(), "must refuse {empty:?}");
        }
    }

    /// Only an absolute root is a root. A relative one would follow the process working
    /// directory, which on a phone is not a place at all.
    #[test]
    fn a_relative_root_is_not_a_root() {
        // Takes the lock like every other test that touches this variable: it is process-global,
        // and `vault_root()` now decides how a path is persisted, so an unguarded set here would
        // race the save/resolve tests above.
        let _env = super::RootGuard::set(Path::new("vaults"));
        assert_eq!(vault_root(), None, "a relative root must be ignored");
        std::env::set_var("FM_VAULT_ROOT", "/data/app/vaults");
        assert_eq!(vault_root(), Some(PathBuf::from("/data/app/vaults")));
        std::env::remove_var("FM_VAULT_ROOT");
        assert_eq!(vault_root(), None, "unset is the desktop, where the user chooses");
    }
}
