//! **What a note costs to save, and whether a big one survives the round trip.**
//!
//! Two gaps this closes, both of which the owner met on a phone before any test did.
//!
//! 1. **Nothing in the tree had ever stored a large note.** The biggest body written through a
//!    `Store` anywhere was ~50 bytes (`fm-app/tests/note.rs`); `perf.rs` hashes a 2.8 MB string
//!    but never indexes, frontmatters, queries or re-reads one. "Freezes with big notes" was
//!    therefore not a claim the suite could evaluate at all.
//! 2. **Nothing measured what a single `put` costs as a vault grows.** `perf.rs` covers search,
//!    the incremental poll, and a full rebuild — all of them whole-corpus operations. The thing
//!    the editor actually does every 500 ms of typing had no budget, and it was quadratic.
//!
//! The second test is the interesting one and it follows `perf.rs`'s own discipline: **assert the
//! shape of the curve, not a duration.** At the few hundred notes a developer has, quadratic is
//! invisible; at the few thousand the owner has, it is the app.

use fm_core::{frontmatter, FileStore, MemoryStore, Store};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use std::time::{Duration, Instant};
use tempfile::tempdir;

/// A long research note. Many lines rather than one enormous line, because that is the shape a
/// person writes and the shape the editor and the merge engine handle differently.
fn big_body(bytes: usize) -> String {
    let mut s = String::with_capacity(bytes + 64);
    let mut i = 0;
    while s.len() < bytes {
        s.push_str(&format!(
            "{i}. The advantage estimate leaks across the meta-update boundary.\n"
        ));
        i += 1;
    }
    s
}

/// **A 2 MB note survives being written, indexed, found and read back.**
///
/// Every step here is one nothing had exercised above a couple of hundred bytes: YAML frontmatter
/// round-tripping, the atomic write, FTS tokenisation of the whole body, and hydration on the way
/// out. The reopen is the load-bearing part — it proves the bytes went to *disk* intact rather
/// than merely surviving in memory, which is the same reason `fm-app/tests/note.rs` reopens.
#[test]
fn a_two_megabyte_note_round_trips_through_disk_and_the_index() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();

    // A needle deep inside the body, so finding it proves the whole thing was tokenised and not
    // just a truncated prefix.
    let mut body = big_body(2_000_000);
    body.push_str("\nthe quokka theorem resolves the marsupial conjecture\n");
    let obj = Object::new(Kind::Note, body.clone());
    let id = obj.id;
    store.put(&obj).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Text("quokka".into())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    assert_eq!(store.query(&q).unwrap().total, 1, "a needle 2 MB into the body must be searchable");

    // Reopen over the same directory: a second index, built from the files.
    drop(store);
    let reopened = FileStore::open(dir.path()).unwrap();
    let back = reopened.get(id).unwrap().expect("the note must still be there");
    assert_eq!(back.body, body, "the body changed on the way through disk");
    assert_eq!(
        reopened.query(&q).unwrap().total,
        1,
        "the rebuilt index must still find the needle"
    );
}

/// **Re-saving a note must not leave a second copy of it in the full-text index.**
///
/// The direct check on the `fts_rowid` mechanism, and the reason it is worth its own test: the
/// rowid is captured with `last_insert_rowid()` after an `INSERT` into an **fts5 virtual table**,
/// and every later delete trusts it. If it were ever wrong — off by one, stale, or not maintained
/// by fts5 the way an ordinary table maintains it — the delete would remove *someone else's* row
/// and leave this note's behind. The visible symptom is a note appearing twice in search results
/// while the note itself is perfectly fine, and the second symptom is another note silently
/// dropping out of search entirely. Neither shows up in a body round trip, so neither of the
/// tests above would notice.
///
/// Twenty saves, because a one-off could coincide.
#[test]
fn re_saving_a_note_never_duplicates_it_in_search() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();

    // A second note, so a mis-aimed delete has something to hit.
    let bystander = Object::new(Kind::Note, "a bystander mentioning quokka once");
    store.put(&bystander).unwrap();

    let mut obj = Object::new(Kind::Note, "the quokka theorem, revision 0");
    let id = obj.id;
    store.put(&obj).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Text("quokka".into())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };

    for round in 1..=20 {
        obj = store.get(id).unwrap().unwrap();
        obj.body = format!("the quokka theorem, revision {round}");
        store.put(&obj).unwrap();

        let r = store.query(&q).unwrap();
        assert_eq!(
            r.total, 2,
            "after {round} saves the index holds {} rows for two notes — a duplicated or a \
             vanished fts row means the remembered fts_rowid is not what the delete needs",
            r.total,
        );
        assert_eq!(
            r.rows.iter().filter(|o| o.id == id).count(),
            1,
            "the re-saved note appears more than once after {round} saves",
        );
        assert!(
            r.rows.iter().any(|o| o.id == bystander.id),
            "the bystander fell out of the index after {round} saves — the delete hit the wrong row",
        );
    }
}

