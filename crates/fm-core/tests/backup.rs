//! Backup via restic — safety-critical and, until now, untested (the module's own
//! comment: "an untested backup is not a backup"). These spin a throwaway restic
//! repo in a tempdir and prove the full round-trip: init is idempotent, a backup
//! then restores the exact note bytes, and integrity check passes. Skipped (not
//! failed) when restic is absent, so `cargo test` stays green outside the pixi
//! env while `pixi run test` — where restic is on PATH — runs them for real.

use fm_core::backup;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn have(bin: &str) -> bool {
    Command::new(bin).arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn backup_restore_round_trips_and_check_passes() {
    if !have("restic") {
        eprintln!("skipping backup test: restic not on PATH");
        return;
    }

    // Keep restic's cache out of the user's ~/.cache — the subprocess inherits
    // this process's environment, so setting it here scopes the whole test.
    let cache = tempdir().unwrap();
    std::env::set_var("RESTIC_CACHE_DIR", cache.path());

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "correct horse battery staple";

    // A minimal vault: one note file with recognizable bytes (the durable data).
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    let contents = "---\ntype: note\n---\nthe durable knowledge\n";
    fs::write(notes.join("01.md"), contents).unwrap();

    // init is idempotent: created the first time, already-present the second.
    assert!(backup::ensure_repo(repo.path(), password).unwrap(), "repo created on first run");
    assert!(!backup::ensure_repo(repo.path(), password).unwrap(), "repo already exists on second");

    backup::backup(vault.path(), repo.path(), password).unwrap();

    // Restore into a fresh dir; restic recreates the source tree there. The note
    // must come back byte-for-byte — the half people skip.
    let dest = tempdir().unwrap();
    backup::restore(repo.path(), password, dest.path()).unwrap();
    let restored = find_file(dest.path(), "01.md").expect("restored note is present");
    assert_eq!(
        fs::read_to_string(&restored).unwrap(),
        contents,
        "note bytes survive backup → restore"
    );

    // Integrity check, re-reading and re-hashing every pack (the off-site scrub).
    backup::check(repo.path(), password, true).unwrap();
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    for e in fs::read_dir(root).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(found) = find_file(&p, name) {
                return Some(found);
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(p);
        }
    }
    None
}
