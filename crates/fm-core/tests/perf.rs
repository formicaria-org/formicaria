//! S1 perf budget: full-text search stays well under 100 ms at 10k notes,
//! because it goes through SQLite FTS5 (`MATCH`) and deserializes only the hits
//! — not all 10k objects. Notes are seeded as files directly (no per-note
//! fsync) so setup is cheap; only the search itself is timed.

use fm_core::{frontmatter, FileStore, Reindex, Store};
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

/// The local poll's budget. Every open tab beats `ping` every 3 seconds and each beat runs
/// an incremental reindex, so this is the one piece of work the app repeats forever — and
/// the quiet case (nothing changed) is the case that has to be nearly free.
///
/// It was not: the deletion sweep tested membership against a `Vec` of every path on disk,
/// making a quiet poll O(notes²). At 10k notes that is ~10⁸ string comparisons, three times
/// a minute, to conclude that nothing had been deleted. The budget below fails loudly if
/// that shape ever comes back.
#[test]
fn a_quiet_incremental_poll_stays_cheap_at_10k_notes() {
    const POLL_BUDGET_MS: u128 = 500;

    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    for i in 0..N {
        let o = Object::new(Kind::Note, format!("note {i}"));
        std::fs::write(notes.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
            .unwrap();
    }
    let mut store = FileStore::open(dir.path()).unwrap();

    // Nothing has changed since `open` — the overwhelmingly common beat.
    let t = Instant::now();
    let stats = store.reindex(Reindex::Incremental).unwrap();
    let elapsed = t.elapsed();

    assert_eq!(stats.scanned, N, "every note is still stat'd");
    assert_eq!(stats.updated, 0, "but none is re-read");
    assert_eq!(stats.removed, 0, "and none has vanished");
    assert!(
        elapsed.as_millis() < POLL_BUDGET_MS,
        "a quiet poll over {N} notes took {elapsed:?}, budget is {POLL_BUDGET_MS} ms"
    );
    println!("quiet incremental poll over {N} notes: {elapsed:?}");
}

/// The lost-update guard hashes a note's body on every read and every write, so it sits on
/// the whiteboard save path — which fires on a 600 ms debounce while someone is drawing, on
/// a body that is a whole Excalidraw scene.
///
/// This is the measurement the design asked for before committing to a hash rather than a
/// timestamp, and it is worth recording what it said: **1.6 ms in release, ~41 ms in debug**
/// for a 2.8 MB body. The shipped number is the release one, against a 600 ms debounce that
/// then writes and fsyncs that same body — so the hash is comparable to the write it
/// precedes, not a new cost of its own.
///
/// The budget below is a **debug** budget, because that is what `pixi run ci` runs (the other
/// budgets in this file are the same). It is set to catch an algorithmic regression — an
/// accidental double-hash, a copy per call — not to police the constant factor.
#[test]
fn hashing_a_whiteboard_sized_body_is_a_rounding_error() {
    // Debug is roughly 25x slower than release here; 100 ms leaves headroom on a loaded CI
    // box while still failing loudly if the work stops being one pass over the bytes.
    const BUDGET_MS: u128 = 100;
    // A scene carrying one pasted screenshot: base64 inflates 2 MB to ~2.8 MB.
    let body = "x".repeat(2_800_000);

    let t = Instant::now();
    for _ in 0..10 {
        std::hint::black_box(fm_core::blob::sha256_hex(body.as_bytes()));
    }
    let each = t.elapsed() / 10;

    assert!(
        each.as_millis() < BUDGET_MS,
        "hashing a 2.8 MB body took {each:?}, budget is {BUDGET_MS} ms — it would be felt \
         on the 600 ms whiteboard save debounce"
    );
    println!("sha256 of a 2.8 MB body: {each:?}");
}