/// The same, for **deletion** — the other caller that now seeks by rowid.
#[test]
fn deleting_a_note_removes_exactly_its_own_search_row() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();

    let keep = Object::new(Kind::Note, "keep me, I mention quokka");
    let drop = Object::new(Kind::Note, "drop me, I mention quokka too");
    store.put(&keep).unwrap();
    store.put(&drop).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Text("quokka".into())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    assert_eq!(store.query(&q).unwrap().total, 2);

    store.delete(drop.id).unwrap();
    let r = store.query(&q).unwrap();
    assert_eq!(r.total, 1, "deleting one note must leave exactly the other");
    assert_eq!(r.rows[0].id, keep.id, "the wrong note's search row was removed");
}

/// The same, through the in-memory backend, because the two are meant to stay behaviour-equivalent
/// and a size-dependent divergence is exactly the kind that hides.
#[test]
fn the_memory_store_agrees_on_a_large_body() {
    let mut store = MemoryStore::new();
    let body = big_body(1_000_000);
    let obj = Object::new(Kind::Note, body.clone());
    let id = obj.id;
    store.put(&obj).unwrap();
    assert_eq!(store.get(id).unwrap().unwrap().body, body);
}

/// **What one save costs, as the vault grows.**
///
/// `FileStore::put` → `index_object`, which refreshes the note's full-text row. `fts` is declared
/// `id UNINDEXED`, and in FTS5 that stores a column *without indexing it* — so
/// `DELETE FROM fts WHERE id = ?` could not seek and walked the entire table. A `fresh` flag had
/// already been added to skip that during a full rebuild (which is why `perf.rs`'s rebuild test
/// passes), but **every ordinary write still paid it**: every keystroke pause in the editor, every
/// board drag, every `set_property`, growing with the size of the vault.
///
/// This asserts the growth ratio rather than a duration, for the reason `perf.rs` gives: a
/// wall-clock budget would not have caught it, because at a developer's note count quadratic is
/// invisible.
///
/// **The threshold is calibrated against both states, measured, not guessed.** Three runs each on
/// the development machine, debug profile:
///
/// | | 2 000 notes | 8 000 notes | growth |
/// |---|---|---|---|
/// | seeking the fts row by `rowid` | ~318 µs | ~378 µs | **1.17 – 1.20×** |
/// | scanning it by `id` (the bug) | ~499 µs | ~1.09 ms | **2.16 – 2.24×** |
///
/// So 1.6 sits clear of both, with more margin below the broken range than above the fixed one.
/// Note the growth is well under the 4× the scan alone implies: a `put` also fsyncs and
/// re-serialises YAML, and that constant cost dilutes the ratio. Which is the argument *for*
/// checking the shape rather than the constant — the absolute numbers here are dominated by
/// something that is not the defect.
#[test]
fn saving_one_note_does_not_get_slower_as_the_vault_grows() {
    /// Seed `n` notes as files (cheap, untimed), open once, then time a fixed number of `put`s.
    fn put_cost(n: usize) -> Duration {
        const WRITES: usize = 40;

        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        for i in 0..n {
            let o = Object::new(Kind::Note, format!("routine note number {i} about estimators"));
            std::fs::write(notes.join(format!("{}.md", o.id)), frontmatter::to_file(&o).unwrap())
                .unwrap();
        }
        let mut store = FileStore::open(dir.path()).unwrap();

        // The notes we will rewrite. Created first so the timed loop is pure re-save — an insert
        // and an update take different paths and mixing them would blur what is being measured.
        let mut ids = Vec::with_capacity(WRITES);
        for i in 0..WRITES {
            let o = Object::new(Kind::Note, format!("note under test {i}"));
            store.put(&o).unwrap();
            ids.push(o.id);
        }

        let t = Instant::now();
        for (round, id) in ids.iter().enumerate() {
            let mut o = store.get(*id).unwrap().unwrap();
            o.body = format!("note under test, revision {round}");
            store.put(&o).unwrap();
        }
        t.elapsed() / WRITES as u32
    }

    let small = put_cost(2_000);
    let large = put_cost(8_000);
    let growth = large.as_secs_f64() / small.as_secs_f64().max(f64::EPSILON);

    println!("per-put at 2k: {small:?}, at 8k: {large:?} (growth {growth:.2}x over a 4x vault)");
    assert!(
        growth < 1.6,
        "saving one note got {growth:.2}x more expensive as the vault grew 4x \
         (2k: {small:?}, 8k: {large:?}). A write should not scale with the corpus — check that \
         index_object still deletes the fts row by rowid rather than by the UNINDEXED id.",
    );
}

