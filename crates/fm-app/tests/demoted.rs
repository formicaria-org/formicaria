//! **The surface the demotion ruling is conditional on.**
//!
//! `decisions.md` (2026-09-07, *a divergent field keeps both, by demoting the loser into a field
//! beside it*) argues that keeping the loser in a `conflict-<field>` key is not last-write-wins,
//! because the loser is visible and one tap from winning — and then makes that a condition:
//! *"if that surface is not built, this ruling should be revisited rather than left standing."*
//!
//! So these are not tests of a list. They are the tests that say the ruling still holds.

use fm_app::commands;
use fm_core::{MemoryStore, Store};
use fm_model::{Kind, Object, PropertyValue};

fn note(id: &str, title: &str) -> Object {
    let mut o = Object::new(Kind::Note, "body");
    o.id = id.parse().unwrap();
    o.title = Some(title.to_string());
    o
}

#[test]
fn a_demoted_field_is_listed_with_both_answers() {
    let mut store = MemoryStore::new();
    let mut o = note("01JQ0000000000000000000000", "Ship the thing");
    o.status = Some("done".into());
    o.extra.insert(
        "conflict-status".into(),
        PropertyValue::List(vec![PropertyValue::Text("doing".into())]),
    );
    store.put(&o).unwrap();

    let rows = commands::demoted(&store).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].field, "status");
    assert_eq!(rows[0].kept, "done", "what the note shows");
    assert_eq!(rows[0].other, vec!["doing".to_string()], "what the other device said");
    assert_eq!(rows[0].title, "Ship the thing", "named as a note, not as a ULID");
}

/// A note with nothing demoted must not appear. Obvious, and the reason the chip can be trusted to
/// mean something: a list that includes everything is a list nobody reads.
#[test]
fn an_ordinary_note_is_not_listed() {
    let mut store = MemoryStore::new();
    let mut o = note("01JQ0000000000000000000001", "Fine");
    o.status = Some("todo".into());
    store.put(&o).unwrap();
    assert!(commands::demoted(&store).unwrap().is_empty());
}

/// **A custom property counts too**, which is the half a hardcoded list of well-known fields would
/// have missed. `merge_objects` demotes any diverged key in `extra`, so anything less here would be
/// a surface that quietly under-reports — and under-reporting is the exact failure the ruling is
/// guarding against.
#[test]
fn a_custom_property_is_listed_like_any_other_field() {
    let mut store = MemoryStore::new();
    let mut o = note("01JQ0000000000000000000002", "Reading");
    o.extra.insert("shelf".into(), PropertyValue::Text("to-read".into()));
    o.extra.insert(
        "conflict-shelf".into(),
        PropertyValue::List(vec![PropertyValue::Text("reading".into())]),
    );
    store.put(&o).unwrap();

    let rows = commands::demoted(&store).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].field, "shelf");
    assert_eq!(rows[0].kept, "to-read");
    assert_eq!(rows[0].other, vec!["reading".to_string()]);
}

/// Two disagreements on one field, and a hand-typed scalar instead of a list. Both are shapes the
/// merge or a person can leave behind, and neither may be dropped on the way to the screen.
#[test]
fn a_second_loser_and_a_hand_typed_one_both_survive_the_trip() {
    let mut store = MemoryStore::new();
    let mut two = note("01JQ0000000000000000000003", "Twice");
    two.status = Some("blocked".into());
    two.extra.insert(
        "conflict-status".into(),
        PropertyValue::List(vec![
            PropertyValue::Text("doing".into()),
            PropertyValue::Text("review".into()),
        ]),
    );
    store.put(&two).unwrap();

    let mut typed = note("01JQ0000000000000000000004", "By hand");
    typed.status = Some("done".into());
    typed.extra.insert("conflict-status".into(), PropertyValue::Text("doing".into()));
    store.put(&typed).unwrap();

    let rows = commands::demoted(&store).unwrap();
    let twice = rows.iter().find(|r| r.title == "Twice").expect("{rows:?}");
    assert_eq!(twice.other, vec!["doing".to_string(), "review".to_string()]);
    let by_hand = rows.iter().find(|r| r.title == "By hand").expect("a scalar is a set of one");
    assert_eq!(by_hand.other, vec!["doing".to_string()]);
}

/// **Promoting the loser clears the record, through the ordinary property path.** This is the "one
/// tap from winning" the ruling rests on, and it must work with no machinery of its own: the panel
/// calls `set_property` twice, which is what a person typing the value would do.
#[test]
fn promoting_the_other_answer_leaves_no_disagreement_behind() {
    let mut store = MemoryStore::new();
    let mut o = note("01JQ0000000000000000000005", "Ship the thing");
    o.status = Some("done".into());
    o.extra.insert(
        "conflict-status".into(),
        PropertyValue::List(vec![PropertyValue::Text("doing".into())]),
    );
    store.put(&o).unwrap();
    let id = "01JQ0000000000000000000005";

    commands::set_property(&mut store, id, "status", "doing").unwrap();
    commands::set_property(&mut store, id, "conflict-status", "").unwrap();

    let after = store.get(id.parse().unwrap()).unwrap().expect("still there");
    assert_eq!(after.status.as_deref(), Some("doing"), "the other answer won");
    assert_eq!(after.extra.get("conflict-status"), None, "and there is nothing left to show");
    assert!(commands::demoted(&store).unwrap().is_empty(), "so the chip clears");
}
