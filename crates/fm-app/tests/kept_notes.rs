//! **The list of notes a merge brought back, and the two ways a row leaves it.**
//!
//! `outstanding.md` §2.12 asked for a surface that outlives one sync run, because a resurrection is
//! a decision the app made on the user's behalf and a dismissible banner should not be the end of
//! it. The git half — reading the paths back out of the merge commits — is pinned two-device and
//! cross-backend in `fm-cli/tests/conflict_resolution_both_devices.rs`. This is the app half: what
//! happens between those paths and a panel.
//!
//! The interesting assertions are the joins, because each of them is how a row *terminates*:
//! deleting the note again drops it permanently with nothing to record, and a kept path that is not
//! a note is reported honestly rather than dressed up as one.

use fm_app::commands;
use fm_core::{MemoryStore, Store};
use fm_model::{Kind, Object};

/// A store holding one note in vault `home`, and its id.
///
/// `vault` is set explicitly because `MemoryStore` stores an object exactly as handed to it — a
/// `FileStore` stamps it on the way out, which is what the dispatch arm reads. Setting it here is
/// what makes the cross-vault test below mean anything.
fn with_note(id: &str, title: &str) -> (MemoryStore, String) {
    let mut store = MemoryStore::new();
    let mut o = Object::new(Kind::Note, "body");
    o.id = id.parse().unwrap();
    o.title = Some(title.to_string());
    o.vault = "home".into();
    store.put(&o).unwrap();
    (store, id.to_string())
}

#[test]
fn a_kept_note_is_reported_with_the_title_a_person_would_recognise() {
    let (store, id) = with_note("01JQ0000000000000000000000", "the note they deleted");

    let rows = commands::kept_notes(&store, "home", &[format!("notes/{id}.md")]).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id.as_deref(), Some(id.as_str()));
    assert_eq!(rows[0].title.as_deref(), Some("the note they deleted"));
    assert_eq!(rows[0].vault, "home");
}

#[test]
fn deleting_it_again_drops_it_from_the_list_with_nothing_to_record() {
    // **The terminator that needs no state.** The merge commit still names the path — history is
    // append-only — but the note is gone from the store, so the row is gone too. That is why the
    // panel's "delete it again" is the ordinary `delete` command and not a new write path: the
    // absence of the note *is* the record of the answer, on every device, permanently.
    let (mut store, id) =
        with_note("01JQ0000000000000000000001", "brought back and sent away again");
    let path = format!("notes/{id}.md");
    assert_eq!(commands::kept_notes(&store, "home", std::slice::from_ref(&path)).unwrap().len(), 1);

    store.delete(id.parse().unwrap()).unwrap();

    assert!(
        commands::kept_notes(&store, "home", &[path]).unwrap().is_empty(),
        "a note deleted again must not keep asking"
    );
}

#[test]
fn a_kept_file_that_is_not_a_note_is_named_as_a_file_rather_than_invented_as_one() {
    // The keep branch settles EVERY delete/modify path in the repo, not only Markdown under
    // `notes/` — a saved view or the attachment manifest can be resurrected the same way. Reporting
    // those as notes would put a "delete it again" button on a file the `delete` command does not
    // handle; dropping them silently would make the persistent surface cover less than the
    // per-run step line it replaces.
    let store = MemoryStore::new();

    let rows =
        commands::kept_notes(&store, "home", &["manifest.json".into(), "views/by-tag.view".into()])
            .unwrap();

    assert_eq!(rows.len(), 2);
    for r in &rows {
        assert!(r.id.is_none(), "{} is not a note and must not claim an id", r.path);
        assert!(r.title.is_none());
    }
}

#[test]
fn a_nested_path_under_notes_is_not_mistaken_for_a_note_id() {
    // `notes/archive/x.md` has a stem containing a slash. The same guard `note_id_from_path`
    // carries in the git layer, restated here because this is a second parser of the same shape
    // and the two must not disagree about what a note path is.
    let store = MemoryStore::new();
    let rows = commands::kept_notes(&store, "home", &["notes/archive/x.md".into()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].id.is_none());
}

#[test]
fn a_note_from_another_vault_is_not_borrowed_into_this_one() {
    // The paths come from one vault's history and the store is asked for that vault's notes. A
    // match on id alone would let two vaults holding the same id cross-report — and vaults are
    // audiences, which is the one seam this codebase does not let leak.
    let (store, id) = with_note("01JQ0000000000000000000002", "mine");

    let rows = commands::kept_notes(&store, "not-this-vault", &[format!("notes/{id}.md")]).unwrap();

    assert!(rows.is_empty(), "a note is only reported for the vault whose history named it");
}

#[test]
fn nothing_kept_asks_the_store_nothing() {
    let store = MemoryStore::new();
    assert!(commands::kept_notes(&store, "home", &[]).unwrap().is_empty());
}
