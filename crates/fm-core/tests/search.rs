//! S1: full-text search is the `Text` predicate, backed by SQLite FTS5 in
//! FileStore. These assert the FTS behaviour — prefix matching, title *and*
//! body, recency ordering, composition with structured predicates, and a safe
//! fallback for inexpressible needles — that MemoryStore's substring scan
//! approximates for the equivalence contract in `file.rs`.

use fm_core::{FileStore, Store};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use tempfile::tempdir;
use time::macros::datetime;

fn text_query(needle: &str) -> Query {
    Query { filter: Filter::new().and(Predicate::Text(needle.into())), ..Default::default() }
}

#[test]
fn search_is_prefix_over_title_and_body_ranked_by_recency() {
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();

    let mut older = Object::new(Kind::Note, "gradient estimator variance");
    older.title = Some("Policy gradients".into());
    older.updated = datetime!(2020-01-01 0:00 UTC);

    let mut newer = Object::new(Kind::Note, "gradient clipping trick");
    newer.updated = datetime!(2026-01-01 0:00 UTC);

    let coffee = Object::new(Kind::Note, "unrelated note about coffee");
    s.put(&older).unwrap();
    s.put(&newer).unwrap();
    s.put(&coffee).unwrap();

    // Prefix "grad" matches both gradient notes (not coffee), newest first.
    let q = Query { sort: vec![SortKey::desc("updated")], ..text_query("grad") };
    let r = s.query(&q).unwrap();
    assert_eq!(r.total, 2, "prefix 'grad' matches the two gradient notes");
    assert_eq!(r.rows[0].id, newer.id, "recency order: newest first");
    assert_eq!(r.rows[1].id, older.id);

    // Title is searchable, not only the body.
    let by_title = s.query(&text_query("policy")).unwrap();
    assert_eq!(by_title.total, 1);
    assert_eq!(by_title.rows[0].id, older.id);
}

#[test]
fn search_composes_with_structured_predicates() {
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();

    let mut asset = Object::new(Kind::Asset, "review the gradient paper");
    asset.status = Some("doing".into());
    let note = Object::new(Kind::Note, "gradient descent notes");
    s.put(&asset).unwrap();
    s.put(&note).unwrap();

    // FTS narrows to both 'gradient' hits; the pure engine then applies Kind —
    // proving Text and structured predicates cooperate across the seam.
    let q = Query {
        filter: Filter::new()
            .and(Predicate::Text("gradient".into()))
            .and(Predicate::Kind(vec![Kind::Asset])),
        ..Default::default()
    };
    let r = s.query(&q).unwrap();
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].id, asset.id);
}

#[test]
fn inexpressible_needle_falls_back_instead_of_erroring() {
    // A needle that yields no FTS tokens must not error or silently match all;
    // it falls back to the pure substring engine. '%%%' matches nothing here.
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();
    s.put(&Object::new(Kind::Note, "plain content")).unwrap();
    let r = s.query(&text_query("%%%")).unwrap();
    assert_eq!(r.total, 0);
}
