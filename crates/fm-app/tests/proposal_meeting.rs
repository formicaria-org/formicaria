//! The two proposal shapes the assistant's meeting pass uses (`decisions.md` 2026-10-09, *the
//! assistant proposes meetings*), end to end over a real git vault:
//!
//! 1. a proposal that also sets a note's `start`/`due`/`location` and adds a tag — the conversation
//!    note becoming its first meeting — and keeps those when a reviewer edits only the text;
//! 2. a proposal of a **new** note — a further meeting — which exists only on its branch until
//!    accepted, and leaves nothing on `main` when rejected.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore, Store};
use fm_model::Id;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn vault_with_a_conversation() -> (tempfile::TempDir, FileStore, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let note = commands::capture(
        &mut store,
        "### Maria — 2026-10-07 09:12\n\nCi vediamo giovedì alle 15.\n",
        "",
    )
    .unwrap();
    git::commit_all(dir.path(), "first", &store.written()).unwrap();
    (dir, store, note.id)
}

#[test]
fn a_proposal_can_set_when_and_where_and_a_reviewers_edit_keeps_it() {
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    let p = dir.path();
    let props = serde_json::json!({
        "start": "2026-10-16T15:00", "due": "2026-10-16T15:00", "location": "COM1", "addTags": ["meeting"]
    });
    let body = "> Meeting: gio 16 ott 15:00\n\n### Maria — 2026-10-07 09:12\n\nCi vediamo giovedì alle 15.\n";
    let prop = commands::create_proposal_with(
        &mut store,
        p,
        &host,
        body,
        Some(&props),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();

    // main is untouched until accepted.
    let id: Id = host.parse().unwrap();
    assert!(store.get(id).unwrap().unwrap().start.is_none());

    // A reviewer edits only the text: the date must survive the revise.
    commands::create_proposal(
        &mut store,
        p,
        &host,
        &format!("{body}\nmy edit\n"),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let content = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert!(content.body.contains("my edit"));
    assert_eq!(
        content.start.as_deref(),
        Some("2026-10-16T15:00"),
        "the date is shown in plain view"
    );
    assert_eq!(content.location.as_deref(), Some("COM1"));
    assert!(!content.is_new);

    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::Merged);
    let file = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(file.contains("start: 2026-10-16T15:00"), "{file}");
    assert!(file.contains("location: COM1"), "{file}");
    assert!(file.contains("meeting"), "{file}");
    assert!(file.contains("my edit"), "the reviewer's text and the date both landed: {file}");
}

#[test]
fn a_reviewer_can_correct_when_and_where_and_keeps_the_tag() {
    // The review shows a proposal's details beside the note's current ones and lets the person change
    // them before accepting. A correction sends only the fields it shows, so it must go OVER the
    // proposal's stored set — never drop the tag the proposal was made with.
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    let p = dir.path();
    let props = serde_json::json!({
        "start": "2026-10-16T15:00", "due": "2026-10-16T15:00", "location": "COM1", "addTags": ["meeting"]
    });
    let prop = commands::create_proposal_with(
        &mut store,
        p,
        &host,
        "the conversation",
        Some(&props),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    let before = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert_eq!(before.was_start, None, "the note has no date yet");
    assert_eq!(before.was_location, None);

    let fix = serde_json::json!({
        "start": "2026-10-16T17:00", "due": "2026-10-16T18:00", "location": "https://zoom.us/j/1"
    });
    commands::create_proposal_with(
        &mut store,
        p,
        &host,
        "the conversation",
        Some(&fix),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let after = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert_eq!(after.start.as_deref(), Some("2026-10-16T17:00"));
    assert_eq!(after.due.as_deref(), Some("2026-10-16T18:00"));
    assert_eq!(after.location.as_deref(), Some("https://zoom.us/j/1"));

    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::Merged);
    let file = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(file.contains("start: 2026-10-16T17:00"), "{file}");
    assert!(file.contains("- meeting"), "the tag survived the correction: {file}");

    // Once the note has a date, a later proposal shows it as the value it would change from. (The
    // accept merged on disk; reopen, as the app re-reads after an accept.)
    let mut store = FileStore::open(p).unwrap();
    commands::create_proposal_with(
        &mut store,
        p,
        &host,
        "the conversation, moved",
        Some(&serde_json::json!({ "start": "2026-10-17T10:00" })),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .map(|next| {
        let c = commands::proposal_content(&store, p, &next.id).unwrap().unwrap();
        assert_eq!(c.was_start.as_deref(), Some("2026-10-16T17:00"));
        assert_eq!(c.was_location.as_deref(), Some("https://zoom.us/j/1"));
        assert_eq!(c.start.as_deref(), Some("2026-10-17T10:00"));
    })
    .unwrap();
}

#[test]
fn a_title_proposed_over_a_waiting_meeting_keeps_the_meeting() {
    // The title arrives after the meeting was proposed: it revises that proposal, and both land.
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    let p = dir.path();
    let meeting = serde_json::json!({ "start": "2026-10-16T15:00", "due": "2026-10-16T15:00" });
    let prop = commands::create_proposal_with(
        &mut store,
        p,
        &host,
        "the conversation",
        Some(&meeting),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    commands::create_proposal_with(
        &mut store,
        p,
        &host,
        "the conversation",
        Some(&serde_json::json!({ "title": "Maria Rossi — Master student, robotics" })),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let c = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert_eq!(c.title, "Maria Rossi — Master student, robotics");
    assert_eq!(c.start.as_deref(), Some("2026-10-16T15:00"), "the meeting is still proposed");
    assert_ne!(
        c.was_title.as_deref(),
        Some(c.title.as_str()),
        "what it renames from is the note's own"
    );

    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::Merged);
    let file = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(file.contains("title: Maria Rossi — Master student, robotics"), "{file}");
    assert!(file.contains("start: 2026-10-16T15:00"), "{file}");
}

#[test]
fn a_proposal_may_not_change_anything_outside_the_allowlist() {
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    // `title` was the example here until 2026-10-15, when a conversation's title joined the allowlist;
    // `status` stays outside it.
    let props = serde_json::json!({ "status": "done" });
    let err = commands::create_proposal_with(
        &mut store,
        dir.path(),
        &host,
        "x",
        Some(&props),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("cannot change 'status'"), "{err}");
}

#[test]
fn a_proposed_new_note_exists_only_on_its_branch_until_accepted() {
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    let p = dir.path();
    let mut draft = fm_model::Object::new(fm_model::Kind::Note, "> «ci rivediamo il 28 alle 10»\n");
    draft.title = Some("Revisione progetto (2)".into());
    fm_core::apply_property(&mut draft, "start", "2026-10-28T10:00").unwrap();
    draft.tags = vec!["meeting".into(), "mail".into()];
    let new_id = draft.id;
    let prop = commands::propose_new_note(
        &mut store,
        p,
        draft,
        Some(host.parse().unwrap()),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();

    let path = p.join(format!("notes/{new_id}.md"));
    assert!(!path.exists(), "nothing on main before a person accepts");
    assert!(store.get(new_id).unwrap().is_none());

    // The review reads it from the branch, and an edit there works although main has no such note.
    let content = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert_eq!(content.host, new_id.to_string());
    assert!(content.is_new, "the review says it adds a note");
    assert_eq!(content.start.as_deref(), Some("2026-10-28T10:00"));
    assert!(
        commands::vault_of_proposed(&store, new_id).is_some(),
        "the proposal says where it belongs"
    );
    commands::create_proposal(
        &mut store,
        p,
        &new_id.to_string(),
        "> «ci rivediamo il 28 alle 10»\n\nporto io le slide\n",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::Merged);
    let file = std::fs::read_to_string(&path).unwrap();
    assert!(file.contains("start: 2026-10-28T10:00"), "{file}");
    assert!(file.contains("porto io le slide"), "{file}");
    assert!(file.contains(&format!("meeting_of: note:{host}")), "linked back: {file}");
}

#[test]
fn a_rejected_new_note_leaves_nothing_on_main() {
    if !have_git() {
        return;
    }
    let (dir, mut store, host) = vault_with_a_conversation();
    let p = dir.path();
    let draft = fm_model::Object::new(fm_model::Kind::Note, "x");
    let new_id = draft.id;
    let prop = commands::propose_new_note(
        &mut store,
        p,
        draft,
        Some(host.parse().unwrap()),
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    commands::reject_proposal(&mut store, p, &prop.id, None).unwrap();
    assert!(!p.join(format!("notes/{new_id}.md")).exists());
}
