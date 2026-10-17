//! **A write that changes nothing is not made** (2026-10-17, `decisions.md#sync`).
//!
//! Opening a note and leaving it re-saved the same text with a new `updated:` stamp. History then
//! carried a change nobody made, and the owner's other device was "behind" on a note neither of
//! them had touched — the laptop sent the owner's own note back to the phone with only the stamp
//! moved. Each test reads the file's bytes before and after, because "nothing happened" is a claim
//! about the disk, not about a return value.

use fm_app::{commands, log};
use fm_core::FileStore;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::tempdir;

fn vault_with_a_note(body: &str) -> (FileStore, tempfile::TempDir, String) {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let meta = commands::capture(&mut store, body, "").unwrap();
    (store, dir, meta.id)
}

fn note_file(dir: &tempfile::TempDir) -> PathBuf {
    std::fs::read_dir(dir.path().join("notes"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("the note file")
}

/// The file's bytes, after a pause long enough that any write would stamp a different `updated`.
fn bytes_after_a_moment(dir: &tempfile::TempDir) -> Vec<u8> {
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::read(note_file(dir)).unwrap()
}

#[test]
fn saving_the_same_text_leaves_the_file_alone() {
    let (mut store, dir, id) = vault_with_a_note("Books\n\n- a canticle for leibowitz\n");
    let note = commands::get(&store, &id).unwrap().unwrap();
    let before = bytes_after_a_moment(&dir);

    let version = commands::update_body(&mut store, &id, &note.body, &note.version).unwrap();

    assert_eq!(std::fs::read(note_file(&dir)).unwrap(), before, "an unchanged body was rewritten");
    assert_eq!(version, note.version, "the version handed back must still be the body's");
}

#[test]
fn a_stale_pane_holding_exactly_the_disk_text_is_not_refused() {
    // A pane whose `base` is out of date but whose text equals the file has nothing to overwrite:
    // refusing it would only put a conflict in front of someone who changed nothing.
    let (mut store, dir, id) = vault_with_a_note("same\n");
    let note = commands::get(&store, &id).unwrap().unwrap();
    let before = bytes_after_a_moment(&dir);

    commands::update_body(&mut store, &id, &note.body, "a-version-from-long-ago").unwrap();

    assert_eq!(std::fs::read(note_file(&dir)).unwrap(), before);
}

#[test]
fn a_real_edit_is_still_written_and_still_guarded() {
    let (mut store, dir, id) = vault_with_a_note("first\n");
    let note = commands::get(&store, &id).unwrap().unwrap();
    let before = bytes_after_a_moment(&dir);

    commands::update_body(&mut store, &id, "first, edited\n", &note.version).unwrap();
    assert_ne!(std::fs::read(note_file(&dir)).unwrap(), before, "a real edit must be written");
    assert!(
        commands::update_body(&mut store, &id, "from a stale pane\n", &note.version).is_err(),
        "a different text from a stale pane must still be refused"
    );
}

#[test]
fn setting_a_property_to_the_value_it_has_leaves_the_file_alone() {
    let (mut store, dir, id) = vault_with_a_note("a task\n");
    commands::set_property(&mut store, &id, "status", "doing").unwrap();
    let before = bytes_after_a_moment(&dir);

    commands::set_property(&mut store, &id, "status", "doing").unwrap();
    assert_eq!(
        std::fs::read(note_file(&dir)).unwrap(),
        before,
        "an unchanged property was rewritten"
    );

    commands::set_property(&mut store, &id, "status", "done").unwrap();
    assert_ne!(
        std::fs::read(note_file(&dir)).unwrap(),
        before,
        "a changed property must be written"
    );
}

#[test]
fn setting_several_properties_to_what_they_are_leaves_the_file_alone() {
    let (mut store, dir, id) = vault_with_a_note("a task\n");
    let props = BTreeMap::from([("status".to_string(), json!("doing"))]);
    log::set_properties(&mut store, &id, &props).unwrap();
    let before = bytes_after_a_moment(&dir);

    let previous = log::set_properties(&mut store, &id, &props).unwrap();
    assert_eq!(std::fs::read(note_file(&dir)).unwrap(), before);
    // What was replaced is still answered, so undo has something to put back.
    assert_eq!(previous.get("status"), Some(&Some("doing".to_string())));
}
