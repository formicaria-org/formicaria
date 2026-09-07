//! Cross-vault create + copy: choosing where a note is born, and porting one into
//! another vault without leaking references out of its new audience.
//!
//! Hermetic: a real two-vault `MultiStore` over tempdirs, so routing, on-disk files,
//! and blob copies all happen for real (a `MemoryStore` can't route by vault).

use fm_app::commands;
use fm_core::{BlobStore, Manifest, MultiStore, Store};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::Filter;
use std::fs;
use std::path::{Path, PathBuf};

// A real 64-hex sha256 (of "abc"), so BlobStore's ab/cd fan-out works.
const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
// A syntactically valid ULID, to sit inside a `note:` link.
const NOTE_ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// personal (default) + lab, and the `(name, root)` list the copy commands take.
fn two() -> (tempfile::TempDir, MultiStore, Vec<(String, PathBuf)>) {
    let dir = tempfile::tempdir().unwrap();
    let personal = dir.path().join("personal");
    let lab = dir.path().join("lab");
    let m = MultiStore::open(&[("personal".into(), &personal), ("lab".into(), &lab)]).unwrap();
    let paths = vec![("personal".to_string(), personal), ("lab".to_string(), lab)];
    (dir, m, paths)
}

fn seed_blob(vault: &Path) {
    // The bytes must actually hash to HASH: copy re-hashes on `put_file` (content-addressed),
    // unlike `resolve_asset_bytes` which reads by path. HASH is sha256("abc").
    let d = vault.join("blobs/sha256").join(&HASH[0..2]).join(&HASH[2..4]);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join(HASH), b"abc").unwrap();
}

fn note_with_asset(m: &mut MultiStore) -> Object {
    let mut n = Object::new(Kind::Note, format!("![p](asset:sha256-{HASH})"));
    n.vault = "personal".into();
    n.assets = vec![format!("sha256:{HASH}")];
    m.put(&n).unwrap();
    n
}

#[test]
fn capture_lands_in_the_named_vault_defaults_empty_and_refuses_unknown() {
    let (_d, mut m, _p) = two();

    let a = commands::capture(&mut m, "a lab thought", "lab").unwrap();
    assert_eq!(a.vault, "lab", "an explicit vault routes there");

    // Empty → the default (first) vault. The returned meta hasn't been re-hydrated, so
    // confirm the *routing* by reading it back from the store.
    let b = commands::capture(&mut m, "a default thought", "").unwrap();
    assert_eq!(
        m.get(b.id.parse().unwrap()).unwrap().unwrap().vault,
        "personal",
        "empty routes to the default vault"
    );

    // The routing itself refuses a typo'd vault (no silent default into the wrong audience).
    let mut typo = Object::new(Kind::Note, "oops");
    typo.vault = "nope".into();
    assert!(m.put(&typo).is_err(), "an unknown vault name is refused, not defaulted");
}

#[test]
fn copy_leaks_no_reference_through_frontmatter_either() {
    // The body was stripped from the start; frontmatter was not, so any custom property
    // (or the title) carried a pointer out of its vault and into permanent git history.
    let (_d, mut m, paths) = two();
    seed_blob(&paths[0].1);
    let mut n = Object::new(Kind::Note, "harmless prose");
    n.vault = "personal".into();
    n.title = Some(format!("Re: [Lab roadmap](note:{NOTE_ID})"));
    // `status` and `tags` are *typed* fields, not `extra` entries — which is exactly how the
    // first version of this fix missed them. They are still free-form user text: `board`
    // groups by any string, and a tag is any string.
    n.status = Some(format!("blocked on note:{NOTE_ID}"));
    n.tags = vec!["real-tag".into(), format!("asset:sha256-{HASH}")];
    n.extra.insert("source".into(), PropertyValue::Text(format!("note:{NOTE_ID}")));
    n.extra.insert(
        "attachments".into(),
        PropertyValue::List(vec![PropertyValue::Text(format!("asset:sha256-{HASH}"))]),
    );
    // A pointer in a property *key*, not a value.
    n.extra.insert(format!("note:{NOTE_ID}"), PropertyValue::Text("keyleak".into()));
    n.extra.insert("count".into(), PropertyValue::Int(3));
    m.put(&n).unwrap();

    let r = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, false).unwrap();
    let copy = m.get(r.meta.id.parse().unwrap()).unwrap().unwrap();

    // Assert over **everything a human can type into**, not a hand-picked pair of fields.
    // The previous version of this test built its haystack from `title` + `extra` only, so it
    // passed green while `status` and `tags` carried pointers straight into the target vault.
    let frontmatter =
        format!("{:?} {:?} {:?} {:?}", copy.title, copy.status, copy.tags, copy.extra);
    assert!(!frontmatter.contains(NOTE_ID), "no note id survives in frontmatter: {frontmatter}");
    assert!(!frontmatter.contains(HASH), "no blob hash survives in frontmatter: {frontmatter}");
    assert!(!frontmatter.contains("Lab roadmap"), "nor the link label: {frontmatter}");
    assert!(copy.tags.iter().any(|t| t == "real-tag"), "an innocent tag survives: {frontmatter}");
    assert_eq!(
        copy.extra.get("count"),
        Some(&PropertyValue::Int(3)),
        "a value with no room for a reference is untouched"
    );
}

