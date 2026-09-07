//! S6: `verify` is report-only integrity checking. These tests prove the three
//! things that make "lasts ten years" real: it flags unparseable notes (error),
//! flags a referenced-but-absent blob (warning, not error — it may be unsynced),
//! and — the hard one — catches silent bit-rot by re-hashing blobs against their
//! content address. The manifest round-trips and lets a scrub spot a blob that
//! vanished or appeared.

use fm_core::blob::BlobStore;
use fm_core::{verify, FileStore, Manifest, Store};
use fm_model::{Kind, Object};
use std::fs;
use tempfile::tempdir;

fn seed_asset(vault: &std::path::Path, bytes: &[u8]) -> String {
    let src = vault.join("incoming.bin");
    fs::write(&src, bytes).unwrap();
    let stored = BlobStore::new(vault).put_file(&src).unwrap();
    fs::remove_file(&src).unwrap();
    stored.hash
}

#[test]
fn a_clean_vault_verifies_with_no_errors() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let hash = seed_asset(dir.path(), b"a figure");
    let mut obj = Object::new(Kind::Asset, "fig");
    obj.assets = vec![format!("sha256:{hash}")];
    store.put(&obj).unwrap();

    let report = verify(dir.path(), true).unwrap();
    assert!(report.ok(), "no errors: {:?}", report.issues);
    assert_eq!(report.blobs, 1);
    assert!(report.notes >= 1);
}

#[test]
fn unparseable_frontmatter_is_an_error() {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("broken.md"), "no frontmatter fence here\n").unwrap();

    let report = verify(dir.path(), false).unwrap();
    assert!(!report.ok());
    assert_eq!(report.errors(), 1);
    assert!(report.issues[0].message.contains("frontmatter"));
}

#[test]
fn a_missing_referenced_blob_is_a_warning_not_an_error() {
    let dir = tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let mut obj = Object::new(Kind::Asset, "fig");
    // Reference a blob that was never stored (e.g. not synced to this machine).
    obj.assets =
        vec!["sha256:0000000000000000000000000000000000000000000000000000000000000000".into()];
    store.put(&obj).unwrap();

    let report = verify(dir.path(), false).unwrap();
    assert!(report.ok(), "a missing blob must not fail verify");
    assert_eq!(report.warnings(), 1);
}

#[test]
fn scrub_catches_bit_rot() {
    let dir = tempdir().unwrap();
    let hash = seed_asset(dir.path(), b"original bytes");

    // Silently corrupt the stored blob without changing its name (the hash).
    let blob = BlobStore::new(dir.path()).path_for(&hash);
    fs::write(&blob, b"corrupted bytes!").unwrap();

    // A plain verify can't see it; a scrub re-hashes and does.
    assert!(verify(dir.path(), false).unwrap().ok());
    let scrub = verify(dir.path(), true).unwrap();
    assert!(!scrub.ok(), "scrub must catch bit-rot");
    assert!(scrub.issues.iter().any(|i| i.message.contains("BIT ROT")));
}

#[test]
fn manifest_round_trips_and_scrub_flags_a_vanished_blob() {
    let dir = tempdir().unwrap();
    let hash = seed_asset(dir.path(), b"keep me");
    let built = Manifest::build(dir.path()).unwrap();
    built.write(dir.path()).unwrap();

    let read = Manifest::read(dir.path()).unwrap().unwrap();
    assert_eq!(built, read);
    assert!(read.blobs.contains_key(&hash));

    // Delete the blob after it was recorded → scrub reports a recorded-but-gone.
    fs::remove_file(BlobStore::new(dir.path()).path_for(&hash)).unwrap();
    let report = verify(dir.path(), true).unwrap();
    assert!(!report.ok());
    assert!(report.issues.iter().any(|i| i.message.contains("missing from the store")));
}

/// **A vault whose notes live somewhere else must still be checked.** `verify` used to look in
/// a hardcoded `notes/`, so a Track-V vault (`vault.json` → `notes: docs`) had that directory
/// simply not exist: the loop never ran and `verify` reported "0 notes, 0 errors" and exited 0.
/// A vault full of unparseable notes passed clean, silently — the exact inversion of what an
/// integrity checker is for. `backup` had always asked the descriptor; this is the drift.
#[test]
fn a_vault_with_a_custom_notes_dir_is_actually_checked() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/broken.md"), "no frontmatter fence here\n").unwrap();

    let report = verify(dir.path(), false).unwrap();

    assert_eq!(report.notes, 1, "the note was found where the vault says it lives");
    assert!(!report.ok(), "and its breakage is reported, not silently passed over");
}
