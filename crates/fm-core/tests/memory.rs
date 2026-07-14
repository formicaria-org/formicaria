//! The `Store` contract, verified against `MemoryStore` with zero filesystem
//! access. `FileStore` will later run this same battery to prove the two
//! backends honour one contract.

use fm_core::{MemoryStore, Reindex, Store};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query};
use std::collections::BTreeMap;
use time::macros::datetime;

fn note(id: &str, status: &str, body: &str) -> Object {
    Object {
        id: id.parse::<Id>().expect("valid ULID"),
        kind: Kind::Task,
        title: None,
        status: Some(status.to_string()),
        due: None,
        hard: false,
        created: datetime!(2026-07-14 09:00 UTC),
        updated: datetime!(2026-07-14 09:00 UTC),
        tags: vec![],
        assets: vec![],
        code: vec![],
        body: body.to_string(),
        extra: BTreeMap::new(),
    }
}

#[test]
fn put_get_delete_roundtrip() {
    let mut s = MemoryStore::new();
    let o = note("01ARZ3NDEKTSV4RRFFQ69G5FA0", "doing", "hello");
    let id = o.id;

    s.put(&o).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s.get(id).unwrap().unwrap().body, "hello");

    s.delete(id).unwrap();
    assert!(s.get(id).unwrap().is_none());
    assert!(s.delete(id).is_err()); // deleting a missing id is an error
}

#[test]
fn query_goes_through_the_seam() {
    let mut s = MemoryStore::new();
    s.put(&note("01ARZ3NDEKTSV4RRFFQ69G5FA0", "doing", "a")).unwrap();
    s.put(&note("01ARZ3NDEKTSV4RRFFQ69G5FA1", "done", "b")).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Prop {
            key: "status".into(),
            op: Op::Eq,
            value: PropertyValue::Text("doing".into()),
        }),
        ..Default::default()
    };
    let r = s.query(&q).unwrap();
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].body, "a");
}

#[test]
fn reindex_is_a_noop_for_memory() {
    let mut s = MemoryStore::new();
    s.put(&note("01ARZ3NDEKTSV4RRFFQ69G5FA0", "doing", "a")).unwrap();
    let stats = s.reindex(Reindex::Full).unwrap();
    assert_eq!(stats.scanned, 1);
}
