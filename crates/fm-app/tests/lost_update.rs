//! The lost-update hole a pull opens, and the guard that closes it.
//!
//! `FileStore::put` already refuses a stale write: it compares the `mtime_ns` it indexed
//! against the file's current mtime, so a note rewritten behind the app's back cannot be
//! overwritten from a stale copy. That guard is real and it works — **except in exactly the
//! case that matters most.**
//!
//! `pull` merges and then immediately `reindex`es, because the views are served from SQLite
//! and a merge is invisible until it does. That reindex writes the *post-merge* mtime into
//! the index. From `put`'s point of view the file is now perfectly in sync — so the guard
//! stands down, and a browser still holding the pre-merge body saves it straight over the
//! collaborator's text. Noticing the change is what disarms the protection against it.
//!
//! mtime cannot see this, because the staleness is in the *client*, not on disk. So the
//! client says what it edited: `update_body` takes the `updated` stamp the caller last saw,
//! and refuses when the note has moved on since.

use fm_app::commands;
use fm_core::{FileStore, Reindex, Store};
use tempfile::tempdir;

/// Set up a vault with one note and return `(store, dir, id, updated)`.
fn vault_with_a_note(body: &str) -> (FileStore, tempfile::TempDir, String, String) {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let meta = commands::capture(&mut store, body, "").unwrap();
    (store, dir, meta.id, meta.updated)
}

/// Rewrite the note's file behind the app's back — what a `git pull`'s merge does — then
/// reindex, which is what `pull` does next and what makes the merge visible.
fn a_collaborators_merge_lands(store: &mut FileStore, dir: &tempfile::TempDir, body: &str) {
    let path = std::fs::read_dir(dir.path().join("notes"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("the note file");
    let text = std::fs::read_to_string(&path).unwrap();
    let (front, _) = text.rsplit_once("---\n").expect("frontmatter fence");
    // The write itself moves mtime; sleep first so the new value is distinguishable from
    // the old one even on a filesystem with coarse granularity. Without a *different*
    // mtime the `put` guard's own precondition is unmet and this test would prove nothing.
    std::thread::sleep(std::time::Duration::from_millis(20));
    // A merge rewrites the body, leaves the frontmatter valid — the whole point of the
    // `.md` driver — and resolves `updated:` to **the later of the two readings**
    // (`merge::merge_objects`). Bumping it here is not test convenience: a helper that left
    // the old stamp would be modelling a merge that cannot happen, and would "prove" a
    // guard that never fires in the real case.
    let later = "2099-01-01T00:00:00Z";
    let front = front
        .lines()
        .map(|l| if l.starts_with("updated:") { format!("updated: {later}") } else { l.to_string() })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, format!("{front}\n---\n\n{body}\n")).unwrap();
    // …and this is what `pull` does next, which is what re-arms the index and disarms the
    // mtime guard. Removing this line makes the bug disappear — which is the finding.
    store.reindex(Reindex::Incremental).unwrap();
}

/// **The bug.** Without a base stamp, the pull's own reindex re-arms `mtime_ns`, the
/// `put` guard sees a file that matches the index, and the stale draft wins.
#[test]
fn a_stale_draft_cannot_overwrite_a_merge_that_landed_under_it() {
    let (mut store, dir, id, base) = vault_with_a_note("my paragraph\n");

    // The pane is open, holding `base`. Their work arrives and is indexed.
    a_collaborators_merge_lands(&mut store, &dir, "my paragraph\n\ntheir paragraph\n");

    // The debounced save fires with the body the pane loaded — pre-merge.
    let result = commands::update_body(&mut store, &id, "my paragraph, edited\n", &base);

    assert!(
        result.is_err(),
        "saving a draft based on a superseded version must be refused, not silently applied"
    );

    // And their paragraph is still there.
    let note = commands::get(&store, &id).unwrap().unwrap();
    assert!(
        note.body.contains("their paragraph"),
        "a collaborator's merged text was overwritten: {:?}",
        note.body
    );
}

/// The ordinary path must stay ordinary: an editor that keeps up saves without friction,
/// repeatedly, using the stamp each write hands back.
#[test]
fn consecutive_saves_from_an_up_to_date_editor_all_succeed() {
    let (mut store, _dir, id, mut base) = vault_with_a_note("draft\n");

    for n in 1..=3 {
        let body = format!("draft {n}\n");
        base = commands::update_body(&mut store, &id, &body, &base)
            .unwrap_or_else(|e| panic!("save {n} should succeed: {e}"));
        assert_eq!(commands::get(&store, &id).unwrap().unwrap().body, body);
    }
}

/// An empty base opts out — `fm-cli`, curl and any caller that never read the note keep
/// working exactly as before. The mtime guard in `put` still covers them.
#[test]
fn an_empty_base_skips_the_check() {
    let (mut store, dir, id, _) = vault_with_a_note("mine\n");
    a_collaborators_merge_lands(&mut store, &dir, "theirs\n");

    assert!(commands::update_body(&mut store, &id, "forced\n", "").is_ok());
    assert_eq!(commands::get(&store, &id).unwrap().unwrap().body, "forced\n");
}
