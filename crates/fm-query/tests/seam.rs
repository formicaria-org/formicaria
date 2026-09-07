//! The seam suite: the query engine exercised with **zero filesystem access**.
//!
//! This test does not check correctness so much as defend the architecture: it
//! builds fixtures in RAM (no tempdir, no file, no DB) and asserts the engine's
//! behaviour. If it ever needs a filesystem to pass, the seam is already gone —
//! fix the code, not the test.

use fm_model::{Id, Kind, Object, PropertyValue, Stamp};
use fm_query::{run, Dir, Filter, Op, Predicate, Query, SortKey};
use std::collections::BTreeMap;
use time::macros::{date, datetime};
use time::Date;

fn obj(
    id: &str,
    kind: Kind,
    status: Option<&str>,
    due: Option<Date>,
    tags: &[&str],
    body: &str,
) -> Object {
    Object {
        id: id.parse::<Id>().expect("valid ULID"),
        kind,
        title: None,
        status: status.map(str::to_string),
        due: due.map(Stamp::day),
        start: None,
        hard: false,
        created: datetime!(2026-07-14 09:00 UTC),
        updated: datetime!(2026-07-14 09:00 UTC),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        assets: vec![],
        code: vec![],
        body: body.to_string(),
        extra: BTreeMap::new(),
        // Not from any store: no audience stated.
        vault: String::new(),
    }
}

fn fixtures() -> Vec<Object> {
    vec![
        obj(
            "01ARZ3NDEKTSV4RRFFQ69G5FA0",
            Kind::Asset,
            Some("doing"),
            Some(date!(2026 - 07 - 20)),
            &["meta-rl"],
            "trust region clipping",
        ),
        obj(
            "01ARZ3NDEKTSV4RRFFQ69G5FA1",
            Kind::Asset,
            Some("todo"),
            Some(date!(2026 - 07 - 25)),
            &["meta-rl", "exploration"],
            "advantage estimator",
        ),
        obj(
            "01ARZ3NDEKTSV4RRFFQ69G5FA2",
            Kind::Asset,
            Some("done"),
            None,
            &["writing"],
            "draft intro section",
        ),
        obj(
            "01ARZ3NDEKTSV4RRFFQ69G5FA3",
            Kind::Note,
            None,
            None,
            &["meta-rl"],
            "idea about GAE lambda",
        ),
    ]
}

fn prop_eq(key: &str, value: &str) -> Predicate {
    Predicate::Prop { key: key.into(), op: Op::Eq, value: PropertyValue::Text(value.into()) }
}

#[test]
fn filter_by_status() {
    let objs = fixtures();
    let q = Query { filter: Filter::new().and(prop_eq("status", "doing")), ..Default::default() };
    let r = run(&q, &objs);
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].body, "trust region clipping");
}

#[test]
fn text_is_just_a_predicate_and_case_insensitive() {
    let objs = fixtures();
    let q = Query {
        filter: Filter::new().and(Predicate::Text("TRUST region".into())),
        ..Default::default()
    };
    let r = run(&q, &objs);
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].body, "trust region clipping");
}

#[test]
fn tags_all_semantics() {
    let objs = fixtures();
    let q = Query {
        filter: Filter::new().and(Predicate::TagsAll(vec!["meta-rl".into(), "exploration".into()])),
        ..Default::default()
    };
    let r = run(&q, &objs);
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].body, "advantage estimator");
}

/// The falsifiability test: the board renderer must not know what `status` is.
/// Grouping by `status` and by `type` go through the *same* generic code path.
#[test]
fn group_by_is_generic() {
    let objs = fixtures();

    let by_status = run(&Query { group_by: Some("status".into()), ..Default::default() }, &objs);
    let groups = by_status.groups.expect("grouped");
    // doing, todo, done, and "(none)" for the Note with no status.
    assert_eq!(groups.len(), 4);
    assert!(groups.iter().any(|g| g.label == "doing" && g.rows.len() == 1));
    assert!(groups.iter().any(|g| g.label == "(none)" && g.rows.len() == 1));

    // Point the SAME engine at `type` and get a board of note/asset — no special
    // casing, no `todo`/`doing`/`done` anywhere.
    let by_type = run(&Query { group_by: Some("type".into()), ..Default::default() }, &objs);
    let groups = by_type.groups.expect("grouped");
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().any(|g| g.label == "asset" && g.rows.len() == 3));
    assert!(groups.iter().any(|g| g.label == "note" && g.rows.len() == 1));
}

/// The agenda view is a query, not a feature: `status != done AND due exists`,
/// sorted by due ascending — the "closest deadline first" list.
#[test]
fn agenda_is_a_query() {
    let objs = fixtures();
    let q = Query {
        filter: Filter::new()
            .and(Predicate::Not(Box::new(prop_eq("status", "done"))))
            .and(Predicate::Prop { key: "due".into(), op: Op::Exists, value: PropertyValue::Null }),
        sort: vec![SortKey::asc("due")],
        ..Default::default()
    };
    let r = run(&q, &objs);
    assert_eq!(r.total, 2);
    // Closest deadline first.
    assert_eq!(r.rows[0].body, "trust region clipping"); // due 07-20
    assert_eq!(r.rows[1].body, "advantage estimator"); // due 07-25
}

#[test]
fn sort_desc_and_paginate() {
    let objs = fixtures();
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![Kind::Asset])),
        sort: vec![SortKey { key: "status".into(), dir: Dir::Desc }],
        limit: Some(2),
        offset: 1,
        ..Default::default()
    };
    let r = run(&q, &objs);
    assert_eq!(r.total, 3); // three assets match
    assert_eq!(r.rows.len(), 2); // page of 2 after skipping 1
}

#[test]
fn date_range_filters_on_any_date_property() {
    let objs = fixtures();
    let q = Query {
        filter: Filter::new().and(Predicate::DateRange {
            key: "due".into(),
            from: Some(date!(2026 - 07 - 21)),
            to: Some(date!(2026 - 07 - 31)),
        }),
        ..Default::default()
    };
    let r = run(&q, &objs);
    assert_eq!(r.total, 1);
    assert_eq!(r.rows[0].body, "advantage estimator"); // due 07-25 is in range
}
