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

/// Commit a note, so each call adds exactly one commit to the vault's history.
fn write_and_commit(vault: &std::path::Path, name: &str, body: &str) {
    let notes = vault.join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join(name), body).unwrap();
    assert!(git::commit_all(vault, &format!("auto: {name}")).unwrap());
}

fn log_count(repo: &std::path::Path) -> usize {
    let out =
        Command::new("git").arg("-C").arg(repo).args(["log", "--oneline"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().count()
}

#[test]
fn set_remote_is_idempotent_and_the_last_url_wins() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    assert_eq!(git::remote(vault.path()).unwrap(), None, "a fresh vault has no remote");

    git::set_remote(vault.path(), "/tmp/first.git").unwrap();
    assert_eq!(git::remote(vault.path()).unwrap().as_deref(), Some("/tmp/first.git"));

    // Saving again must update, not fail on "remote already exists" — the panel
    // just saves whatever the user typed.
    git::set_remote(vault.path(), "/tmp/second.git").unwrap();
    assert_eq!(git::remote(vault.path()).unwrap().as_deref(), Some("/tmp/second.git"));
}

#[test]
fn an_empty_remote_url_is_refused_rather_than_silently_broken() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();

    // `git remote add origin ""` succeeds and leaves a remote that reports its
    // own *name* as its URL — the vault would then claim a destination it hasn't
    // got. Better to refuse than to lie about where a backup went.
    assert!(git::set_remote(vault.path(), "   ").is_err(), "blank URL refused");
    assert_eq!(git::remote(vault.path()).unwrap(), None, "and no remote was created");
}

#[test]
fn push_refuses_without_a_remote() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    write_and_commit(vault.path(), "01.md", "a note\n");

    let err = git::push_squashed(vault.path(), "backup").unwrap_err().to_string();
    assert!(err.contains("no remote configured"), "our own message, not git's stderr: {err}");
}

/// The whole push contract, against a local bare repo — real git, no network and
/// no credentials.
#[test]
fn push_sends_history_whole_the_first_time_then_squashes_each_later_push() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let bare = tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "--bare"])
        .arg(bare.path())
        .output()
        .unwrap()
        .status
        .success());

    write_and_commit(vault.path(), "01.md", "one\n");
    write_and_commit(vault.path(), "02.md", "two\n");
    write_and_commit(vault.path(), "03.md", "three\n");
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();

    // Nothing has ever been pushed, so there is no tracking ref to measure
    // against — and squashing here would destroy the only copy of that history.
    assert_eq!(git::unpushed(vault.path()).unwrap(), None, "no tracking ref before the first push");
    assert_eq!(git::push_squashed(vault.path(), "backup: first").unwrap(), 0, "first push: no squash");
    assert_eq!(log_count(bare.path()), 3, "the first push sends the history as it stands");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(0), "nothing left to push");

    // From here on, a window of auto-commits collapses into one remote commit.
    write_and_commit(vault.path(), "04.md", "four\n");
    write_and_commit(vault.path(), "05.md", "five\n");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(2));

    assert_eq!(git::push_squashed(vault.path(), "backup: second").unwrap(), 2, "squashed the pair");
    assert_eq!(log_count(bare.path()), 4, "3 + exactly one squashed commit");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(0));

    // The squash must not lose content: both notes are in the pushed tree.
    let files = Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&files.stdout);
    assert!(files.contains("notes/04.md") && files.contains("notes/05.md"), "squashed: {files}");
}

#[test]
fn a_single_unpushed_commit_is_pushed_as_is() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();

    write_and_commit(vault.path(), "01.md", "one\n");
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    // One commit is already one commit — squashing it would be pointless churn.
    write_and_commit(vault.path(), "02.md", "two\n");
    assert_eq!(git::push_squashed(vault.path(), "backup: second").unwrap(), 0, "nothing to squash");
    assert_eq!(log_count(bare.path()), 2);
}
