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

    let meta = commands::ingest(&mut store, vault.path(), "personal", "poster v2.png", png).unwrap();
    assert_eq!(meta.kind, "asset");
    // The note joins the same audience as its bytes. Split them and the people who can
    // see the file cannot see the note describing it — and vice versa.
    assert_eq!(meta.vault, "personal", "the asset note lands in the vault its blob did");
    assert_eq!(meta.title.as_deref(), Some("poster v2.png"));
    let reference = &meta.assets[0];
    assert!(reference.starts_with("sha256:"), "reference is a blob pointer: {reference}");

    let status = commands::asset_status(vault.path(), reference).unwrap();
    assert!(status.has_blob, "the ingested blob is present");
    assert_eq!(status.mime.as_deref(), Some("image/png"));

    // The blob dedups but a second ingest still makes its own asset note.
    let meta2 = commands::ingest(&mut store, vault.path(), "personal", "again.png", png).unwrap();
    assert_eq!(meta2.assets[0], *reference, "same bytes → same blob reference");
    assert_ne!(meta2.id, meta.id, "but a distinct note");
}

/// **A thumbnail you can point a URL at.**
///
/// Thumbnails have been generated on every image ingest and readable through `resolve_asset_bytes`
/// since they were built — but the *URL* form had no way to ask for one, so every inline image
/// decoded the original. `render.ts` measures the cost: a 12 MP JPEG is ~50 MB of decoded pixels.
/// A feed of notes makes that per-screen instead of per-note, which is what forced this.
#[test]
fn a_thumbnail_can_be_asked_for_by_path() {
    let dir = tempdir().unwrap();
    seed(dir.path());
    let r = format!("sha256:{HASH}");

    let full = commands::blob_path_of_kind(dir.path(), &r, false).unwrap();
    let thumb = commands::blob_path_of_kind(dir.path(), &r, true).unwrap();
    assert_ne!(full, thumb, "asking for a thumbnail must not hand back the original");
    assert_eq!(fs::read(&thumb).unwrap(), b"thumb-bytes");
    assert_eq!(fs::read(&full).unwrap(), b"blob-bytes");
}

/// **A missing thumbnail is not a missing image.** A vault ingested before thumbnails existed, or
/// one where `vipsthumbnail` was never installed, still has to show its picture — slowly, which is
/// the right degradation for "we could not make a small copy". Returning 404 here would make the
/// feed look broken on exactly the vaults that predate it.
#[test]
fn a_missing_thumbnail_falls_back_to_the_full_blob() {
    let dir = tempdir().unwrap();
    seed(dir.path());
    fs::remove_dir_all(dir.path().join("derived")).unwrap();
    let r = format!("sha256:{HASH}");

    let got = commands::blob_path_of_kind(dir.path(), &r, true).unwrap();
    assert_eq!(fs::read(&got).unwrap(), b"blob-bytes", "it must fall back, not fail");
}

/// The blob still has to be *here* before a thumbnail is served from here. That check is what
/// `find_blob` uses to decide which vault a reference belongs to, and therefore what keeps a
/// paired device out of another audience's media — resolving a `derived/` file directly would
/// route around it.
#[test]
fn a_thumbnail_is_refused_when_the_blob_itself_is_absent() {
    let dir = tempdir().unwrap();
    let thumb_dir = dir.path().join("derived").join(HASH);
    fs::create_dir_all(&thumb_dir).unwrap();
    fs::write(thumb_dir.join("thumb.webp"), b"thumb-bytes").unwrap();

    let r = format!("sha256:{HASH}");
    assert!(
        commands::blob_path_of_kind(dir.path(), &r, true).is_err(),
        "a thumbnail must not answer for a vault that does not hold the blob"
    );
}

/// **The bytes API behaves like the path API — which it did not until 2026-09-04.**
///
/// `blob_path_of_kind` has had the fallback and the blob-first ordering all along, and the two
/// tests above pin them. `resolve_asset_bytes` — the arm `resolve_asset` and the phone's `fmblob`
/// handler both go through — did **not** use it: it had its own two-arm `match` that read
/// `thumb_path` directly and hard-errored when the file was absent.
///
/// That is why the read view still asks for full blobs everywhere. `vipsthumbnail` does not exist
/// on Android, so **every** image ingested on a phone has a blob and no thumbnail; asking for the
/// small copy would have turned each one into a 404 placeholder. A performance fix that breaks the
/// picture is not a fix, so the fallback had to land first — these two tests are that prerequisite.
#[test]
fn resolve_asset_bytes_falls_back_to_the_full_blob_when_there_is_no_thumb() {
    let vault = tempdir().unwrap();
    seed(vault.path());
    // A blob with no derivative — the phone's normal case, not an edge one.
    fs::remove_dir_all(vault.path().join("derived")).unwrap();

    let reference = format!("sha256:{HASH}");
    let status = commands::asset_status(vault.path(), &reference).unwrap();
    assert!(status.has_blob && !status.has_thumb, "the fixture must have a blob and no thumb");

    let bytes = commands::resolve_asset_bytes(vault.path(), &reference, "thumb")
        .expect("asking for a thumb that is not there must fall back, never fail");
    assert_eq!(bytes, b"blob-bytes", "the fallback serves the full blob");
}

/// And the entitlement ordering reaches the bytes API too: a `derived/` file must not answer for a
/// vault whose blob the caller was never entitled to. Pinned separately from the path-API twin
/// above, because the whole defect was that the two had drifted apart.
#[test]
fn resolve_asset_bytes_refuses_a_thumb_when_the_blob_itself_is_absent() {
    let vault = tempdir().unwrap();
    seed(vault.path());
    fs::remove_dir_all(vault.path().join("blobs")).unwrap();

    let reference = format!("sha256:{HASH}");
    assert!(
        commands::resolve_asset_bytes(vault.path(), &reference, "thumb").is_err(),
        "a derived file must not stand in for a blob the caller cannot have"
    );
}