#[test]
fn copy_is_prose_only_by_default_and_leaks_no_reference() {
    let (_d, mut m, paths) = two();
    seed_blob(&paths[0].1);
    let mut n = Object::new(
        Kind::Note,
        format!(
            "Plan: ![secret-plan.png](asset:sha256-{HASH}) see [Lab roadmap](note:{NOTE_ID}) end"
        ),
    );
    n.vault = "personal".into();
    n.assets = vec![format!("sha256:{HASH}")];
    m.put(&n).unwrap();

    let r = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, false).unwrap();
    assert_ne!(r.meta.id, n.id.to_string(), "a copy is a NEW note");
    assert_eq!(r.meta.vault, "lab");
    assert!(r.new_blobs.is_empty(), "prose-only copies no blob");

    // The source is untouched.
    assert!(m.get(n.id).unwrap().is_some(), "the original stays put");

    // The copy carries no cross-vault reference — not the id, hash, or the labels.
    let copy = m.get(r.meta.id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(copy.vault, "lab");
    assert!(copy.assets.is_empty(), "frontmatter assets cleared");
    assert!(!copy.body.contains("note:"));
    assert!(!copy.body.contains("asset:"));
    assert!(!copy.body.contains("sha256"));
    assert!(!copy.body.contains("secret-plan"), "the filename label is gone too");
    assert!(!copy.body.contains("Lab roadmap"));
    assert!(copy.body.contains("Plan:") && copy.body.contains("end"), "prose survives");

    // And the blob was not carried into lab.
    assert!(!BlobStore::new(&paths[1].1).exists(HASH), "no blob left the source vault");
}

#[test]
fn copy_with_assets_carries_the_blob_records_it_and_dedups() {
    let (_d, mut m, paths) = two();
    seed_blob(&paths[0].1);
    let n = note_with_asset(&mut m);

    let r = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, true).unwrap();
    assert_eq!(r.new_blobs, vec![HASH.to_string()], "the blob was newly written to lab");
    assert!(BlobStore::new(&paths[1].1).exists(HASH), "blob now lives in the target vault");
    let man = Manifest::read(&paths[1].1).unwrap().unwrap();
    assert!(man.blobs.contains_key(HASH), "and is recorded in the target manifest");

    // The copy keeps the asset reference (now internal to lab).
    let copy = m.get(r.meta.id.parse().unwrap()).unwrap().unwrap();
    assert!(copy.body.contains(&format!("asset:sha256-{HASH}")));
    assert_eq!(copy.assets, vec![format!("sha256:{HASH}")]);

    // A second copy dedups — the bytes are already there.
    let r2 = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, true).unwrap();
    assert!(r2.new_blobs.is_empty(), "already present → nothing newly written");
}

#[test]
fn uncopy_recedes_the_note_and_the_blob_it_wrote() {
    let (_d, mut m, paths) = two();
    seed_blob(&paths[0].1);
    let n = note_with_asset(&mut m);

    let r = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, true).unwrap();
    assert!(BlobStore::new(&paths[1].1).exists(HASH));

    commands::uncopy_note(&mut m, &r.meta.id, "lab", &r.new_blobs, &paths).unwrap();
    assert!(m.get(r.meta.id.parse().unwrap()).unwrap().is_none(), "the copy is gone");
    assert!(!BlobStore::new(&paths[1].1).exists(HASH), "its unreferenced blob is gone too");
}

#[test]
fn uncopy_keeps_a_blob_another_copy_still_references() {
    let (_d, mut m, paths) = two();
    seed_blob(&paths[0].1);
    // Two DISTINCT source notes referencing the same blob (distinct, so neither overrides the
    // other in the target — that only happens when the *same* source is re-copied).
    let a = note_with_asset(&mut m);
    let b = note_with_asset(&mut m);

    let ra = commands::copy_note(&mut m, &a.id.to_string(), "lab", &paths, true).unwrap();
    let _rb = commands::copy_note(&mut m, &b.id.to_string(), "lab", &paths, true).unwrap();
    assert_eq!(
        ra.new_blobs,
        vec![HASH.to_string()],
        "the first copy wrote the blob; the second dedups"
    );

    // Undoing A's copy must not orphan B's: the blob is still referenced there → kept.
    commands::uncopy_note(&mut m, &ra.meta.id, "lab", &ra.new_blobs, &paths).unwrap();
    assert!(BlobStore::new(&paths[1].1).exists(HASH), "still referenced by the other copy → kept");
}

#[test]
fn re_copying_the_same_note_replaces_the_prior_copy_never_duplicates() {
    let (_d, mut m, paths) = two();
    let mut n = Object::new(Kind::Note, "an idea worth sharing");
    n.vault = "personal".into();
    m.put(&n).unwrap();

    let r1 = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, false).unwrap();
    assert_eq!(r1.replaced, 0, "first copy replaces nothing");
    assert!(
        commands::copy_status(&m, &n.id.to_string(), "lab").unwrap(),
        "the target now reports an existing copy"
    );

    let r2 = commands::copy_note(&mut m, &n.id.to_string(), "lab", &paths, false).unwrap();
    assert_eq!(r2.replaced, 1, "the prior copy was replaced, not added alongside");
    assert_ne!(r2.meta.id, r1.meta.id, "the replacement has its own id");
    assert!(m.get(r1.meta.id.parse().unwrap()).unwrap().is_none(), "the old copy is gone");

    let lab_copies =
        m.candidates(&Filter::new()).unwrap().0.into_iter().filter(|o| o.vault == "lab").count();
    assert_eq!(lab_copies, 1, "exactly one copy of the source lives in the target");
}

#[test]
fn copy_refuses_the_notes_own_vault_and_an_unknown_target() {
    let (_d, mut m, paths) = two();
    let mut n = Object::new(Kind::Note, "hi");
    n.vault = "personal".into();
    m.put(&n).unwrap();

    assert!(
        commands::copy_note(&mut m, &n.id.to_string(), "personal", &paths, false).is_err(),
        "copying into its own vault is a no-op error"
    );
    assert!(
        commands::copy_note(&mut m, &n.id.to_string(), "ghost", &paths, false).is_err(),
        "an unknown target vault is refused"
    );
}
