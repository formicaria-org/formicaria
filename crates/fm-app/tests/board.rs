//! S3: the board is a query + a generic renderer. These tests prove the whole
//! thesis without a webview: group by *any* property (a user-invented key needs
//! zero new code — the falsifiability test the MASTERPLAN calls for); a drop
//! writes the column's value back to disk exactly like `fm set`; and only notes
//! get cards. MemoryStore covers the logic; one FileStore test proves the
//! write-back actually hits the file and survives a reindex.

use fm_app::commands::{board, capture, set_property};
use fm_core::{FileStore, MemoryStore, Store};
use std::collections::HashSet;
use tempfile::tempdir;

#[test]
fn board_groups_by_status_into_columns() {
    let mut s = MemoryStore::new();
    let a = capture(&mut s, "alpha", "").unwrap().id;
    let b = capture(&mut s, "beta", "").unwrap().id;
    let c = capture(&mut s, "gamma", "").unwrap().id;
    set_property(&mut s, &a, "status", "todo").unwrap();
    set_property(&mut s, &b, "status", "doing").unwrap();
    set_property(&mut s, &c, "status", "todo").unwrap();

    let b = board(&s, "status").unwrap();
    assert_eq!(b.group_by, "status");
    let todo = b.columns.iter().find(|c| c.label == "todo").unwrap();
    let doing = b.columns.iter().find(|c| c.label == "doing").unwrap();
    assert_eq!(todo.cards.len(), 2);
    assert_eq!(doing.cards.len(), 1);
    // The column echoes the settable string back; a drop calls set_property with it.
    assert_eq!(doing.value, "doing");
}

#[test]
fn the_board_shows_notes_only_never_assets() {
    // An asset is a blob a note *references*, not something you plan, so it has
    // no card. Grouping by `type` is the sharpest way to ask: even pointed
    // straight at the kind, the board can only ever answer "note".
    let mut s = MemoryStore::new();
    let _n = capture(&mut s, "a note", "").unwrap().id;
    let a = capture(&mut s, "an ingested file", "").unwrap().id;
    set_property(&mut s, &a, "type", "asset").unwrap();

    let b = board(&s, "type").unwrap();
    let labels: HashSet<&str> = b.columns.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains("note"), "got {labels:?}");
    assert!(!labels.contains("asset"), "assets are excluded from the board, got {labels:?}");

    // The asset is gone from every column, not merely from its own.
    let carded: HashSet<&str> =
        b.columns.iter().flat_map(|c| c.cards.iter()).map(|m| m.id.as_str()).collect();
    assert!(!carded.contains(a.as_str()), "the asset still has a card");
}

#[test]
fn drag_write_back_hits_disk_and_survives_reindex() {
    let dir = tempdir().unwrap();
    let id = {
        let mut s = FileStore::open(dir.path()).unwrap();
        let id = capture(&mut s, "trust region clipping", "").unwrap().id;
        set_property(&mut s, &id, "status", "todo").unwrap();
        // The drag: drop the card into the "doing" column.
        set_property(&mut s, &id, "status", "doing").unwrap();
        id
    };

    // The value is on disk in the Markdown file, not just in an index.
    let raw = std::fs::read_to_string(dir.path().join(format!("notes/{id}.md"))).unwrap();
    assert!(raw.contains("status: doing"), "drag write-back hit disk:\n{raw}");

    // A fresh store (index rebuilt from files) places the card in the new column.
    let s2 = FileStore::open(dir.path()).unwrap();
    let b = board(&s2, "status").unwrap();
    let doing = b.columns.iter().find(|c| c.label == "doing").unwrap();
    assert_eq!(doing.cards.len(), 1);
    assert_eq!(doing.cards[0].id, id);
    assert!(b.columns.iter().all(|c| c.label != "todo"), "the card left the todo column");
}

#[test]
fn dropping_into_the_none_column_clears_the_property() {
    let mut s = MemoryStore::new();
    let id = capture(&mut s, "x", "").unwrap().id;
    set_property(&mut s, &id, "status", "doing").unwrap();
    // The "(none)" column carries an empty value; dropping there clears status.
    set_property(&mut s, &id, "status", "").unwrap();

    let obj = s.get(id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(obj.status, None);

    let b = board(&s, "status").unwrap();
    let none = b.columns.iter().find(|c| c.value.is_empty()).unwrap();
    assert_eq!(none.label, "(none)");
    assert_eq!(none.cards.len(), 1);
}

#[test]
fn board_by_a_custom_property_works_and_props_flow_through() {
    // "Board by ANY property" — a user-invented key groups with no code change,
    // and the same key is visible to the card via the open `props` map.
    let mut s = MemoryStore::new();
    let id = capture(&mut s, "x", "").unwrap().id;
    set_property(&mut s, &id, "project", "alpha").unwrap();

    let b = board(&s, "project").unwrap();
    let alpha = b.columns.iter().find(|c| c.label == "alpha").unwrap();
    assert_eq!(alpha.cards.len(), 1);
    assert_eq!(alpha.cards[0].props.get("project").unwrap(), &serde_json::json!("alpha"));
}
