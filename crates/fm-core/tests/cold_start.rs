//! **Opening a vault must produce the same index however it gets there.**
//!
//! `FileStore::open` unconditionally ran `Reindex::Full`: drop both tables, re-read and re-parse
//! every `.md` file, rebuild the whole FTS index. On a desktop that is a startup cost you pay once
//! a day. On Android the OS kills backgrounded apps constantly, so it is what happens on *every*
//! relaunch — and `index.sqlite` persisting across those relaunches bought precisely nothing.
//!
//! `mobile-design.md` has called for opening `Incremental` since M0. What was missing was any way
//! to know it is safe, and `known-issues.md` says so: *"cold-start tests that do not exist."*
//! This file is those tests, and they are written in the shape this repo already uses for merge
//! (`merge_differential.rs`) — **differential**, not budgeted. The question is not "is it faster",
//! it is "does it produce the same answer as the rebuild it replaces", and the honest way to ask
//! is to build both and compare.
//!
//! The interesting case is the third one: a writer that changes a file **without moving its
//! mtime**. Incremental reconciles on mtime, so such a writer is invisible to it. That is not
//! hypothetical — `cp -p`, `rsync -a` and a restic restore all preserve mtimes — and it is the
//! reason the incremental open is gated rather than simply switched on.

use fm_core::{frontmatter, FileStore, Reindex, Store};
use fm_model::{Kind, Object};
use fm_query::{Query, SortKey};
use std::path::Path;
use tempfile::{tempdir, TempDir};

/// Everything the index claims to hold, in a comparable form: every note, id and body, ordered.
fn snapshot(store: &FileStore) -> Vec<(String, String)> {
    let q = Query { sort: vec![SortKey::asc("created")], limit: None, ..Default::default() };
    let mut rows: Vec<(String, String)> =
        store.query(&q).unwrap().rows.iter().map(|o| (o.id.to_string(), o.body.clone())).collect();
    rows.sort();
    rows
}

/// A vault of `n` notes on disk, plus the store over it.
fn seeded(n: usize) -> (TempDir, Vec<std::path::PathBuf>) {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let mut paths = Vec::new();
    for i in 0..n {
        let o = Object::new(Kind::Note, format!("note {i} about gradients"));
        let p = notes.join(format!("{}.md", o.id));
        std::fs::write(&p, frontmatter::to_file(&o).unwrap()).unwrap();
        paths.push(p);
    }
    (dir, paths)
}

/// What a *fresh* full rebuild of this directory says is in it — the oracle.
fn full_rebuild_snapshot(root: &Path) -> Vec<(String, String)> {
    let copy = tempdir().unwrap();
    let notes = copy.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    for entry in std::fs::read_dir(root.join("notes")).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) == Some("md") {
            std::fs::copy(&p, notes.join(p.file_name().unwrap())).unwrap();
        }
    }
    snapshot(&FileStore::open(copy.path()).unwrap())
}

/// Rewrite a note's body, leaving the file's mtime to move as it normally would.
fn rewrite(path: &Path, body: &str) {
    let content = std::fs::read_to_string(path).unwrap();
    let obj = frontmatter::from_file(&content).unwrap();
    let mut fresh = obj.clone();
    fresh.body = body.to_string();
    std::fs::write(path, frontmatter::to_file(&fresh).unwrap()).unwrap();
}

#[test]
fn an_incremental_open_agrees_with_a_full_rebuild_on_ordinary_changes() {
    let (dir, paths) = seeded(30);
    {
        let _warm = FileStore::open(dir.path()).unwrap(); // writes index.sqlite
    }

    // (a) an ordinary rewrite, (b) a deletion, (c) a brand-new file.
    rewrite(&paths[3], "rewritten on another device");
    std::fs::remove_file(&paths[7]).unwrap();
    let extra = Object::new(Kind::Note, "arrived in a pull");
    std::fs::write(
        dir.path().join("notes").join(format!("{}.md", extra.id)),
        frontmatter::to_file(&extra).unwrap(),
    )
    .unwrap();

    // Reopen and reconcile incrementally, exactly as a gated cold start would.
    let mut store = FileStore::open_incremental(dir.path()).unwrap();
    let stats = store.reindex(Reindex::Incremental).unwrap();
    assert_eq!(stats.updated, 0, "a second incremental pass should find nothing left to do");

    assert_eq!(
        snapshot(&store),
        full_rebuild_snapshot(dir.path()),
        "the incrementally-opened index disagrees with a fresh full rebuild of the same files",
    );
}

/// **The case that makes this need a gate rather than a switch.**
///
/// Incremental reconciles on mtime. A writer that restores a file's old mtime — `cp -p`,
/// `rsync -a`, `restic restore` — is therefore invisible to it: the file changed, the index does
/// not notice, and the note is served stale forever (or until something else touches it).
///
/// So the test asserts the *failure*, deliberately. `open_incremental` must not be reachable on a
/// platform where such a writer exists, and the constraint is written down here rather than
/// discovered later by someone whose notes silently stopped updating.
#[test]
fn an_mtime_preserving_writer_is_invisible_to_the_incremental_path() {
    let (dir, paths) = seeded(10);
    {
        let _warm = FileStore::open(dir.path()).unwrap();
    }

    // Change the content, then put the mtime back exactly as it was.
    let before = std::fs::metadata(&paths[2]).unwrap().modified().unwrap();
    rewrite(&paths[2], "restored from a backup, same mtime");
    let f = std::fs::File::options().write(true).open(&paths[2]).unwrap();
    f.set_modified(before).unwrap();
    drop(f);

    let store = FileStore::open_incremental(dir.path()).unwrap();
    let incremental = snapshot(&store);
    let full = full_rebuild_snapshot(dir.path());

    assert_ne!(
        incremental, full,
        "if this now agrees, the incremental path has learned to detect mtime-preserving writes \
         (a size or content check?) — which would be good news, but the gate on \
         `open_incremental` was justified by exactly this divergence and should be revisited \
         deliberately rather than left in place by accident",
    );
}

/// A cold start on an index written by an older schema must not trust it.
///
/// Android exits by `SIGKILL`, so there is no clean-shutdown hook that could mark an index
/// complete on the way out. The marker is therefore written *after* the reindex has committed —
/// so a marker implies the rows are durable, and an interrupted rebuild rolls its transaction back
/// without ever reaching it.
#[test]
fn an_index_with_no_completion_marker_is_rebuilt_from_scratch() {
    let (dir, paths) = seeded(12);
    {
        let _warm = FileStore::open(dir.path()).unwrap();
    }

    // Simulate a rebuild that never finished: clear the marker, and change a file behind the
    // index's back in a way it could not otherwise notice.
    let db = rusqlite::Connection::open(dir.path().join("index.sqlite")).unwrap();
    db.execute_batch("PRAGMA user_version = 0").unwrap();
    drop(db);
    let before = std::fs::metadata(&paths[1]).unwrap().modified().unwrap();
    rewrite(&paths[1], "changed while the index was mid-rebuild");
    let f = std::fs::File::options().write(true).open(&paths[1]).unwrap();
    f.set_modified(before).unwrap();
    drop(f);

    // With no marker the open must fall back to Full, which re-reads the file and sees the change
    // the mtime hid.
    let store = FileStore::open_incremental(dir.path()).unwrap();
    assert_eq!(
        snapshot(&store),
        full_rebuild_snapshot(dir.path()),
        "an index with no completion marker must be rebuilt, not trusted",
    );
}
