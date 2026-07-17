//! Perf budget: the query engine must scan 10k objects well under the
//! "search < 100 ms @ 10k" target from the spec. This is the engine-level proxy;
//! the SQLite FTS5 path gets its own budget when FileStore lands. The margin is
//! deliberately huge so this catches gross regressions without flaking in CI.
//! See it printed with:  pixi run perf

use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{run, Filter, Op, Predicate, Query};
use std::collections::BTreeMap;
use std::time::Instant;
use time::macros::datetime;

fn synth(n: usize) -> Vec<Object> {
    (0..n)
        .map(|i| Object {
            id: Id::from_parts((i as u64) + 1, i as u128),
            kind: if i % 3 == 0 { Kind::Asset } else { Kind::Note },
            title: None,
            status: Some(if i % 2 == 0 { "doing" } else { "done" }.to_string()),
            due: None,
            start: None,
            hard: false,
            created: datetime!(2026-07-14 09:00 UTC),
            updated: datetime!(2026-07-14 09:00 UTC),
            tags: vec![format!("t{}", i % 20)],
            assets: vec![],
            code: vec![],
            body: format!("note {i} about trust region clipping and advantage estimation"),
            extra: BTreeMap::new(),
            // Not from any store: no audience stated.
            vault: String::new(),
        })
        .collect()
}

#[test]
fn query_10k_under_budget() {
    let objs = synth(10_000);
    let q = Query {
        filter: Filter::new()
            .and(Predicate::Prop {
                key: "status".into(),
                op: Op::Eq,
                value: PropertyValue::Text("doing".into()),
            })
            .and(Predicate::Text("trust region".into())),
        ..Default::default()
    };

    let _ = run(&q, &objs); // warm caches
    let start = Instant::now();
    let r = run(&q, &objs);
    let elapsed = start.elapsed();

    assert!(r.total > 0, "expected matches");
    assert!(
        elapsed.as_millis() < 100,
        "query over 10k took {elapsed:?}, budget is 100 ms"
    );
    eprintln!("query over 10k objects: {elapsed:?} ({} matches)", r.total);
}
