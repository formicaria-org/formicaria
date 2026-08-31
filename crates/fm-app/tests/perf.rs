//! Budgets for the two commands a **feed with inline discussions** makes hot.
//!
//! `outstanding.md` asked for the first of these by name — *"a `perf.rs` budget for `thread()`
//! (measured ~170 ms at 30k notes) — the one 7-item finding not yet covered by a test; **add it
//! before a comments panel makes it hot**"*. This is that comments panel, so this is that budget.
//!
//! Both assert the **shape of the curve**, not a duration, for the reason `fm-core/tests/perf.rs`
//! records: a wall-clock number is a property of whatever machine CI has, while a ratio is a
//! property of the code.

use fm_app::commands::{recent, reply, thread};
use fm_core::{frontmatter, FileStore};
use fm_model::{Kind, Object};
use tempfile::tempdir;

/// A vault of `notes` ordinary notes, with `messages` replies hanging off the first one.
/// Seeded as files directly — `FileStore::open` indexes them once — so setup is cheap and only
/// the command under test is timed.
fn vault(notes: usize, messages: usize) -> (tempfile::TempDir, FileStore, String) {
    let dir = tempdir().unwrap();
    let notes_dir = dir.path().join("notes");
    std::fs::create_dir_all(&notes_dir).unwrap();
    let mut root = String::new();
    for i in 0..notes {
        let mut o = Object::new(Kind::Note, format!("body of note {i}, with a few words in it"));
        o.title = Some(format!("note {i}"));
        if i == 0 {
            root = o.id.to_string();
        }
        std::fs::write(notes_dir.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
            .unwrap();
    }
    let mut store = FileStore::open(dir.path()).unwrap();
    for i in 0..messages {
        reply(&mut store, &root, &format!("message {i}")).unwrap();
    }
    (dir, store, root)
}

/// **`thread()` is linear in the corpus, and this pins that it does not become worse.**
///
/// It is *not* asserted to be linear in the messages returned, and that is the honest statement of
/// this design's central cost. `commands::thread` filters on a bare `NoteRef` with **no `Kind`
/// conjunct** — deliberately and correctly: carrying a well-formed `thread_of` is what makes
/// something a message, and a second, weaker spelling of that condition would also trip the CI grep
/// that keeps the notes-only base in one place. The price is that `kind_pushdown` has nothing to
/// push, so every row is hydrated to find a handful of replies.
///
/// **What that fact bought, in design terms:** a feed must never call this once per row. Thirty
/// visible threads would be thirty whole-corpus reads, serialized on one mutex, each parking the
/// phone's JS thread. So counts come from `thread_roots` (every count in one pass) and `thread()`
/// fires only when a reader opens one discussion — the configuration this budget measures.
///
/// If this ever fails, the fix is **not** to add `Kind(Note)` here. It is to push the `NoteRef`
/// predicate down into the index, or to keep the call count at one.
#[test]
fn a_thread_read_stays_linear_in_the_corpus() {
    fn cost(notes: usize) -> std::time::Duration {
        let (_dir, store, root) = vault(notes, 20);
        // Warm: the first read pays first-touch I/O, which is not what is being measured.
        let _ = thread(&store, &root).unwrap();
        let t = std::time::Instant::now();
        let view = thread(&store, &root).unwrap();
        let e = t.elapsed();
        assert_eq!(view.messages.len(), 20, "the budget must measure a real read");
        e
    }

    let (small, large) = (2_000usize, 8_000usize);
    let per_note_small = cost(small).as_secs_f64() / small as f64;
    let per_note_large = cost(large).as_secs_f64() / large as f64;
    let growth = per_note_large / per_note_small;
    // Printed so the absolute cost is visible when this is run with `--nocapture`: the ratio is
    // what fails the test, but the number is what tells a designer whether to call it per row.
    eprintln!(
        "thread(): {:.1} µs/note at {small}, {:.1} µs/note at {large} — growth {growth:.2}x",
        per_note_small * 1e6,
        per_note_large * 1e6,
    );

    // Linear in the corpus ⇒ ~1.0. Quadratic ⇒ ~4.0 at a 4x corpus. 2.5 sits clear of both, the
    // same margin `fm-core/tests/perf.rs` uses and for the same reason: the failure guarded against
    // is a change of complexity class, not a 20% regression on a noisy runner.
    assert!(
        growth < 2.5,
        "per-note cost of one thread read grew {growth:.1}x when the vault grew 4x \
         ({:.1} µs/note at {small} vs {:.1} µs/note at {large}) — reading one discussion has gone \
         superlinear in the corpus, which is invisible at a developer's vault and unusable at the \
         10k this project claims",
        per_note_small * 1e6,
        per_note_large * 1e6,
    );
}

/// **A feed row's payload is bounded, whatever a note contains.**
///
/// `recent()` is the largest outbound payload in the app and it has **no limit** — every note in
/// scope is serialized, however few the client mounts. So the only thing standing between a vault
/// and a multi-megabyte response is the per-row cap, and this is the test that says so.
///
/// It exists because the tempting way to show "more of the note, fading towards half" is to send a
/// *fraction* of the body — which removes the bound entirely. A whiteboard body is pretty-printed
/// Excalidraw JSON, and `dto.rs` already calls ~2.8 MB "the largest thing a body ever is"; half of
/// one is a 1.4 MB string in a single array element. A char cap keeps the property; a fraction
/// silently discards it. See `decisions.md`, 2026-08-31.
#[test]
fn a_feed_row_never_carries_an_unbounded_body() {
    // One ordinary note, one note whose body is very large, one whiteboard-shaped body of JSON on
    // a single line — the case where a line-based cap does nothing at all.
    let dir = tempdir().unwrap();
    let notes_dir = dir.path().join("notes");
    std::fs::create_dir_all(&notes_dir).unwrap();
    let bodies = [
        "an ordinary short note".to_string(),
        "x".repeat(400_000),
        format!("{{\"type\":\"excalidraw\",\"elements\":[{}]}}", "0,".repeat(100_000)),
    ];
    for (i, body) in bodies.iter().enumerate() {
        let mut o = Object::new(Kind::Note, body.clone());
        o.title = Some(format!("note {i}"));
        std::fs::write(notes_dir.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
            .unwrap();
    }
    let store = FileStore::open(dir.path()).unwrap();

    let rows = recent(&store).unwrap();
    assert_eq!(rows.len(), bodies.len());
    let json = serde_json::to_string(&rows).unwrap();
    eprintln!("recent(): {} B for {} rows ({} B/row)", json.len(), rows.len(), json.len() / rows.len());

    // Generous: the cap on the text fields plus room for every other field, tags and props. The
    // failure this catches is a field that scales with the body, not a field that got a bit longer.
    const CEILING: usize = 4_096;
    assert!(
        json.len() < rows.len() * CEILING,
        "one feed row averaged {} bytes against a {CEILING} B ceiling — a field on `ObjectMeta` is \
         scaling with the note's body. `recent()` has no limit, so that is unbounded payload on \
         the app's largest response, re-fetched on every poll beat",
        json.len() / rows.len(),
    );
}
