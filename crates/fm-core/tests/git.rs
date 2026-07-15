//! Vault versioning via git — the notes' durable, pushable history. Hermetic: a
//! tempdir vault, a repo-local identity written by `ensure_repo`, no network.
//! Skipped (not failed) when git is absent.

use fm_core::git;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn ensure_repo_initializes_once_and_writes_gitignore() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();

    assert!(git::ensure_repo(vault.path()).unwrap(), "repo created on first run");
    assert!(vault.path().join(".git").exists(), ".git directory present");

    let ignore = fs::read_to_string(vault.path().join(".gitignore")).unwrap();
    assert!(ignore.contains("index.sqlite"), "disposable index ignored");
    assert!(ignore.contains("derived/"), "regenerable thumbnails ignored");
    assert!(ignore.contains("blobs/"), "heavy blobs ignored (they sync out-of-band)");

    assert!(!git::ensure_repo(vault.path()).unwrap(), "already a repo on the second run");
}

#[test]
fn commit_all_commits_changes_then_reports_a_clean_tree() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("01.md"), "the durable knowledge\n").unwrap();

    assert!(git::commit_all(vault.path(), "first snapshot").unwrap(), "committed the new note");

    let log = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&log.stdout).lines().count(),
        1,
        "exactly one commit in the history"
    );

    // A clean tree is not an error — it is the common case for a debounced
    // auto-commit and must report "nothing to commit" as `false`.
    assert!(!git::commit_all(vault.path(), "no-op").unwrap(), "clean tree → nothing to commit");
}
