//! S4: the gallery is a second renderer over the *same* query layer — the
//! checkpoint that proves the seam is real. There is no gallery-specific query
//! path: it is `type = asset` through the identical engine that answers the
//! board. These tests hold against MemoryStore and, unchanged, FileStore.

use fm_app::commands::{capture, gallery, set_property};
use fm_core::{FileStore, MemoryStore, Store};
use tempfile::tempdir;

fn asset(store: &mut dyn Store, name: &str) -> String {
    let id = capture(store, name).unwrap().id;
    set_property(store, &id, "type", "asset").unwrap();
    id
}

#[test]
fn gallery_returns_only_assets_newest_first() {
    let mut s = MemoryStore::new();
    // A mix of kinds; only the assets should surface in the gallery.
    let _note = capture(&mut s, "a plain note").unwrap();
    let first = asset(&mut s, "figure_1.pdf");
    let second = asset(&mut s, "slides.key");

    let cards = gallery(&s).unwrap();
    assert_eq!(cards.len(), 2, "only the two assets");
    assert!(cards.iter().all(|c| c.kind == "asset"));
    // Newest first — the same sort the board uses.
    assert_eq!(cards[0].id, second);
    assert_eq!(cards[1].id, first);
}

#[test]
fn gallery_is_empty_when_there_are_no_assets() {
    let mut s = MemoryStore::new();
    capture(&mut s, "just a note").unwrap();
    assert!(gallery(&s).unwrap().is_empty());
}

#[test]
fn gallery_reads_assets_from_disk_like_the_board() {
    let dir = tempdir().unwrap();
    let id = {
        let mut s = FileStore::open(dir.path()).unwrap();
        asset(&mut s, "poster.png")
    };
    // A fresh store rebuilt from files still sees the asset in the gallery.
    let s2 = FileStore::open(dir.path()).unwrap();
    let cards = gallery(&s2).unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].id, id);
    assert_eq!(cards[0].kind, "asset");
}
