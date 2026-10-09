//! *A deleted note is never more than a list away* (`decisions.md` 2026-10-09), end to end over a real
//! git vault: the session undo (`delete_keeping` → `restore_note`) and *Recently deleted*
//! (`vcs::deleted_notes`), and the property undo's previous value.

use fm_app::commands;
use fm_core::{git, vcs, FileStore, Store};
use fm_model::Id;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn vault() -> (tempfile::TempDir, FileStore, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let n =
        commands::capture(&mut store, "# Lab meeting\n\nRoom 3, bring the plots.\n", "").unwrap();
    commands::set_property(&mut store, &n.id, "tags", "meeting, lab").unwrap();
    git::commit_all(dir.path(), "first", &store.written()).unwrap();
    (dir, store, n.id)
}

#[test]
fn a_deleted_note_comes_back_as_it_was_and_only_once() {
    let (dir, mut store, id) = vault();
    let gone = commands::delete_keeping(&mut store, &id).unwrap();
    let parsed: Id = id.parse().unwrap();
    assert!(store.get(parsed).unwrap().is_none());

    commands::restore_note(&mut store, &gone.vault, &gone.file).unwrap();
    let back = store.get(parsed).unwrap().expect("it is back");
    assert!(back.body.contains("bring the plots"));
    assert_eq!(back.tags, vec!["meeting".to_string(), "lab".into()]);

    let again = commands::restore_note(&mut store, &gone.vault, &gone.file).unwrap_err();
    assert!(again.to_string().contains("already back"), "{again}");
    drop(dir);
}

#[test]
fn recently_deleted_finds_a_deletion_git_recorded_with_its_text() {
    if !have_git() {
        return;
    }
    let (dir, mut store, id) = vault();
    commands::delete_keeping(&mut store, &id).unwrap();
    git::commit_all(dir.path(), "auto: delete", &store.written()).unwrap();

    let found = vcs::deleted_notes(dir.path(), 30).unwrap();
    let d = found.iter().find(|d| d.id == id).expect("listed");
    assert_eq!(commands::deleted_title(&d.file), "Lab meeting");
    commands::restore_note(&mut store, "", &d.file).unwrap();
    assert!(store.get(id.parse().unwrap()).unwrap().is_some());
}

#[cfg(feature = "native-git")]
#[test]
fn the_phones_git_finds_the_same_deletion() {
    if !have_git() {
        return;
    }
    let (dir, mut store, id) = vault();
    commands::delete_keeping(&mut store, &id).unwrap();
    git::commit_all(dir.path(), "auto: delete", &store.written()).unwrap();
    let desktop = git::deleted_notes(dir.path(), 30).unwrap();
    let phone = fm_core::git_native::deleted_notes(dir.path(), 30).unwrap();
    assert_eq!(desktop.len(), 1);
    assert_eq!(phone.len(), 1);
    assert_eq!(
        (&phone[0].id, &phone[0].commit, &phone[0].file),
        (&desktop[0].id, &desktop[0].commit, &desktop[0].file)
    );
}

#[test]
fn a_property_change_answers_the_value_it_replaced() {
    let (_dir, mut store, id) = vault();
    let prev = commands::set_property_answering(&mut store, &id, "tags", "meeting").unwrap();
    assert_eq!(prev.as_deref(), Some("meeting, lab"));
    // Setting it back with that answer restores exactly what was there.
    commands::set_property(&mut store, &id, "tags", prev.as_deref().unwrap()).unwrap();
    let o = store.get(id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(o.tags, vec!["meeting".to_string(), "lab".into()]);
    let none =
        commands::set_property_answering(&mut store, &id, "due", "2026-10-16T12:00").unwrap();
    assert_eq!(none, None, "there was no date before");
}
