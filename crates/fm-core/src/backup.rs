//! Backup via restic (subprocess) — dedup + encryption + integrity + remotes,
//! none of which we should reimplement. `restic check --read-data` is the
//! off-site bit-rot scrub that complements `fm verify --scrub`.
//!
//! The disposable, per-machine index (`index.sqlite`) and regenerable thumbnails
//! (`derived/`) are excluded: they rebuild from the notes and blobs, and the
//! index in particular must never travel between machines (the DB-corruption-by-
//! sync lesson).
//!
//! **Only the vault's own directories are snapshotted** — its notes dir (wherever
//! `vault.json` puts it) and `blobs/`. Not the vault root: a vault is increasingly a repo you
//! already have, and the root then also holds your source, your `.env` and a `.git`, none of
//! which belong in a restic repo that may be a lab's rather than yours. Naming what is ours
//! beats excluding what is not, because the set to exclude has no end.

use crate::StoreError;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Is restic on this machine?
///
/// **An optional feature's dependency, declared rather than assumed** — the same shape as
/// [`crate::git::available`]. The core (notes, scheduling, board, search) is `FileStore`
/// over Markdown files and spawns nothing; media backup is a *feature*, and a feature
/// whose tool is absent should simply not be offered. Without this the panel gates its
/// checkbox on "repo configured and password set" — which is not the same claim as "this
/// can run", so it enables, you tick it, and restic isn't there.
///
/// Cached: restic does not appear mid-run, and this is asked on every status read.
pub fn available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("restic").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
    })
}

fn restic(repo: &Path, password: &str) -> Command {
    let mut c = Command::new("restic");
    c.arg("--repo").arg(repo).env("RESTIC_PASSWORD", password);
    c
}

/// Initialize the repo if it isn't one yet. Returns true if it was created.
pub fn ensure_repo(repo: &Path, password: &str) -> Result<bool, StoreError> {
    // `cat config` succeeds only against an initialized repo.
    let probe = restic(repo, password).arg("cat").arg("config").output().map_err(spawn)?;
    if probe.status.success() {
        return Ok(false);
    }
    let out = restic(repo, password).arg("init").output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("restic init", &out));
    }
    Ok(true)
}

/// Snapshot the vault's **own** directories — notes and blobs — initializing the repo on
/// first run.
///
/// **Not the vault root**, and that is the whole point of this function's shape. A vault is
/// increasingly a repo you already have: point one at a project and the root also holds your
/// source, your `.env`, your `data/` and a `.git`. Snapshotting the root put all of it into
/// whatever restic repo the vault names — which for a lab's shared vault is not your repo.
/// Excluding `index.sqlite`/`derived` was never enough, because it enumerated what to leave
/// out of an unbounded set.
///
/// So this takes what we know is ours instead. `notes` comes from the store (a `vault.json`
/// may put it in `docs/`), `blobs` is fixed. Anything else under the vault root — including
/// a project's own files — is the user's to back up their own way.
///
/// **Answers what it did**, which it did not use to. Returning unit meant a caller could say a
/// snapshot had been taken and nothing whatever about what was in it — so the panel printed
/// "notes and attachments" over every vault, including the ones that have no attachments yet,
/// and a person watching a backup they cannot see had no way to tell a full one from an empty
/// one. See [`Backed`].
pub fn backup(vault: &Path, repo: &Path, password: &str) -> Result<Backed, StoreError> {
    ensure_repo(repo, password)?;
    // Asked here rather than taken as an argument, so the three callers cannot drift on
    // *which* directories are ours — `vault.json` may put the notes in `docs/`.
    let desc = crate::descriptor::Descriptor::read(vault)?;
    let notes = desc.notes_dir(vault);
    // A vault with no blobs yet is normal; restic errors on a path that does not exist.
    let blobs = vault.join("blobs");
    // Read once and reported, rather than asked again below: what went into the snapshot and
    // what the caller is told went into it must be the same answer, and two `exists()` calls
    // either side of a subprocess are two chances for them not to be.
    let (has_notes, has_blobs) = (notes.exists(), blobs.exists());
    let mut paths: Vec<&Path> = Vec::new();
    if has_notes {
        paths.push(&notes);
    }
    if has_blobs {
        paths.push(&blobs);
    }
    if paths.is_empty() {
        return Err(StoreError::Io(
            "nothing to back up: this vault has no notes or blobs directory yet".into(),
        ));
    }
    // `--json` for the summary line alone. It also moves restic's errors into JSON objects on
    // stderr, which is why the failure below goes through [`failed_json`] — passing that
    // through raw would hand the user a serialized struct where a sentence used to be.
    let out = restic(repo, password)
        .arg("backup")
        .arg("--json")
        .args(&paths)
        .arg("--tag")
        .arg("fm")
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed_json("restic backup", &out));
    }
    Ok(Backed {
        // The directory's own name, not its path: this is for a sentence in front of a person,
        // and a vault that keeps its notes in `docs/` should say `docs`.
        notes_dir: has_notes.then(|| {
            desc.notes.clone().unwrap_or_else(|| PathBuf::from("notes")).display().to_string()
        }),
        blobs: has_blobs,
        contents: summary(&out.stdout),
    })
}

