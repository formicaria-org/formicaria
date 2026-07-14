//! Backup via restic (subprocess) — dedup + encryption + integrity + remotes,
//! none of which we should reimplement. `restic check --read-data` is the
//! off-site bit-rot scrub that complements `fm verify --scrub`.
//!
//! The disposable, per-machine index (`index.sqlite`) and regenerable thumbnails
//! (`derived/`) are excluded: they rebuild from the notes and blobs, and the
//! index in particular must never travel between machines (the DB-corruption-by-
//! sync lesson). What's backed up is the durable knowledge: notes, blobs, views,
//! themes, scripts, and the manifest.

use crate::StoreError;
use std::path::Path;
use std::process::{Command, Output};

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

/// Snapshot the vault (initializing the repo on first run).
pub fn backup(vault: &Path, repo: &Path, password: &str) -> Result<(), StoreError> {
    ensure_repo(repo, password)?;
    let out = restic(repo, password)
        .arg("backup")
        .arg(vault)
        .arg("--exclude")
        .arg("index.sqlite")
        .arg("--exclude")
        .arg("derived")
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
