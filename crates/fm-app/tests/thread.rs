//! Discussion as notes: one message = one file, ULID-named, in the vault of the note it is
//! about (`collaboration-design.md` ruling 11; `MASTERPLAN.md:110` names note-per-message as
//! the *safe* side of the granularity fork).
//!
//! Three things are proven here, and the second and third are the ones a previous attempt at
//! this feature got wrong:
//!
//! 1. A message round-trips and re-roots correctly.
//! 2. **Messages do not pollute the note views** — including `activity()`, which is a `git log`
//!    read-model with no filter to hang a predicate on and so was silently missed.
//! 3. **A broken pointer never hides a message**, and neither does a stray property value.

use fm_app::commands::{agenda, board, capture, recent, reply, search, set_property, thread};
use fm_core::{MemoryStore, Store};

/// A note with `n` replies posted straight to it.
fn discussion(s: &mut MemoryStore, n: usize) -> (String, Vec<String>) {
    let note = capture(s, "the note under discussion", "").unwrap().id;
    let ids = (0..n).map(|i| reply(s, &note, &format!("message {i}")).unwrap().id).collect();
    (note, ids)
}

#[test]
fn a_reply_is_one_note_carrying_its_pointers() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "the note", "").unwrap().id;

    let first = reply(&mut s, &note, "what about the second column?").unwrap();

    // Spelled `note:<ULID>` — the form `refs::strip_cross_vault` can see, which is what stops
    // `copy_note` carrying the pointer into another vault's permanent history.
    assert_eq!(first.props.get("thread_of").unwrap(), &serde_json::json!(format!("note:{note}")));
    assert_eq!(first.props.get("reply_to").unwrap(), &serde_json::json!(format!("note:{note}")));

    let t = thread(&s, &note).unwrap();
    assert_eq!(t.count, 1);
    assert_eq!(t.messages[0].body, "what about the second column?");
    assert_eq!(t.messages[0].depth, 0, "a reply to the root sits at the root");
    assert!(t.root.is_some());
}

/// The gesture a comment UI makes most naturally after "reply": reply to *that*. It must join
/// the same discussion, not start an invisible one hanging off a message no view can reach.
#[test]
fn replying_to_a_message_re_roots_to_the_note() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "the note", "").unwrap().id;
    let first = reply(&mut s, &note, "first").unwrap();

    let second = reply(&mut s, &first.id, "answering the first").unwrap();

    assert_eq!(
        second.props.get("thread_of").unwrap(),
        &serde_json::json!(format!("note:{note}")),
        "the thread is the NOTE's, not the message's"
    );
    assert_eq!(second.props.get("reply_to").unwrap(), &serde_json::json!(format!("note:{}", first.id)));

    let t = thread(&s, &note).unwrap();
    assert_eq!(t.count, 2, "both messages are in the one discussion");
    assert_eq!(t.messages[1].depth, 1, "and the answer is indented under its parent");
}

/// **The test the design doc asked for.** A busy thread must leave every planning view exactly
/// as it was — `recent()` most of all, because the editor's `/` note-picker is built on it.
#[test]
fn a_thousand_messages_change_no_note_view() {
    let mut s = MemoryStore::new();
    let a = capture(&mut s, "alpha", "").unwrap().id;
    let b = capture(&mut s, "beta", "").unwrap().id;
    set_property(&mut s, &a, "status", "todo").unwrap();
    set_property(&mut s, &b, "due", "2026-08-01").unwrap();

    let cards = |s: &MemoryStore| {
        board(s, "status").unwrap().columns.iter().map(|c| c.cards.len()).sum::<usize>()
    };
    let before = (recent(&s).unwrap().len(), agenda(&s).unwrap().len(), cards(&s));

    for i in 0..1_000 {
        reply(&mut s, &a, &format!("message {i}")).unwrap();
    }

    assert_eq!(recent(&s).unwrap().len(), before.0, "the timeline and `/` picker are unmoved");
    assert_eq!(agenda(&s).unwrap().len(), before.1, "the agenda is unmoved");
    assert_eq!(cards(&s), before.2, "no message reached a column — `(none)` least of all");
}

/// The `/` note-picker is `recent()`, so this is the daily gesture the exclusion protects.
/// Asserted by identity, not by count.
#[test]
fn the_note_picker_still_lists_exactly_the_real_notes() {
    let mut s = MemoryStore::new();
    let (note, _) = discussion(&mut s, 50);

    let listed: Vec<String> = recent(&s).unwrap().into_iter().map(|m| m.id).collect();

    assert_eq!(listed, vec![note], "only the note itself is offerable as a link target");
}

