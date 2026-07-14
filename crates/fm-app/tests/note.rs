//! S5c: the read view's payload. `get` returns a single note *with its body* —
//! the list commands are meta-only, and the body crosses IPC only when a note is
//! opened. The body is returned verbatim (the read view is a pure view layer;
//! it never mutates the bytes), which is the frontend half of the byte
//! round-trip invariant.

use fm_app::commands::{capture, get, update_body};
use fm_core::{FileStore, MemoryStore};
use tempfile::tempdir;

#[test]
fn get_returns_the_full_body_verbatim() {
    let mut s = MemoryStore::new();
    let body = "The GAE lambda interacts badly. $\\lambda = 0.95$.\n\n```mermaid\ngraph TD;A-->B;\n```\n";
    let id = capture(&mut s, body).unwrap().id;

    let note = get(&s, &id).unwrap().expect("note exists");
    assert_eq!(note.body, body, "body is returned byte-for-byte");
    assert_eq!(note.meta.id, id);
    assert_eq!(note.meta.kind, "note");
}

#[test]
fn get_is_none_for_a_missing_note() {
    let s = MemoryStore::new();
    // A well-formed 26-char ULID (all zeros) that was never stored.
    assert!(get(&s, "00000000000000000000000000").unwrap().is_none());
}

#[test]
fn get_rejects_a_malformed_id() {
    let s = MemoryStore::new();
    assert!(get(&s, "not-a-ulid").is_err());
}

#[test]
fn edit_body_round_trips_byte_for_byte_through_disk() {
    // The central files-as-truth invariant: edit -> store -> read === bytes, even
    // for content that stresses the frontmatter parser (a `---` fence line inside
    // the body, unicode, math, a trailing newline).
    let dir = tempdir().unwrap();
    let tricky = "Edited.\n\n---\n\n$\\lambda = 0.95$\n\ncafé ☕\n";
    let id = {
        let mut s = FileStore::open(dir.path()).unwrap();
        let id = capture(&mut s, "original body").unwrap().id;
        update_body(&mut s, &id, tricky).unwrap();
        id
    };
    // Re-open (index rebuilt from the .md file) and confirm the body is verbatim.
    let s2 = FileStore::open(dir.path()).unwrap();
    let note = get(&s2, &id).unwrap().unwrap();
    assert_eq!(note.body, tricky, "the edited body round-trips exactly");
}