/// restic's own account of the snapshot it just wrote.
///
/// Every number is restic's or the whole thing is absent — see [`summary`]. There is no field
/// here this crate computes, because a count we worked out ourselves sitting beside counts
/// restic gave us is exactly how a surface starts being confidently wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contents {
    /// restic's **short** id for the snapshot, so this names it the way [`latest`] does.
    pub id: String,
    /// Files added, changed, and found already in the repository. The third is the interesting
    /// one on a healthy vault: it is how much of this backup was free.
    pub files_new: u64,
    pub files_changed: u64,
    pub files_unmodified: u64,
    /// Bytes read out of the vault.
    pub bytes_processed: u64,
    /// Bytes the repository actually grew by. Deduplication is why this is normally a fraction
    /// of the line above, and why a daily snapshot of a 4 GB vault costs almost nothing.
    pub bytes_added: u64,
}

/// What one [`backup`] run put in the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backed {
    /// The notes directory that went in, by its own name — `notes` unless a `vault.json` moved
    /// it. `None` when the vault has none yet, which is a real state: a vault holding only
    /// `blobs/` is still worth snapshotting, and [`backup`] does.
    pub notes_dir: Option<String>,
    /// Whether `blobs/` existed and went in. **A vault with no attachments yet is normal**, and
    /// a surface that says "notes and attachments" over one is overstating what it did — which
    /// is the one thing a backup surface must never do.
    pub blobs: bool,
    /// What restic said the snapshot contains. `None` means the snapshot was written and restic
    /// did not describe it — **not** that it was empty. Kept apart for the same reason
    /// [`latest`] keeps *never backed up* apart from *this machine cannot tell you*.
    pub contents: Option<Contents>,
}

/// restic's `summary` line out of a `--json` run.
///
/// **Absent rather than an error.** The snapshot is already written by the time this is read, so
/// refusing the backup because a line would not parse would turn a snapshot that exists into a
/// reported failure — the worst answer available, and the one that teaches a user to distrust
/// the panel. An unrecognised stream means the caller is told *nothing* about the contents.
///
/// All of restic's numbers or none of them, for the same reason: a missing field defaulted to
/// zero reads as an empty vault, and there is no way to tell it from one.
fn summary(stdout: &[u8]) -> Option<Contents> {
    let text = String::from_utf8_lossy(stdout);
    text.lines().find_map(|line| {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        if v.get("message_type").and_then(|m| m.as_str()) != Some("summary") {
            return None;
        }
        let num = |k: &str| v.get(k).and_then(serde_json::Value::as_u64);
        Some(Contents {
            // restic's own `short_id` — which `latest` reads — is the first 8 characters of the
            // full id, and the two commands have to name the same snapshot the same way.
            id: v.get("snapshot_id").and_then(|v| v.as_str())?.chars().take(8).collect(),
            files_new: num("files_new")?,
            files_changed: num("files_changed")?,
            files_unmodified: num("files_unmodified")?,
            bytes_processed: num("total_bytes_processed")?,
            bytes_added: num("data_added")?,
        })
    })
}

/// Restore the latest snapshot into `dest` (restic recreates the source tree
/// there). This is the half people skip — an untested backup is not a backup.
pub fn restore(repo: &Path, password: &str, dest: &Path) -> Result<(), StoreError> {
    let out = restic(repo, password)
        .arg("restore")
        .arg("latest")
        .arg("--target")
        .arg(dest)
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("restic restore", &out));
    }
    Ok(())
}

/// One snapshot, as restic describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub id: String,
    pub time: String,
    /// The **source** paths, absolute on whatever machine took the snapshot.
    pub paths: Vec<PathBuf>,
}

/// The most recent `fm`-tagged snapshot, or `None` for a repo that has none.
///
/// Tag-filtered on purpose. A restic repo is frequently not ours — [`backup`] says so — and
/// restoring a vault from someone's photo backup because it happened to be the latest
/// snapshot is the kind of confident wrongness that costs a directory.
pub fn latest(repo: &Path, password: &str) -> Result<Option<Snapshot>, StoreError> {
    let out = restic(repo, password)
        .args(["snapshots", "--json", "--tag", "fm", "--latest", "1"])
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("restic snapshots", &out));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        StoreError::Io(format!(
            "restic returned something that is not JSON ({e}) — is {} really a restic repo?",
            repo.display()
        ))
    })?;
    let Some(first) = parsed.as_array().and_then(|a| a.first()) else {
        return Ok(None);
    };
    let paths: Vec<PathBuf> = first
        .get("paths")
        .and_then(|p| p.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(PathBuf::from).collect())
        .unwrap_or_default();
    if paths.is_empty() {
        return Err(StoreError::Io(
            "the latest formicaria snapshot records no paths, so there is nothing to restore \
             from it"
                .into(),
        ));
    }
    Ok(Some(Snapshot {
        id: first
            .get("short_id")
            .or_else(|| first.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        time: first.get("time").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
        paths,
    }))
}

