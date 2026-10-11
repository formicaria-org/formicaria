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
    eprintln!(
        "recent(): {} B for {} rows ({} B/row)",
        json.len(),
        rows.len(),
        json.len() / rows.len()
    );

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

/// **A year or three of logged entries must not change the complexity of the planning views.**
///
/// A logged entry (`decisions.md#data`, 2026-10-11) is excluded from the board by a predicate the
/// index cannot answer — only `Kind` and text reach SQL — so every entry is still read and parsed
/// on every board fetch, then dropped. That is a cost in proportion to the entries, and this pins
/// that it is *only* that: the cost per object must not grow when nine in ten objects are entries.
///
/// It prints the absolute numbers, because the ratio guards the complexity class and the number is
/// what says whether the exclusion needs to be pushed down into the index. **If this fails, that is
/// the fix** — a third pushdown beside `Kind` and text — not a cache.
#[test]
fn logged_entries_cost_the_board_no_more_per_object_than_notes_do() {
    use fm_app::commands::board;
    use fm_app::log::{create_notes, log_days, NoteDraft};
    use fm_app::views::totals_named;

    fn timed(notes: usize, entries: usize) -> (std::time::Duration, std::time::Duration) {
        let (_dir, mut store, _) = vault(notes, 0);
        let per_day = 8;
        let mut made = 0;
        let mut day = time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
        while made < entries {
            let drafts: Vec<NoteDraft> = (0..per_day.min(entries - made))
                .map(|i| {
                    serde_json::from_value(serde_json::json!({
                        "title": format!("food {i}"),
                        "start": format!("{:04}-{:02}-{:02}", day.year(), day.month() as u8, day.day()),
                        "props": { "log": "food", "slot": "lunch", "kcal": 200 + i, "protein_mg": 9000 },
                    }))
                    .unwrap()
                })
                .collect();
            made += drafts.len();
            create_notes(&mut store, "", &drafts).unwrap();
            day = day.next_day().unwrap();
        }
        let _ = board(&store, "status").unwrap(); // warm
        let t = std::time::Instant::now();
        let b = board(&store, "status").unwrap();
        let on_board = t.elapsed();
        let cards: usize = b.columns.iter().map(|c| c.cards.len()).sum();
        assert_eq!(cards, notes, "no entry may reach the board");

        let def = totals_named(&[], "Day").unwrap();
        let t = std::time::Instant::now();
        let r = log_days(&store, &def, "2024-01-01", "2024-01-14", true).unwrap();
        let one_fortnight = t.elapsed();
        assert_eq!(r.days.len(), 14);
        (on_board, one_fortnight)
    }

    let notes = 1_000usize;
    let entries = 9_000usize;
    let (alone, _) = timed(notes, 0);
    let (crowded, fortnight) = timed(notes, entries);
    let per_alone = alone.as_secs_f64() / notes as f64;
    let per_crowded = crowded.as_secs_f64() / (notes + entries) as f64;
    let growth = per_crowded / per_alone;
    eprintln!(
        "board(): {:.1} ms over {notes} notes; {:.1} ms with {entries} entries beside them \
         ({:.1} vs {:.1} µs/object, {growth:.2}x). log_days(14 days of {entries}): {:.1} ms",
        alone.as_secs_f64() * 1e3,
        crowded.as_secs_f64() * 1e3,
        per_alone * 1e6,
        per_crowded * 1e6,
        fortnight.as_secs_f64() * 1e3,
    );
    assert!(
        growth < 2.5,
        "an excluded entry costs the board {growth:.1}x what a note does per object — the \
         exclusion has stopped being a cheap filter. Push `log` down into the index."
    );
}

/// **The Library's *Everything* lists logged entries — and must stay a list a phone can draw.**
///
/// It is the one view that leaves entries in (`decisions.md#ui`, 2026-10-12), so it is the one
/// place ten thousand of them could reach a screen. Two things are pinned: it hands back no more
/// than its `limit` while saying how many there are, and reading them costs no more per object
/// than the board pays to read and drop them.
#[test]
fn everything_over_ten_thousand_entries_is_bounded_and_costs_what_the_board_does() {
    use fm_app::commands::board;
    use fm_app::log::{create_notes, NoteDraft};
    use fm_app::views::run_shipped;

    let notes = 1_000usize;
    let entries = 10_000usize;
    let (_dir, mut store, _) = vault(notes, 0);
    let mut made = 0;
    while made < entries {
        let drafts: Vec<NoteDraft> = (0..200.min(entries - made))
            .map(|i| {
                serde_json::from_value(serde_json::json!({
                    "title": format!("food {i}"),
                    "start": "2026-01-01",
                    "props": { "log": "food", "slot": "lunch", "kcal": 200 + i },
                }))
                .unwrap()
            })
            .collect();
        made += drafts.len();
        create_notes(&mut store, "", &drafts).unwrap();
    }

    let _ = board(&store, "status").unwrap(); // warm
    let t = std::time::Instant::now();
    let _ = board(&store, "status").unwrap();
    let on_board = t.elapsed();

    let t = std::time::Instant::now();
    let all = run_shipped(&store, "Everything").expect("it ships with the app").unwrap();
    let everything = t.elapsed();

    let rows = all.rows.expect("a timeline view has rows");
    assert_eq!(rows.len(), 300, "no more than its limit reaches the screen");
    assert_eq!(all.total, Some(notes + entries), "and it says how many there are");
    assert!(
        rows.iter().any(|r| r.props.contains_key("log")),
        "the newest are entries: this is the view that shows them"
    );

    let ratio = everything.as_secs_f64() / on_board.as_secs_f64();
    eprintln!(
        "Everything over {notes} notes and {entries} entries: {:.1} ms; board(): {:.1} ms ({ratio:.2}x)",
        everything.as_secs_f64() * 1e3,
        on_board.as_secs_f64() * 1e3,
    );
    assert!(
        ratio < 2.5,
        "Everything costs {ratio:.1}x the board over the same vault — sorting every entry to show \
         300 has stopped being cheap. Push the order and the limit down into the index."
    );
}
