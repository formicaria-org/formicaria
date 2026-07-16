//! The `search` command function — the same engine and `ObjectMeta` the other
//! views use, driven by a `Text` predicate. Over `FileStore` that is the SQLite
//! FTS5 path, so this also exercises full-text search end-to-end.

use fm_app::commands;
use fm_core::{FileStore, Store};
use fm_model::{Kind, Object};
use tempfile::tempdir;
use time::OffsetDateTime;

#[test]
fn search_finds_by_text_and_ignores_empty_queries() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    store.put(&Object::new(Kind::Note, "GAE lambda and markerzzz")).unwrap();
    store.put(&Object::new(Kind::Note, "an unrelated note")).unwrap();

    let hits = commands::search(&store, "markerzzz").unwrap();
    assert_eq!(hits.len(), 1, "finds only the note containing the marker");
    assert!(hits[0].preview.contains("markerzzz"));

    // An empty or whitespace query returns nothing rather than the whole vault.
    assert!(commands::search(&store, "").unwrap().is_empty());
    assert!(commands::search(&store, "   ").unwrap().is_empty());
}

#[test]
fn recent_returns_every_note_newest_created_first() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    // Distinct created timestamps so the ordering is unambiguous.
    let mut older = Object::new(Kind::Note, "older note");
    older.created = OffsetDateTime::from_unix_timestamp(1_600_000_000).unwrap();
    let mut newer = Object::new(Kind::Note, "newer note");
    newer.created = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    // An asset is not part of the feed — it is a blob a note references, and the
    // timeline (and the editor's `/` suggestions) are about notes.
    let mut blob = Object::new(Kind::Asset, "an ingested file");
    blob.created = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    store.put(&older).unwrap();
    store.put(&newer).unwrap();
    store.put(&blob).unwrap();

    let feed = commands::recent(&store).unwrap();
    assert_eq!(feed.len(), 2, "recent returns every note and no asset");
    assert!(feed[0].preview.contains("newer"), "newest-created first");
    assert!(feed[1].preview.contains("older"));
    // Newest of all, so it would head the feed if it were included at all.
    assert!(feed.iter().all(|m| m.kind != "asset"), "the asset leaked into the feed");
}
