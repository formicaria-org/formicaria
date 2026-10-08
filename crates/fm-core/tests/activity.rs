//! `git::activity` — the collaboration read-model: who last edited each note, from git alone.
//! Hermetic: a tempdir vault with real commits by two identities. Skipped (not failed) if git
//! is absent, like the other git tests.

use fm_core::git;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// The note files in a vault — what `FileStore::put` would have recorded had these tests
/// gone through the store rather than writing files directly. `commit_all` now stages an
/// explicit list, so a test has to say what it wrote, the same as the app does.
fn notes_of(vault: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(vault.join("notes"))
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "md"))
                .collect()
        })
        .unwrap_or_default()
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Commit-as: set the repo-local identity so the next `commit_all` is attributed to this person.
fn as_person(vault: &std::path::Path, name: &str, email: &str) {
    for (k, v) in [("user.name", name), ("user.email", email)] {
        Command::new("git").arg("-C").arg(vault).args(["config", k, v]).output().unwrap();
    }
}

fn write_note(vault: &std::path::Path, id: &str, body: &str) {
    fs::write(vault.join("notes").join(format!("{id}.md")), body).unwrap();
}

#[test]
fn activity_reports_each_notes_last_editor_newest_first() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    fs::create_dir_all(vault.path().join("notes")).unwrap();

    // Alice writes note A; Bob writes note B — two authors, two commits.
    as_person(vault.path(), "Alice", "alice@example.com");
    write_note(vault.path(), "note-a", "alice's note\n");
    git::commit_all(vault.path(), "a", &notes_of(vault.path())).unwrap();

    as_person(vault.path(), "Bob", "bob@example.com");
    write_note(vault.path(), "note-b", "bob's note\n");
    git::commit_all(vault.path(), "b", &notes_of(vault.path())).unwrap();

    let acts = git::activity(vault.path(), "1 year ago").unwrap();
    assert_eq!(acts.len(), 2, "one touch per note");
    assert_eq!(acts[0].id, "note-b", "newest edit is first");
    assert_eq!(acts[0].author, "Bob");
    assert_eq!(acts[0].email, "bob@example.com");
    assert_eq!(acts[1].id, "note-a");
    assert_eq!(acts[1].author, "Alice");

    // Bob edits Alice's note → its LAST editor becomes Bob, and it moves to the front.
    write_note(vault.path(), "note-a", "alice's note, edited by bob\n");
    git::commit_all(vault.path(), "c", &notes_of(vault.path())).unwrap();

    let acts = git::activity(vault.path(), "1 year ago").unwrap();
    assert_eq!(acts.len(), 2, "still one touch per note (last edit only)");
    assert_eq!(acts[0].id, "note-a", "re-edited note is now most recent");
    assert_eq!(acts[0].author, "Bob", "last editor, not the creator");
}

#[test]
fn activity_is_empty_without_a_repo_or_history() {
    if !have_git() {
        return;
    }
    // Not a repo at all.
    let plain = tempdir().unwrap();
    assert!(git::activity(plain.path(), "1 year ago").unwrap().is_empty());

    // A repo with no commits yet.
    let empty = tempdir().unwrap();
    git::ensure_repo(empty.path()).unwrap();
    assert!(git::activity(empty.path(), "1 year ago").unwrap().is_empty());
}

/// **All of history means all of it, however old the commit.** `"@0"` was passed for this, as
/// "git's epoch syntax", and git 2.53 reads a zero timestamp as *now*: the log kept only commits from
/// the current second. So `who_left_a_message_is_read_from_git` went red whenever a slow runner
/// crossed a second boundary between commit and log (2026-08 to 2026-10-08, cause unknown until the
/// diagnostics it had been given printed an empty `git log`), and a real discussion lost its
/// participants. A commit dated 2020 makes this deterministic rather than a race.
#[test]
fn all_history_reaches_a_commit_from_long_before_this_second() {
    if !have_git() {
        return;
    }
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    fs::create_dir_all(vault.path().join("notes")).unwrap();
    as_person(vault.path(), "Ada", "ada@example.org");
    write_note(vault.path(), "note-old", "written long ago\n");
    let git_old = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(vault.path())
            .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00Z")
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    };
    git_old(&["add", "notes/note-old.md"]);
    git_old(&["commit", "-q", "-m", "old"]);

    let acts = git::activity(vault.path(), git::ALL_HISTORY).unwrap();
    assert_eq!(acts.len(), 1, "a 2020 commit is part of all of history");
    assert_eq!(acts[0].id, "note-old");
    assert_eq!(acts[0].author, "Ada");
}