/// Search is deliberately unfiltered: discussion you cannot find in five years defeats the
/// reason for keeping it in the vault rather than in a chat app.
#[test]
fn discussion_is_still_searchable() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "the note", "").unwrap().id;
    reply(&mut s, &note, "the fluorescence baseline drifted on Tuesday").unwrap();

    assert_eq!(search(&s, "fluorescence").unwrap().len(), 1);
}

/// A vault is an audience, so a reply joins the vault of the note it is about — never the
/// caller's choice.
#[test]
fn a_message_joins_the_vault_of_the_note_it_is_about() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "a lab note", "lab").unwrap().id;

    assert_eq!(reply(&mut s, &note, "a lab reply").unwrap().vault, "lab");
}

/// `reply_to` is hand-editable frontmatter, so it can dangle or cycle. Neither may cost a
/// message its visibility — an unreachable note is the failure mode this project refuses.
#[test]
fn a_dangling_or_circular_reply_to_never_hides_a_message() {
    let mut s = MemoryStore::new();
    let (note, ids) = discussion(&mut s, 3);

    // The parent is deleted out from under two of them...
    s.delete(ids[0].parse().unwrap()).unwrap();

    let t = thread(&s, &note).unwrap();
    assert_eq!(t.count, 2, "every surviving message is shown exactly once");
    assert!(t.messages.iter().all(|m| m.depth == 0), "an unresolvable parent costs indent only");
}

/// Discussion structure is not settable by hand — which is what stops a board grouped by
/// `thread_of` from erasing a note on a single drag.
#[test]
fn thread_structure_cannot_be_written_by_set_property() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "an ordinary note", "").unwrap().id;

    for key in ["thread_of", "reply_to"] {
        assert!(set_property(&mut s, &note, key, "doing").is_err(), "{key} must be refused");
    }
    assert_eq!(recent(&s).unwrap().len(), 1, "and the note is still visible everywhere");
}

/// Even if a stray value did reach the property, it must not read as a pointer — the views
/// hide a note only when it genuinely names a discussion.
#[test]
fn a_non_reference_value_never_hides_a_note() {
    use fm_model::{Kind, Object, PropertyValue};
    let mut s = MemoryStore::new();
    let mut n = Object::new(Kind::Note, "a note about thread count in textiles");
    n.extra.insert("thread_of".into(), PropertyValue::Text("40".into()));
    s.put(&n).unwrap();

    assert_eq!(recent(&s).unwrap().len(), 1, "a word in the property is not a discussion");
}

#[test]
fn a_reply_refuses_a_bad_target_or_an_empty_body() {
    let mut s = MemoryStore::new();
    let note = capture(&mut s, "the note", "").unwrap().id;

    assert!(reply(&mut s, "01ARZ3NDEKTSV4RRFFQ69G5FAV", "hi").is_err(), "unknown target");
    assert!(reply(&mut s, "not-an-id", "hi").is_err(), "unparseable target");
    assert!(reply(&mut s, &note, "   ").is_err(), "an empty message is a mistake, not a post");
    assert!(thread(&s, &note).unwrap().messages.is_empty(), "and none of them wrote a file");
}

/// A deleted note must not swallow its discussion.
#[test]
fn a_deleted_root_leaves_its_messages_readable() {
    let mut s = MemoryStore::new();
    let (note, _) = discussion(&mut s, 2);

    s.delete(note.parse().unwrap()).unwrap();

    let t = thread(&s, &note).unwrap();
    assert!(t.root.is_none(), "the subject is gone");
    assert_eq!(t.count, 2, "the reasoning about it is not");
}

/// **`activity()` is the surface a `Predicate` cannot reach**, and so the one a filter-shaped
/// fix silently misses — which is exactly what happened on the first attempt at this feature.
/// It is a `git log` read-model: it walks touched paths and resolves each through `Store::get`,
/// with no `Filter` anywhere. Every reply is a new file and therefore a git touch, so without a
/// hand-applied guard a busy thread floods the recent-edits feed, the contributor filter and
/// the `EditedBy` labels.
///
/// Real git, real commits — the thing under test is a contract with `git log`, not a function.
#[test]
fn a_thread_does_not_flood_the_activity_feed() {
    use fm_core::{FileStore, Store as _};
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    fm_core::git::ensure_repo(dir.path()).unwrap();
    fm_core::git::set_identity(dir.path(), "Tester", "t@example.org").unwrap();

    let note = capture(&mut store, "the note under discussion", "").unwrap().id;
    for i in 0..5 {
        reply(&mut store, &note, &format!("message {i}")).unwrap();
    }
    let paths = store.written();
    assert!(fm_core::git::commit_all(dir.path(), "auto: seed", &paths).unwrap());

    let events = fm_app::commands::activity(&store, dir.path(), "1 year ago").unwrap();

    assert_eq!(events.len(), 1, "six files were touched; only the note is an edit worth showing");
    assert_eq!(events[0].id, note);
}
