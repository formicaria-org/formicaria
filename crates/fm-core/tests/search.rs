//! S1: full-text search is the `Text` predicate, backed by SQLite FTS5 in
//! FileStore. These assert the FTS behaviour — prefix matching, title *and*
//! body, recency ordering, composition with structured predicates, and a safe
//! fallback for inexpressible needles — that MemoryStore's substring scan
//! approximates for the equivalence contract in `file.rs`.

use fm_core::{FileStore, MemoryStore, Store};
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

/// The same notes in both stores — the seam contract, now for the searches FileStore used to miss.
fn both(notes: &[Object]) -> (FileStore, MemoryStore, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let mut f = FileStore::open(dir.path()).unwrap();
    let mut m = MemoryStore::new();
    for n in notes {
        f.put(n).unwrap();
        m.put(n).unwrap();
    }
    (f, m, dir)
}

fn ids(s: &dyn Store, q: &Query) -> Vec<String> {
    s.query(q).unwrap().rows.iter().map(|o| o.id.to_string()).collect()
}

/// **The inside of a compound word is found** (2026-10-29). The index kept `Ernährungsberatung` as
/// one word and matched only its start, so `beratung` found nothing in FileStore while MemoryStore's
/// substring scan found it.
#[test]
fn the_inside_of_a_compound_word_is_found_in_both_stores() {
    let compound = Object::new(Kind::Note, "Termin zur Ernährungsberatung am Montag");
    let other = Object::new(Kind::Note, "Einkaufsliste");
    let (f, m, _dir) = both(&[compound.clone(), other]);
    let q = text_query("beratung");
    assert_eq!(ids(&f, &q), vec![compound.id.to_string()]);
    assert_eq!(ids(&f, &q), ids(&m, &q));
}

/// **A Chinese word is found inside a run without spaces** (2026-10-29): `世界` was never the start
/// of the index's one-word `你好世界`.
#[test]
fn a_chinese_word_is_found_inside_a_run_in_both_stores() {
    let zh = Object::new(Kind::Note, "你好世界，我喜欢学习");
    let other = Object::new(Kind::Note, "维生素和铁");
    let (f, m, _dir) = both(&[zh.clone(), other]);
    for word in ["世界", "学习", "好"] {
        let q = text_query(word);
        assert_eq!(ids(&f, &q), vec![zh.id.to_string()], "{word}");
        assert_eq!(ids(&f, &q), ids(&m, &q), "{word}");
    }
}

/// A short term stays a word start: `ab` inside every word that contains it would bury the notes
/// that begin with it.
#[test]
fn a_short_term_is_found_only_at_the_start_of_a_word() {
    let starts = Object::new(Kind::Note, "abacus lesson");
    let inside = Object::new(Kind::Note, "a crab on the beach");
    let (f, _m, _dir) = both(&[starts.clone(), inside]);
    assert_eq!(ids(&f, &text_query("ab")), vec![starts.id.to_string()]);
}

/// **The best answer first, in both stores** (2026-10-29): a note titled with the words outranks a
/// newer note that only mentions them, and recency still orders notes the search scores alike.
#[test]
fn search_ranks_by_relevance_then_recency_alike_in_both_stores() {
    let mut about = Object::new(Kind::Note, "Leafy greens and natto.");
    about.title = Some("Vitamin K".into());
    about.updated = datetime!(2020-01-01 0:00 UTC);
    let mut mention = Object::new(Kind::Note, "Shopping: greens, they have vitamin K.");
    mention.updated = datetime!(2026-01-01 0:00 UTC);
    let mut mention_older = Object::new(Kind::Note, "Shopping: greens, they have vitamin K.");
    mention_older.updated = datetime!(2025-01-01 0:00 UTC);
    let (f, m, _dir) = both(&[mention_older.clone(), about.clone(), mention.clone()]);
    let q = Query {
        sort: vec![SortKey::desc("updated")],
        rank_by: Some("vitamin k".into()),
        ..text_query("vitamin k")
    };
    let want = vec![about.id.to_string(), mention.id.to_string(), mention_older.id.to_string()];
    assert_eq!(ids(&f, &q), want);
    assert_eq!(ids(&m, &q), want);
}
