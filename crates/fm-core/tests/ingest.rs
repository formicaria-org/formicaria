//! S5: asset ingest. The blob store is content-addressed and deduplicating; the
//! extracted text lands in the asset note's body and is therefore full-text
//! searchable through the same FTS path as any note. These tests are hermetic —
//! they use text files, so no external tool (pdftotext/vipsthumbnail) is needed.
//! The pdftotext path is verified end-to-end via the `fm add` CLI against a real
//! PDF, where poppler is on PATH from the pixi env.

use fm_core::blob::BlobStore;
use fm_core::{ingest_file, FileStore, Store};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::{Filter, Predicate, Query};
use std::fs;
use tempfile::tempdir;

#[test]
fn identical_bytes_are_stored_once() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path());

    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt"); // different name, identical bytes
    fs::write(&a, "same bytes").unwrap();
    fs::write(&b, "same bytes").unwrap();

    let first = store.put_file(&a).unwrap();
    let second = store.put_file(&b).unwrap();

    assert_eq!(first.hash, second.hash, "same content → same hash");
    assert!(!first.deduped, "first write stores the bytes");
    assert!(second.deduped, "second write is a dedup, not a rewrite");

    // Exactly one blob file exists under the content-addressed tree.
    let count = walk_count(&dir.path().join("blobs"));
    assert_eq!(count, 1, "one blob on disk for identical bytes");
    assert!(store.exists(&first.hash));
}

#[test]
fn known_content_hashes_to_the_expected_sha256() {
    // Anchors the hashing to the universal tool: `printf 'abc' | sha256sum`.
    let dir = tempdir().unwrap();
    let f = dir.path().join("abc");
    fs::write(&f, b"abc").unwrap();
    let stored = BlobStore::new(dir.path()).put_file(&f).unwrap();
    assert_eq!(
        stored.hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn ingesting_a_text_file_makes_its_contents_searchable() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("meta-rl.txt");
    fs::write(&src, "GAE lambda interacts badly with inner-loop adaptation. marker_xyzzy.").unwrap();

    let ing = ingest_file(dir.path(), &src).unwrap();
    assert_eq!(ing.filename, "meta-rl.txt");
    assert!(ing.text.as_deref().unwrap().contains("marker_xyzzy"));

    // Build the asset note exactly as `fm add` does, then prove FTS finds a word
    // that only appears *inside* the ingested file.
    let mut store = FileStore::open(dir.path()).unwrap();
    let mut obj = Object::new(Kind::Asset, ing.text.clone().unwrap());
    obj.title = Some(ing.filename.clone());
    obj.assets = vec![format!("sha256:{}", ing.hash)];
    obj.extra.insert("mime".into(), PropertyValue::Text(ing.mime.clone()));
    store.put(&obj).unwrap();

    let q = Query {
        filter: Filter::new().and(Predicate::Text("marker_xyzzy".into())),
        ..Default::default()
    };
    let hits = store.query(&q).unwrap();
    assert_eq!(hits.total, 1, "search inside the asset's extracted text finds it");
    assert_eq!(hits.rows[0].kind, Kind::Asset);
}

#[test]
fn put_bytes_dedups_and_ingest_bytes_sniffs_mime() {
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    // PNG magic bytes — `infer` recognizes image/png from the signature alone.
    let png: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let first = store.put_bytes(png).unwrap();
    let second = store.put_bytes(png).unwrap();
    assert_eq!(first.hash, second.hash, "same bytes → same hash");
    assert!(!first.deduped, "first write stores the bytes");
    assert!(second.deduped, "second is a dedup, not a rewrite");
    assert!(store.exists(&first.hash));

    let ing = fm_core::ingest::ingest_bytes(dir.path(), "poster.png", png).unwrap();
    assert_eq!(ing.mime, "image/png", "MIME sniffed from magic bytes, not the name");
    assert_eq!(ing.filename, "poster.png");
    assert_eq!(
        fm_core::ingest::sniff_mime(&store.path_for(&ing.hash)).as_deref(),
        Some("image/png"),
        "sniff_mime reads the stored blob"
    );
}

#[test]
fn svg_is_sniffed_as_image_even_though_infer_misses_it() {
    // SVG is XML text with no magic bytes, so `infer` returns None — but the read
    // view must get `image/svg+xml` or an <img> won't draw it. Both the ingest
    // path and the stored-blob sniff must agree.
    let dir = tempdir().unwrap();
    let store = BlobStore::new(dir.path());
    let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    let ing = fm_core::ingest::ingest_bytes(dir.path(), "logo.svg", svg).unwrap();
    assert_eq!(ing.mime, "image/svg+xml", "ingest sniffs SVG by its <svg> head");

    assert_eq!(
        fm_core::ingest::sniff_mime(&store.path_for(&ing.hash)).as_deref(),
        Some("image/svg+xml"),
        "asset_status re-sniffs the stored blob as SVG too"
    );
}

fn walk_count(dir: &std::path::Path) -> usize {
    let mut n = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                n += walk_count(&p);
            } else {
                n += 1;
            }
        }
    }
    n
}