/// **The same question, on the polling path — which is a different code path, and was broken
/// while the one above was green.**
///
/// `ping` runs `Reindex::Incremental` every 15 s, and the Android cold start now opens through it
/// (`ColdStart::TrustIndex`). `perf.rs` already budgets the *quiet* case — nothing changed, so
/// nothing is re-indexed — and that is precisely why this gap survived: the quiet path never
/// reaches `index_object` at all.
///
/// When notes *have* changed, `reindex` calls `forget_path` (which drops the `objects` row) and
/// then `index_object`. So a rowid remembered in `objects` is gone by the time `index_object`
/// looks for it, and the delete fell back to the `UNINDEXED` `id` scan — one full FTS table walk
/// **per changed note**, on the path that runs three times a minute. The fix is that both callers
/// having already deleted by rowid, `index_object` is told the row is `fresh`.
///
/// **It measures the *marginal* cost of a changed note, by subtracting a quiet poll.**
///
/// The first version of this test divided the whole incremental pass by the number of changed
/// notes, and failed at 3.7× on correct code. An incremental poll has an inherent O(n) component
/// — `SELECT path, mtime_ns` over every row, a `read_dir`, and a `stat` per file — which
/// `perf.rs` already budgets separately as the *quiet* case. Dividing the total by 40 was mostly
/// measuring that sweep, and it grew with the vault exactly as it is supposed to.
///
/// So the quiet pass is timed first at the same `n` and subtracted. What is left is what
/// re-indexing one note actually costs, which is the thing that must not scale with the corpus.
///
/// **Calibrated against both states, measured over repeated runs**, debug profile, 200 changed
/// notes:
///
/// | | 2 000 notes | 8 000 notes | growth |
/// |---|---|---|---|
/// | `fresh: true` after deleting by rowid | ~127 µs | ~150–176 µs | **1.19 – 1.43×** |
/// | falling through to the `id` scan | ~345 µs | ~950 µs – 1.0 ms | **2.82 – 2.89×** |
///
/// 2.0 sits clear of both. The residual above 1.0 in the fixed column is fts5's own index
/// maintenance (a deeper b-tree, incremental merges), not our SQL — which is why the budget is a
/// shape check and not a duration.
#[test]
fn an_incremental_poll_stays_linear_in_what_changed() {
    /// Seed `n` notes, open, time a quiet poll, then touch `CHANGED` of them and time another —
    /// returning the difference per changed note.
    fn poll_cost(n: usize) -> Duration {
        // Enough changed notes that the marginal term dominates noise in the subtracted baseline.
        // At 40 the two were comparable and the ratio swung between 1.07 and 1.59 run to run,
        // which is a measurement problem masquerading as a signal.
        const CHANGED: usize = 200;

        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        let mut paths = Vec::new();
        for i in 0..n {
            let o = Object::new(Kind::Note, format!("routine note number {i} about estimators"));
            let p = notes.join(format!("{}.md", o.id));
            std::fs::write(&p, frontmatter::to_file(&o).unwrap()).unwrap();
            paths.push(p);
        }
        let mut store = FileStore::open(dir.path()).unwrap();

        // **Both terms are the minimum of several passes, not one sample** (2026-09-10). The
        // marginal cost is a *difference* of two timings, so noise in either survives the
        // subtraction — and at 8k notes the floor is a stat of 8k files, whose variance on a
        // shared 4-core runner is larger than the marginal term being measured. A single sample
        // gave 2.56x on CI against a 2.0 threshold while this machine read ~1.3x.
        //
        // The minimum is the right estimator here: scheduling noise, page-cache misses and a busy
        // neighbour can only ever make a pass *slower*, so the fastest of several is the closest
        // any of them got to the real cost. This is the same lesson the comment above records —
        // "a measurement problem masquerading as a signal" — met a second time at a larger scale.
        const PASSES: usize = 3;

        // The floor: everything this pass does that is not about a changed note.
        let mut baseline = Duration::MAX;
        for _ in 0..PASSES {
            let t = Instant::now();
            let quiet = store.reindex(fm_core::Reindex::Incremental).unwrap();
            baseline = baseline.min(t.elapsed());
            assert_eq!(quiet.updated, 0, "the baseline pass must find nothing to do");
        }

        let mut marginal = Duration::MAX;
        for pass in 0..PASSES {
            // Rewrite a fixed number of files behind the store's back — a pull, or another device.
            // The body differs per pass, or the second round would find nothing changed.
            for p in paths.iter().take(CHANGED) {
                let content = std::fs::read_to_string(p).unwrap();
                let mut o = fm_core::frontmatter::from_file(&content).unwrap();
                o.body = format!("changed by a pull, round {pass}");
                std::fs::write(p, frontmatter::to_file(&o).unwrap()).unwrap();
            }

            let t = Instant::now();
            let stats = store.reindex(fm_core::Reindex::Incremental).unwrap();
            let elapsed = t.elapsed();
            assert_eq!(
                stats.updated, CHANGED,
                "the poll must have re-indexed exactly what changed"
            );
            marginal = marginal.min(elapsed.saturating_sub(baseline));
        }
        marginal / CHANGED as u32
    }

    let small = poll_cost(2_000);
    let large = poll_cost(8_000);
    let growth = large.as_secs_f64() / small.as_secs_f64().max(f64::EPSILON);

    println!("per-changed-note poll at 2k: {small:?}, at 8k: {large:?} (growth {growth:.2}x)");
    assert!(
        growth < 2.0,
        "re-indexing one changed note got {growth:.2}x more expensive as the vault grew 4x \
         (2k: {small:?}, 8k: {large:?}). The poll must scale with what CHANGED, not with the \
         corpus — check that reindex still deletes by rowid and passes `fresh: true` to \
         index_object, rather than letting it fall back to the UNINDEXED id scan.",
    );
}
