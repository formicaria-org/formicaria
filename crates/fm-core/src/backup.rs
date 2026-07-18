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
use std::path::Path;
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
        Command::new("restic")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
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
pub fn backup(vault: &Path, repo: &Path, password: &str) -> Result<(), StoreError> {
    ensure_repo(repo, password)?;
    // Asked here rather than taken as an argument, so the three callers cannot drift on
    // *which* directories are ours — `vault.json` may put the notes in `docs/`.
    let notes = crate::descriptor::Descriptor::read(vault)?.notes_dir(vault);
    // A vault with no blobs yet is normal; restic errors on a path that does not exist.
    let blobs = vault.join("blobs");
    let mut paths: Vec<&Path> = Vec::new();
    if notes.exists() {
        paths.push(&notes);
    }
    if blobs.exists() {
        paths.push(&blobs);
    }
    if paths.is_empty() {
        return Err(StoreError::Io(
            "nothing to back up: this vault has no notes or blobs directory yet".into(),
        ));
    }
    let out = restic(repo, password)
        .arg("backup")
        .args(&paths)
        .arg("--tag")
        .arg("fm")
        .output()
        .map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("restic backup", &out));
    }
    Ok(())
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

fn failed(what: &str, out: &Output) -> StoreError {
    let stderr = String::from_utf8_lossy(&out.stderr);
    StoreError::Io(format!("{what} failed: {}", stderr.trim()))
}
