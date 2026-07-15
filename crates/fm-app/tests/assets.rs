//! Asset resolution + ingest command functions — the pure core the HTTP layer
//! calls. Hermetic: a tempdir vault (hand-placed blobs, or a real ingest of
//! magic bytes), so no external tooling is needed.

use fm_app::commands;
use fm_core::MemoryStore;
use std::fs;
use tempfile::tempdir;

// A real 64-hex sha256 (of the bytes "abc"), so BlobStore's ab/cd fan-out works.
const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn seed(vault: &std::path::Path) {
    let blob_dir = vault.join("blobs/sha256").join(&HASH[0..2]).join(&HASH[2..4]);
    fs::create_dir_all(&blob_dir).unwrap();
    fs::write(blob_dir.join(HASH), b"blob-bytes").unwrap();

    let thumb_dir = vault.join("derived").join(HASH);
    fs::create_dir_all(&thumb_dir).unwrap();
    fs::write(thumb_dir.join("thumb.webp"), b"thumb-bytes").unwrap();
}

#[test]
fn resolves_blob_and_thumb_from_every_reference_spelling() {
    let vault = tempdir().unwrap();
    seed(vault.path());

    // Stored form, Markdown/mock form, and a bare hash all normalize to one blob.
    for reference in [format!("sha256:{HASH}"), format!("asset:sha256-{HASH}"), HASH.to_string()] {
        let full = commands::resolve_asset_bytes(vault.path(), &reference, "full").unwrap();
        assert_eq!(full, b"blob-bytes", "full resolves the blob for `{reference}`");
        let thumb = commands::resolve_asset_bytes(vault.path(), &reference, "thumb").unwrap();
        assert_eq!(thumb, b"thumb-bytes", "thumb resolves the preview for `{reference}`");
    }

    let status = commands::asset_status(vault.path(), &format!("sha256:{HASH}")).unwrap();
    assert!(status.has_blob && status.has_thumb);
}

#[test]
fn missing_asset_is_a_warning_not_a_crash() {
    let vault = tempdir().unwrap(); // nothing seeded
    let reference = format!("sha256:{HASH}");

    let status = commands::asset_status(vault.path(), &reference).unwrap();
    assert!(!status.has_blob, "absence is reported, not an error");
    assert!(!status.has_thumb);
    // Reading the bytes of an absent blob is an Err the UI degrades to the
    // inline "not available" placeholder.
    assert!(commands::resolve_asset_bytes(vault.path(), &reference, "full").is_err());
}

#[test]
fn a_non_asset_reference_is_rejected() {
    let vault = tempdir().unwrap();
    assert!(commands::resolve_asset_bytes(vault.path(), "not-a-hash!", "full").is_err());
    assert!(commands::asset_status(vault.path(), "xy").is_err(), "too short to be a hash");
}

#[test]
fn ingest_creates_an_asset_note_and_status_reports_its_mime() {
    let vault = tempdir().unwrap();
    let mut store = MemoryStore::default();
    // PNG magic bytes — sniffed to image/png regardless of the filename.
    let png: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let meta = commands::ingest(&mut store, vault.path(), "poster v2.png", png).unwrap();
    assert_eq!(meta.kind, "asset");
    assert_eq!(meta.title.as_deref(), Some("poster v2.png"));
    let reference = &meta.assets[0];
    assert!(reference.starts_with("sha256:"), "reference is a blob pointer: {reference}");

    let status = commands::asset_status(vault.path(), reference).unwrap();
    assert!(status.has_blob, "the ingested blob is present");
    assert_eq!(status.mime.as_deref(), Some("image/png"));

    // The blob dedups but a second ingest still makes its own asset note.
    let meta2 = commands::ingest(&mut store, vault.path(), "again.png", png).unwrap();
    assert_eq!(meta2.assets[0], *reference, "same bytes → same blob reference");
    assert_ne!(meta2.id, meta.id, "but a distinct note");
}