/// What a restore actually produced, so the caller can tell the user the truth about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub from: Snapshot,
    /// The notes directory's name as it was on the source machine — `notes` unless a
    /// `vault.json` there put it somewhere else.
    pub notes_dir: String,
    pub had_blobs: bool,
}

/// Restore the latest `fm` snapshot into `dest` **as a vault**, not as a copy of someone
/// else's filesystem tree.
///
/// The difference is the whole function. [`restore`] hands restic a target and restic
/// faithfully recreates the absolute source path underneath it, so a plain restore of Ada's
/// vault gives you `dest/home/ada/vault/notes/…` — correct as a *backup* restore, useless as
/// a vault. Here the snapshot's own recorded paths say where the source root was, and the
/// tree is lifted out of that prefix into `dest`.
///
/// **What arrives is a vault with no history.** [`backup`] snapshots the vault's own
/// directories and deliberately not its root, so `.git` was never in there to come back. That
/// is the honest shape of restic as an acquisition method: it returns your notes and your
/// media, not your history and not a collaboration. Anything that needs those needs git.
///
/// **Refuses to overwrite — on a name collision, not on a non-empty destination.** Every entry is
/// moved into `dest` only if nothing of that name is already there, and the whole set is checked
/// before anything moves, so a refusal leaves the destination exactly as it was. That is what stops
/// two vaults merging into one: every vault has a notes directory, so two of them always collide.
///
/// A destination holding *unrelated* content — a `README.md`, a `.git`, the repo you are adopting
/// as a vault — is not refused, and should not be. This sentence used to say that pointing it at
/// "a directory with content in it" fails, which overstated the guard by enough to mislead someone
/// deciding whether it was safe to run (corrected 2026-09-07;
/// `a_restore_lands_beside_unrelated_files_and_refuses_only_on_a_collision` pins both halves).
pub fn restore_vault(repo: &Path, password: &str, dest: &Path) -> Result<Restored, StoreError> {
    let snap = latest(repo, password)?.ok_or_else(|| {
        StoreError::Io(format!(
            "{} has no formicaria snapshot in it — this repo has never backed up a vault",
            repo.display()
        ))
    })?;
    let root = common_parent(&snap.paths).ok_or_else(|| {
        StoreError::Io(
            "the snapshot's paths share no common folder, so we cannot tell what the vault \
             root was"
                .into(),
        )
    })?;

    // Staging *inside* `dest` so the move below is a rename on one filesystem rather than a
    // second full copy of what may be gigabytes of blobs.
    let staging = dest.join(".fm-restoring");
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(io)?;
    }
    std::fs::create_dir_all(&staging).map_err(io)?;

    let outcome = (|| {
        restore(repo, password, &staging)?;
        let src = staging.join(strip_prefix(&root));
        if !src.is_dir() {
            return Err(StoreError::Io(format!(
                "restic restored the snapshot but {} was not in it — the repo may have been \
                 written by a different version",
                root.display()
            )));
        }
        // Refuse *before* moving anything: a half-moved vault is worse than a failed one.
        let mut entries: Vec<PathBuf> = Vec::new();
        for e in std::fs::read_dir(&src).map_err(io)? {
            let e = e.map_err(io)?;
            let target = dest.join(e.file_name());
            if target.exists() {
                return Err(StoreError::Io(format!(
                    "{} already exists — restoring here would overwrite it, so nothing was \
                     changed",
                    target.display()
                )));
            }
            entries.push(e.path());
        }
        for from in entries {
            let to = dest.join(from.file_name().expect("read_dir yields named entries"));
            std::fs::rename(&from, &to).map_err(io)?;
        }
        Ok(())
    })();

    // Clean up the staging tree whichever way that went. Best-effort: failing to remove it
    // must not turn a good restore into a reported failure.
    let _ = std::fs::remove_dir_all(&staging);
    outcome?;

    let names: Vec<String> = snap
        .paths
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    Ok(Restored {
        notes_dir: names.iter().find(|n| *n != "blobs").cloned().unwrap_or_else(|| "notes".into()),
        had_blobs: names.iter().any(|n| n == "blobs"),
        from: snap,
    })
}

