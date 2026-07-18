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
