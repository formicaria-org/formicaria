//! Stale notes — **derived from git, stored nowhere.**
//!
//! Ruling 15 decided the mechanism before the feature existed: *"Presence/status → derived from
//! git log or pushed by the peer, stored nowhere. … Status is not knowledge."* So there is no
//! `stale:` property to keep true, nothing travels into a collaborator's clone as an opinion,
//! and the threshold belongs to whoever asks rather than to a config file.
//!
//! Real git, because the thing under test is a read of `git log`.

use fm_app::commands::{capture, reply, set_property, stale};
use fm_core::{git, FileStore};
use tempfile::tempdir;

fn have_git() -> bool {
    std::process::Command::new("git").arg("--version").output().is_ok()
}

/// Push HEAD's dates back to 2020, so a "90 days ago" window genuinely excludes it.
///
/// **A fixed date in the past, deliberately.** `--since=now` does not work: git still counts a
/// commit made in the same second, so the first version of these tests asserted an assumption
/// about git rather than a fact about the code. A fixed *future* date would work today and
/// become a time bomb — the exact shape that broke `edit.rs` this morning. A fixed past date is
/// permanently more than 90 days ago and can never expire.
fn backdate(vault: &std::path::Path) {
    let when = "2020-01-01T00:00:00+00:00";
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(vault)
        .args(["commit", "--amend", "--no-edit", "--date", when])
        .env("GIT_COMMITTER_DATE", when)
        .output()
        .unwrap();
    assert!(out.status.success(), "backdate failed: {}", String::from_utf8_lossy(&out.stderr));
}

/// A vault with git and an identity, ready to commit.
fn vault() -> (tempfile::TempDir, FileStore) {
    let dir = tempdir().unwrap();
    let store = FileStore::open(dir.path()).unwrap();
    git::ensure_repo(dir.path()).unwrap();
    git::set_identity(dir.path(), "Tester", "t@example.org").unwrap();
    (dir, store)
}

#[test]
fn a_note_committed_inside_the_window_is_not_stale() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (dir, mut store) = vault();
    capture(&mut store, "written just now", "").unwrap();
    git::commit_all(dir.path(), "auto: seed", &store.written()).unwrap();

    let old = stale(&store, dir.path(), "90 days ago").unwrap();

    assert!(old.is_empty(), "a note touched today is not stale: {old:?}");
}

/// The real shape of the answer: a note whose last commit predates the window.
#[test]
fn a_note_untouched_since_the_window_is_reported() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (dir, mut store) = vault();
    let id = capture(&mut store, "written long ago", "").unwrap().id;
    git::commit_all(dir.path(), "auto: seed", &store.written()).unwrap();

    backdate(dir.path());

    let old = stale(&store, dir.path(), "90 days ago").unwrap();

    assert_eq!(old.len(), 1, "the note falls outside the window");
    assert_eq!(old[0].id, id);
}

/// Messages are not stale notes. A short reply from March is a finished sentence, not
/// something you forgot to revisit — and it comes out via the shared notes-only base.
#[test]
fn discussion_messages_are_never_reported_as_stale_notes() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (dir, mut store) = vault();
    let note = capture(&mut store, "the note", "").unwrap().id;
    for i in 0..5 {
        reply(&mut store, &note, &format!("message {i}")).unwrap();
    }
    git::commit_all(dir.path(), "auto: seed", &store.written()).unwrap();
    backdate(dir.path());

    let old = stale(&store, dir.path(), "90 days ago").unwrap();

    assert_eq!(old.len(), 1, "six files, one note: {old:?}");
    assert_eq!(old[0].id, note);
}

/// **"No evidence" is not "old".** A vault with no history cannot answer the question, and both
/// silent answers are lies: an empty list claims nothing is stale, a full one claims everything
/// is. Git is a capability this product declares, not one it assumes — so it refuses out loud.
#[test]
fn a_vault_without_history_refuses_rather_than_guessing() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    capture(&mut store, "a note in a vault with no git", "").unwrap();

    let answer = stale(&store, dir.path(), "90 days ago");

    assert!(answer.is_err(), "an unanswerable question must not get a confident answer");
}

/// Oldest first — the point of the view is that the worst offender is at the top.
#[test]
fn the_oldest_note_comes_first() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (dir, mut store) = vault();
    let first = capture(&mut store, "older", "").unwrap().id;
    let second = capture(&mut store, "newer", "").unwrap().id;
    // Move the second note's `updated` forward, which is what an edit does.
    set_property(&mut store, &second, "status", "doing").unwrap();
    git::commit_all(dir.path(), "auto: seed", &store.written()).unwrap();
    backdate(dir.path());

    let old = stale(&store, dir.path(), "90 days ago").unwrap();

    assert_eq!(old.len(), 2);
    assert_eq!(old[0].id, first, "the least recently updated note leads");
}