/// The deepest folder that contains all of `paths` — the vault root on the source machine.
///
/// Pure, and separated from the restore so it can be tested without a restic repo: this is
/// the piece that decides which directory gets lifted, and getting it wrong moves the wrong
/// tree.
fn common_parent(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut it = paths.iter();
    let mut common: Vec<std::path::Component> = it.next()?.parent()?.components().collect();
    for p in it {
        let theirs: Vec<_> = p.parent()?.components().collect();
        let keep = common.iter().zip(&theirs).take_while(|(a, b)| a == b).count();
        common.truncate(keep);
    }
    // **A shared prefix of just `/` is not a vault root.** Two snapshot paths with nothing in
    // common still share the filesystem root, and `!is_empty()` accepts that — it leaves
    // `[RootDir]`, which would lift the entire restored tree and treat the machine's root as
    // the vault. Requiring a named folder is the difference between "we found the root" and
    // "we found nothing and said `/`".
    common
        .iter()
        .any(|c| matches!(c, std::path::Component::Normal(_)))
        .then(|| common.iter().collect())
}

/// An absolute path as restic nests it under a restore target: `/home/ada/v` → `home/ada/v`.
fn strip_prefix(root: &Path) -> PathBuf {
    root.components().filter(|c| matches!(c, std::path::Component::Normal(_))).collect()
}

/// Verify repo integrity. `read_data` re-reads and re-hashes every pack — the
/// off-site bit-rot scrub — which is slow but the only way to catch silent rot.
pub fn check(repo: &Path, password: &str, read_data: bool) -> Result<(), StoreError> {
    let mut c = restic(repo, password);
    c.arg("check");
    if read_data {
        c.arg("--read-data");
    }
    let out = c.output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("restic check", &out));
    }
    Ok(())
}

fn spawn(e: std::io::Error) -> StoreError {
    StoreError::Io(format!("could not run restic (is it installed?): {e}"))
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

fn failed(what: &str, out: &Output) -> StoreError {
    let stderr = String::from_utf8_lossy(&out.stderr);
    StoreError::Io(format!("{what} failed: {}", stderr.trim()))
}

/// The same, for a command run with `--json`.
///
/// `--json` puts restic's errors on stderr as objects rather than sentences —
/// `{"message_type":"exit_error","code":12,"message":"Fatal: wrong password or no key found"}` —
/// so [`failed`] would hand the user a serialized struct. Pulls out every `message` there is,
/// and falls back to the raw text so a restic that wrote plain words still reaches them.
fn failed_json(what: &str, out: &Output) -> StoreError {
    let stderr = String::from_utf8_lossy(&out.stderr);
    let said: Vec<String> = stderr
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            v.get("message")
                .or_else(|| v.get("error").and_then(|e| e.get("message")))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .collect();
    if said.is_empty() {
        return failed(what, out);
    }
    StoreError::Io(format!("{what} failed: {}", said.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordinary snapshot: notes and blobs side by side under the vault root.
    #[test]
    fn common_parent_of_notes_and_blobs_is_the_vault_root() {
        let paths =
            [PathBuf::from("/home/ada/vault/notes"), PathBuf::from("/home/ada/vault/blobs")];
        assert_eq!(common_parent(&paths), Some(PathBuf::from("/home/ada/vault")));
    }

    /// A vault that has never held an attachment snapshots one path, and the root is still
    /// recoverable — this is the common case on a fresh vault and must not fail.
    #[test]
    fn common_parent_of_one_path_is_its_parent() {
        let paths = [PathBuf::from("/home/ada/vault/notes")];
        assert_eq!(common_parent(&paths), Some(PathBuf::from("/home/ada/vault")));
    }

    /// A `vault.json` may put the notes in `docs/`, and the root is unchanged by that.
    #[test]
    fn common_parent_survives_a_custom_notes_dir() {
        let paths = [PathBuf::from("/srv/team/vault/docs"), PathBuf::from("/srv/team/vault/blobs")];
        assert_eq!(common_parent(&paths), Some(PathBuf::from("/srv/team/vault")));
    }

    /// Two unrelated trees share only `/`, whose parent-of-parents is empty. Returning `None`
    /// is what makes `restore_vault` refuse rather than lift the whole filesystem.
    #[test]
    fn common_parent_refuses_when_there_is_no_shared_folder() {
        let paths = [PathBuf::from("/notes"), PathBuf::from("/blobs")];
        assert_eq!(common_parent(&paths), None);
    }

    /// The mapping from a source path to where restic puts it under `--target`.
    #[test]
    fn strip_prefix_drops_the_root_so_the_path_nests() {
        assert_eq!(strip_prefix(Path::new("/home/ada/vault")), PathBuf::from("home/ada/vault"));
    }
}
