//! **An anchored asset reference** — `asset:sha256-<hex>#page=4` — is how a note points at a *place*
//! in a PDF rather than at the file as a whole.
//!
//! `#page=N` is deliberately the standard PDF open parameter and not an invention: a real reader
//! honours it, so the reference degrades to a working link outside this app entirely. That is what
//! keeps files-as-truth honest for an annotation — the note is the store, and the note still means
//! something in a plain text editor.
//!
//! Every one of these failed before 2026-08-29: `parse_ref` requires all-hex, and `asset_status`,
//! `resolve_asset` and `GET /api/blob` all route through it.

use fm_app::refs::{references, strip_cross_vault, REDACTION};

const HEX: &str = "9f2c8a1b3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8";

#[test]
fn an_anchored_reference_still_names_the_blob_it_anchors_into() {
    let body = format!("A highlight.\n\n[p. 4](asset:sha256-{HEX}#page=4)\n");
    let (assets, _) = references(&body);
    // The *bare* blob: the page is where to look, not what to fetch. Anything else and the
    // git-assets size logic and the blob GC would stop seeing a referenced file.
    assert_eq!(assets, vec![HEX.to_string()], "the fragment is not part of the hash");
}

#[test]
fn several_anchors_into_one_paper_are_one_reference() {
    let body = format!(
        "[p. 4](asset:sha256-{HEX}#page=4)\n[p. 9](asset:sha256-{HEX}#page=9)\n\
         and the file itself: ![](asset:sha256-{HEX})\n"
    );
    let (assets, _) = references(&body);
    assert_eq!(assets, vec![HEX.to_string()], "deduped to one blob, however many places");
}

#[test]
fn a_redacted_anchor_leaves_no_page_number_behind() {
    // `REDACTION` promises to carry **none** of the original. A dangling `#page=4` is only a page
    // number, but the contract is the contract — and visible debris looks like a bug.
    let body = format!("see sha256:{HEX}#page=4 for the detail\n");
    let out = strip_cross_vault(&body, false);
    assert!(out.contains(REDACTION), "{out}");
    assert!(!out.contains("#page"), "the fragment went with the token:\n{out}");
    assert!(!out.contains(HEX), "{out}");

    // A fragment must not swallow the sentence after it.
    let body = format!("see sha256:{HEX}#page=4, and then the appendix\n");
    let out = strip_cross_vault(&body, false);
    assert!(out.contains(", and then the appendix"), "the fragment stops at punctuation:\n{out}");
}

#[test]
fn a_markdown_anchor_span_is_removed_whole() {
    let body = format!("prose [p. 4](asset:sha256-{HEX}#page=4) more prose\n");
    let out = strip_cross_vault(&body, false);
    assert!(out.contains(REDACTION), "{out}");
    assert!(!out.contains("p. 4"), "the label goes too — it can name private work:\n{out}");
    assert!(out.contains("prose") && out.contains("more prose"), "{out}");
}

#[test]
fn keeping_assets_keeps_the_anchor_intact() {
    // A same-vault copy keeps its references, fragment and all — otherwise the copy loses the
    // place every highlight pointed at.
    let body = format!("[p. 4](asset:sha256-{HEX}#page=4)\n");
    assert_eq!(strip_cross_vault(&body, true), body);
}

/// **`parse_ref` is the choke point.** `asset_status`, `resolve_asset` and `GET /api/blob` all go
/// through it, and it required every character after the scheme to be hex — so an anchored
/// reference was rejected outright with *"not an asset reference"*, the same string
/// `ui/src/lib/locate.ts` records from the 2026-08-20 phone incident. Exercised here through
/// `asset_status`, which is the public door onto it.
#[test]
fn an_anchored_reference_reaches_the_blob_store_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"%PDF-1.4 not really, but bytes are bytes";
    let stored = fm_core::BlobStore::new(dir.path()).put_bytes(bytes).unwrap();

    // Anchored and bare must answer identically: the page is where to look, not what to fetch.
    for reference in [
        format!("asset:sha256-{}", stored.hash),
        format!("asset:sha256-{}#page=4", stored.hash),
        format!("sha256:{}#page=12", stored.hash),
    ] {
        let got = fm_app::commands::asset_status(dir.path(), &reference)
            .unwrap_or_else(|e| panic!("{reference} was refused: {e}"));
        assert!(got.has_blob, "{reference} should resolve to the blob we just stored");
    }
}
