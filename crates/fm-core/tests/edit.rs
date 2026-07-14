//! S2: properties are editable — a changed property is written back to the
//! Markdown file and survives reload; and (the load-bearing part) editing a note
//! never drops custom frontmatter a user added by hand. That last guarantee is
//! also the precondition for "board by ANY property" in S3.

use fm_core::{frontmatter, FileStore, Store};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::Query;
use std::fs;
use tempfile::tempdir;
use time::macros::{date, datetime};

#[test]
fn set_property_is_written_to_disk_and_survives_reload() {
    let dir = tempdir().unwrap();
    let id = {
        let mut s = FileStore::open(dir.path()).unwrap();
        let mut o = Object::new(Kind::Task, "ship S2");
        let id = o.id;
        s.put(&o).unwrap();

        // Edit: set status + due, stamp updated (what `fm set` does).
        o.status = Some("doing".into());
        o.due = Some(date!(2026 - 08 - 01));
        o.updated = datetime!(2026-07-20 9:00 UTC);
        s.put(&o).unwrap();
        id
    };

    // The raw file reflects the edit...
    let raw = fs::read_to_string(dir.path().join(format!("notes/{id}.md"))).unwrap();
    assert!(raw.contains("status: doing"), "frontmatter carries the new status:\n{raw}");
    assert!(raw.contains("due: 2026-08-01"), "and the due date:\n{raw}");

    // ...and a fresh store (reindex from files) reads it back.
    let s2 = FileStore::open(dir.path()).unwrap();
    let got = s2.get(id).unwrap().unwrap();
    assert_eq!(got.status.as_deref(), Some("doing"));
    assert_eq!(got.due, Some(date!(2026 - 08 - 01)));
    assert!(got.updated > got.created, "editing bumped updated past created");
}

#[test]
fn editing_a_note_preserves_hand_added_custom_properties() {
    // The data-integrity guarantee: a property fm doesn't know about (added in
    // Vim) must not vanish when fm rewrites the file after an unrelated edit.
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();

    // Hand-author a note carrying a custom `project` property.
    let mut hand = Object::new(Kind::Note, "external note");
    hand.extra.insert("project".into(), PropertyValue::Text("alpha".into()));
    let id = hand.id;
    fs::write(notes.join(format!("{id}.md")), frontmatter::to_file(&hand).unwrap()).unwrap();

    // fm loads it, edits an unrelated property, writes back.
    let mut s = FileStore::open(dir.path()).unwrap();
    let mut got = s.get(id).unwrap().unwrap();
    assert_eq!(got.get("project"), PropertyValue::Text("alpha".into()));
    got.status = Some("doing".into());
    s.put(&got).unwrap();

    // The custom property is still there after the edit.
    let reread = FileStore::open(dir.path()).unwrap().get(id).unwrap().unwrap();
    assert_eq!(reread.get("project"), PropertyValue::Text("alpha".into()));
    assert_eq!(reread.status.as_deref(), Some("doing"));
}

#[test]
fn custom_property_is_groupable_like_any_other() {
    // Proves extra properties flow through get() into the generic query engine —
    // the precondition for grouping a board by a user-invented property in S3.
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();
    for (body, proj) in [("a", "alpha"), ("b", "alpha"), ("c", "beta")] {
        let mut o = Object::new(Kind::Note, body);
        o.extra.insert("project".into(), PropertyValue::Text(proj.into()));
        s.put(&o).unwrap();
    }

    let q = Query { group_by: Some("project".into()), ..Default::default() };
    let groups = s.query(&q).unwrap().groups.unwrap();

    let mut labels: Vec<_> = groups.iter().map(|g| g.label.clone()).collect();
    labels.sort();
    assert_eq!(labels, vec!["alpha", "beta"]);
    let alpha = groups.iter().find(|g| g.label == "alpha").unwrap();
    assert_eq!(alpha.rows.len(), 2);
}
