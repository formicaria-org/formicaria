//! **A note that leaves the store keeps its clock** (`decisions.md#vault`, 2026-10-24, F17/F18).
//!
//! A vault's times are written on the vault's clock and read onto this device's (`FileStore`'s
//! seam). A proposal, an undo and a restore all carry a note's text through git rather than
//! through `put`, and each used to write or read that text on the wrong clock: accepting a proposal
//! that only renamed a timed note moved its meeting by the gap between the two clocks.
//!
//! **Its own test binary, on purpose:** the device's zone is the process's `TZ`, set here before
//! anything reads it, so the two clocks always differ — `Asia/Tokyo` (+09:00, no daylight saving)
//! against a vault on `+01:00` — whatever machine runs the suite.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore, Store};
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

/// A vault on `+01:00`, read on a device on `+09:00`, with one note at 10:00 device time.
fn vault() -> (tempfile::TempDir, FileStore, String) {
    std::env::set_var("TZ", "Asia/Tokyo");
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    // The repo carries its own committer: a runner has no global `user.email` to fall back on.
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Tester"],
        &["config", "user.email", "tester@example.com"],
    ] {
        let ok = Command::new("git").args(args).current_dir(p).status().unwrap().success();
        assert!(ok, "git {args:?}");
    }
    std::fs::write(p.join("vault.json"), r#"{"notes_time_zone": "+01:00"}"#).unwrap();
    let mut store = FileStore::open(p).unwrap();
    let id = commands::capture(&mut store, "# Meeting\n\nthe agenda", "").unwrap().id;
    commands::set_property(&mut store, &id, "start", "2026-11-02T10:00").unwrap();
    git::commit_all(p, "note", &store.written()).unwrap();
    (dir, store, id)
}

fn on_disk(p: &Path, id: &str, key: &str) -> String {
    let text = std::fs::read_to_string(p.join(format!("notes/{id}.md"))).unwrap();
    text.lines().find(|l| l.starts_with(&format!("{key}:"))).unwrap_or_default().to_string()
}

fn start_read(store: &FileStore, id: &str) -> String {
    store.get(id.parse().unwrap()).unwrap().unwrap().start.unwrap().to_string()
}

#[test]
fn the_two_clocks_really_differ() {
    if !have_git() {
        return;
    }
    let (dir, store, id) = vault();
    assert_eq!(
        on_disk(dir.path(), &id, "start"),
        "start: 2026-11-02T02:00",
        "on the vault's clock"
    );
    assert_eq!(start_read(&store, &id), "2026-11-02T10:00", "read onto the device's");
}

#[test]
fn accepting_a_proposal_that_does_not_touch_a_time_leaves_it_where_it_was() {
    if !have_git() {
        return;
    }
    let (dir, mut store, id) = vault();
    let p = dir.path();
    let before = on_disk(p, &id, "start");
    let props = serde_json::json!({ "title": "Meeting — renamed" });
    let prop = commands::create_proposal_with(
        &mut store,
        p,
        &id,
        "# Meeting\n\nthe agenda, revised",
        Some(&props),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    // The review shows the time it would keep, on this device's clock.
    let shown = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert_eq!(
        shown.start.as_deref(),
        Some("2026-11-02T10:00"),
        "the review is on the device's clock"
    );
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();
    commands::accept_proposal(&mut store, p, &prop.id).unwrap();
    assert_eq!(on_disk(p, &id, "start"), before, "the meeting did not move");
    assert_eq!(start_read(&store, &id), "2026-11-02T10:00");
}

#[test]
fn a_proposed_new_meeting_lands_at_the_time_it_was_proposed() {
    if !have_git() {
        return;
    }
    let (dir, mut store, id) = vault();
    let p = dir.path();
    let mut draft = fm_model::Object::new(fm_model::Kind::Note, "Next meeting");
    draft.vault = store.get(id.parse().unwrap()).unwrap().unwrap().vault;
    fm_core::apply_property(&mut draft, "start", "2026-11-09T16:30").unwrap();
    let new_id = draft.id.to_string();
    let prop = commands::propose_new_note(
        &mut store,
        p,
        draft,
        None,
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();
    commands::accept_proposal(&mut store, p, &prop.id).unwrap();
    store.reindex(fm_core::Reindex::Incremental).unwrap();
    assert_eq!(on_disk(p, &new_id, "start"), "start: 2026-11-09T08:30", "on the vault's clock");
    assert_eq!(start_read(&store, &new_id), "2026-11-09T16:30", "where it was proposed");
}

#[test]
fn undoing_a_delete_and_restoring_from_history_move_nothing() {
    if !have_git() {
        return;
    }
    let (dir, mut store, id) = vault();
    let p = dir.path();
    let before = on_disk(p, &id, "start");
    let vault = store.get(id.parse().unwrap()).unwrap().unwrap().vault;

    // The session's undo: the kept text comes back.
    let gone = commands::delete_keeping(&mut store, &id).unwrap();
    commands::restore_note(&mut store, &vault, &gone.file).unwrap();
    assert_eq!(on_disk(p, &id, "start"), before, "undo");
    git::commit_all(p, "restored", &store.written()).unwrap();

    // *Recently deleted*: the text git kept, which is the file as it was on disk.
    let raw = std::fs::read_to_string(p.join(format!("notes/{id}.md"))).unwrap();
    store.delete(id.parse().unwrap()).unwrap();
    commands::restore_note(&mut store, &vault, &raw).unwrap();
    assert_eq!(on_disk(p, &id, "start"), before, "recently deleted");
    assert_eq!(start_read(&store, &id), "2026-11-02T10:00");
}
