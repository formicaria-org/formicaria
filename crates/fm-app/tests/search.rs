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

/// `decisions.md#seams`, 2026-10-15: a note's tags and the values of its properties are found by
/// search, over the FTS5 index — and the bookkeeping an importer writes is not.
#[test]
fn search_reads_tags_and_property_values_but_not_bookkeeping() {
    use fm_model::PropertyValue;
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let mut paper = Object::new(Kind::Note, "a body that names nobody");
    paper.title = Some("A paper".into());
    paper.tags = vec!["Ada Lovelace".into()];
    paper.extra.insert("authors".into(), PropertyValue::Text("Grace Hopper".into()));
    paper.extra.insert(
        "venue".into(),
        PropertyValue::List(vec![PropertyValue::Text("Cognitive Neuroscience".into())]),
    );
    paper.extra.insert("source_key".into(), PropertyValue::Text("pages/zzsourcepath.md".into()));
    store.put(&paper).unwrap();

    for q in ["hopper", "lovelace", "neuroscience"] {
        let hits = commands::search(&store, q).unwrap();
        assert_eq!(hits.len(), 1, "{q} should find the paper by its tag or property");
    }
    assert!(commands::search(&store, "authors").unwrap().is_empty(), "a key is not a value");
    assert!(
        commands::search(&store, "zzsourcepath").unwrap().is_empty(),
        "an importer's source path is bookkeeping, not text"
    );
}
