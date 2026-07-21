//! Proposals as notes: a note carrying `proposes: branch:<name>` is a proposal — a second hidden
//! note-class. Like a message it is a real `Kind::Note` you do not *plan*, so the planning views
//! exclude it and the Collaboration surface gathers it instead.
//!
//! Two things are proven, and both are the ones a filter-shaped fix silently gets wrong:
//! 1. The value must **parse as a branch reference** before a note is treated as a proposal, so a
//!    stray property value (a board drag, a typo) never turns an ordinary note into one — nor
//!    hides it from the views, the same erase-hazard `thread_of` guards one property over.
//! 2. **`activity()` excludes proposals too** — it is a `git log` read-model with no `Filter`, so
//!    the exclusion is applied by hand there; a proposal is a git touch, and without the guard a
//!    pile of them floods the recent-edits feed exactly as a busy thread would.

use fm_app::commands::{agenda, board, capture, proposals, recent, set_property};
use fm_core::MemoryStore;

/// A note that proposes a branch — the read-half's proposal, created out of band (the in-app
/// "Propose a change" button + `create-proposal` are the deferred write half).
fn proposal(s: &mut MemoryStore, title: &str, branch: &str) -> String {
    let id = capture(s, title, "").unwrap().id;
    set_property(s, &id, "proposes", &format!("branch:{branch}")).unwrap();
    id
}

#[test]
fn a_proposal_is_listed_as_one_and_hidden_from_the_planning_views() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "an ordinary note", "").unwrap().id;
    let prop = proposal(&mut s, "rework the clipping protocol", "rework-clip");

    // It is a proposal — the Collaboration surface's feed lists exactly it.
    let listed: Vec<String> = proposals(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(listed, vec![prop.clone()], "only the proposal is offered on the collaboration surface");

    // ...and therefore not a note you plan: absent from the timeline / `/` note-picker...
    let recent_ids: Vec<String> = recent(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(recent_ids, vec![note], "the proposal never appears where you pick a link target");

    // ...and off the board and agenda entirely, `(none)` column least of all.
    let cards: usize = board(&s, "status").unwrap().columns.iter().map(|c| c.cards.len()).sum();
    assert_eq!(cards, 1, "the proposal never lands on the board");
    assert!(agenda(&s).unwrap().iter().all(|m| m.id != prop), "nor on the agenda");
}

#[test]
fn a_stray_proposes_value_is_not_a_proposal_and_hides_nothing() {
    let mut s = MemoryStore::new();
    let id = capture(&mut s, "a note that mentions proposes in its prose", "").unwrap().id;
    // The kind of value a board drop or a typo would write — not a branch reference.
    set_property(&mut s, &id, "proposes", "doing").unwrap();

    assert!(proposals(&s).unwrap().is_empty(), "a word is not a branch reference");
    let recent_ids: Vec<String> = recent(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(recent_ids, vec![id], "and the note is still visible everywhere");
}

#[test]
fn a_proposal_is_listed_even_when_its_branch_no_longer_exists() {
    // The note outlives the branch (Ruling 15): `proposals()` lists the proposal *note*; whether
    // the branch still resolves is a git question for the read layer, never a reason to hide it.
    let mut s = MemoryStore::new();
    let prop = proposal(&mut s, "a proposal whose branch was merged and deleted", "long-gone");

    let listed: Vec<String> = proposals(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(listed, vec![prop]);
}

/// **`activity()` is the surface a `Predicate` cannot reach**, so it is where a filter-shaped fix
/// silently misses proposals — the exact class of miss the first threads attempt made with
/// messages. Real git, real commits: the contract under test is with `git log`.
#[test]
fn proposals_do_not_flood_the_activity_feed() {
    use fm_core::FileStore;
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    fm_core::git::ensure_repo(dir.path()).unwrap();
    fm_core::git::set_identity(dir.path(), "Tester", "t@example.org").unwrap();

    let note = capture(&mut store, "a real note", "").unwrap().id;
    for i in 0..3 {
        let id = capture(&mut store, &format!("proposal {i}"), "").unwrap().id;
        set_property(&mut store, &id, "proposes", &format!("branch:p{i}")).unwrap();
    }
    let paths = store.written();
    assert!(fm_core::git::commit_all(dir.path(), "auto: seed", &paths).unwrap());

    let events = fm_app::commands::activity(&store, dir.path(), "1 year ago").unwrap();

    assert_eq!(events.len(), 1, "proposals are git touches, but not edits worth showing");
    assert_eq!(events[0].id, note);
}
