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

/// **A full rebuild must stay linear in the note count.**
///
/// This is the test whose absence let an O(n²) rebuild ship. A wall-clock budget alone would not
/// have caught it: at the few hundred notes a developer actually has, quadratic is invisible, and
/// the cost only becomes absurd at the 10k the project claims to support. So this asserts the
/// *shape* of the curve, not a duration — the per-note cost at 4x the notes must not have grown.
///
/// What it caught, measured on 2026-07-20 before the fix: 2 500 notes 2.2 s, 5 000 notes 10.3 s,
/// 10 000 notes 54.7 s — ~5x per doubling. Two unindexed scans per note were responsible:
/// `forget_path` deleting by `objects.path`, which had no index, and a `DELETE FROM fts` against
/// an `UNINDEXED` id column. Afterwards: 0.29 s at 10 000, and a flat per-note cost.
///
/// Deliberately a ratio with a wide margin rather than a tight time: this runs on whatever machine
/// CI has, and the failure being guarded against is 4x per doubling, not 20%.
#[test]
fn a_full_rebuild_stays_linear_in_the_note_count() {
    /// Seed `n` notes and return how long one `Reindex::Full` over them takes.
    fn rebuild_cost(n: usize) -> std::time::Duration {
        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        for i in 0..n {
            let mut o = Object::new(Kind::Note, format!("body of note {i} with a few words in it"));
            o.title = Some(format!("note {i}"));
            std::fs::write(notes.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
                .unwrap();
        }
        // `open` already performs one full rebuild; time a second so the files are cache-warm
        // and the measurement is of the rebuild rather than of first-touch I/O.
        let mut store = FileStore::open(dir.path()).unwrap();
        let t = Instant::now();
        store.reindex(Reindex::Full).unwrap();
        t.elapsed()
    }

    let small = 2_000usize;
    let large = 8_000usize;
    let per_note_small = rebuild_cost(small).as_secs_f64() / small as f64;
    let per_note_large = rebuild_cost(large).as_secs_f64() / large as f64;
    let growth = per_note_large / per_note_small;

    // Linear ⇒ ~1.0. Quadratic ⇒ ~4.0 at a 4x note count. 2.5 sits clear of both, so this fails
    // on a reintroduced quadratic and not on a noisy machine.
    assert!(
        growth < 2.5,
        "per-note rebuild cost grew {growth:.1}x when the vault grew 4x \
         ({:.1} µs/note at {small} vs {:.1} µs/note at {large}) — a full rebuild has gone \
         superlinear again, which is invisible at small vaults and unusable at large ones",
        per_note_small * 1e6,
        per_note_large * 1e6,
    );
}

/// **A planning view must not pay for the bodies it is about to discard.**
///
/// Every budget above this one seeds ~55-byte bodies, so all of them measure *note count* and
/// none can see *bytes per note* — and bytes per note is the axis a library of papers moves.
/// `ingest` puts a PDF's extracted text in the **body of its asset note**
/// (`commands::asset_note`), which measures 1.7–2.2 KB per page, so a few thousand papers is
/// >100 MB of body. `board`/`agenda`/`recent`/`timeline` all filter `Kind(Note)` and never want
/// any of it.
///
/// Until 2026-08-29 `FileStore::candidates` pushed only `Predicate::Text` into SQL and answered
/// everything else with `load_all()`, so each of those views hydrated — YAML-parsed, then copied
/// the body twice — every asset note on every switch.
///
/// **The assertion is a ratio, not a millisecond count**, for the reason `known-issues.md` records
/// after the O(n²) rebuild: a wall-clock budget only catches what someone thought to measure at
/// the right size, while a shape holds on any machine. Same note count, same query, the only
/// difference being asset notes the filter excludes — so the honest answer is "about the same".
#[test]
fn a_planning_view_does_not_hydrate_the_assets_it_filters_out() {
    const NOTES: usize = 300;
    const ASSETS: usize = 600;
    // ~60 KB ≈ a thirty-page paper's extracted text.
    const PDF_TEXT: usize = 60_000;

    fn timed_board(with_assets: bool) -> (std::time::Duration, usize) {
        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        for i in 0..NOTES {
            let o = Object::new(Kind::Note, format!("a plan about step {i}"));
            std::fs::write(notes.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
                .unwrap();
        }
        if with_assets {
            let text = "lorem ipsum dolor sit amet ".repeat(PDF_TEXT / 27);
            for _ in 0..ASSETS {
                let mut o = Object::new(Kind::Asset, text.clone());
                o.title = Some("paper.pdf".into());
                std::fs::write(
                    notes.join(format!("{}.md", o.id)),
                    frontmatter::to_file(&o).unwrap(),
                )
                .unwrap();
            }
        }
        let store = FileStore::open(dir.path()).unwrap();

        // What every planning view asks for.
        let q = Query {
            filter: Filter { all: vec![Predicate::Kind(vec![Kind::Note])] },
            sort: vec![SortKey::desc("updated")],
            ..Default::default()
        };
        let t = Instant::now();
        let page = store.query(&q).unwrap();
        (t.elapsed(), page.total)
    }

    let (bare, bare_total) = timed_board(false);
    let (mixed, mixed_total) = timed_board(true);

    assert_eq!(bare_total, NOTES, "the notes-only vault holds only notes");
    assert_eq!(mixed_total, NOTES, "and the mixed vault answers with the same notes");

    // Measured: ~1.4-1.6x with the pushdown (stable across seven unloaded and three CPU-loaded
    // runs — load raises both halves, so it *compresses* the ratio rather than inflating it), and
    // ~4.9x without. 2.0 keeps ~25% headroom over the honest value while still failing a clean 2x
    // regression, which 3.0 would have waved through.
    let ratio = mixed.as_secs_f64() / bare.as_secs_f64().max(1e-9);
    println!(
        "board over {NOTES} notes: bare {bare:?}, with {ASSETS} asset notes \
         ({} MB of extracted text) {mixed:?} — ratio {ratio:.1}x",
        ASSETS * PDF_TEXT / 1_000_000
    );
    assert!(
        ratio < 2.0,
        "a Kind(Note) view got {ratio:.1}x slower merely because the vault also holds \
         {ASSETS} asset notes it filters out ({bare:?} -> {mixed:?}). The bodies are being \
         hydrated before the filter runs — see `FileStore::candidates`."
    );
}
