//! S1 perf budget: full-text search stays well under 100 ms at 10k notes,
//! because it goes through SQLite FTS5 (`MATCH`) and deserializes only the hits
//! — not all 10k objects. Notes are seeded as files directly (no per-note
//! fsync) so setup is cheap; only the search itself is timed.

use fm_core::{frontmatter, FileStore, Store};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use std::time::Instant;
use tempfile::tempdir;

const N: usize = 10_000;
const BUDGET_MS: u128 = 100;

#[test]
fn search_10k_under_budget() {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();

    // 10k notes; exactly one carries the rare needle.
    for i in 0..N {
        let body = if i == 7_777 {
            "the quokka theorem resolves the marsupial conjecture".to_string()
        } else {
            format!("routine note number {i} about gradients and estimators")
        };
        let o = Object::new(Kind::Note, body);
        let content = frontmatter::to_file(&o).unwrap();
        std::fs::write(notes.join(format!("{}.md", o.id)), content).unwrap();
    }

    // One reindex on open (untimed setup).
    let store = FileStore::open(dir.path()).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Text("quokka".into())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };

    let t = Instant::now();
    let r = store.query(&q).unwrap();
    let elapsed = t.elapsed();

    assert_eq!(r.total, 1, "the rare needle matches exactly one note");
    assert!(
        elapsed.as_millis() < BUDGET_MS,
        "search over {N} notes took {elapsed:?}, budget is {BUDGET_MS} ms"
    );
    println!("fts search over {N} notes: {elapsed:?}");
}
