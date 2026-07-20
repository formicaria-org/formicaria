//! **A captured photo, from bytes to the reference that displays it** — through `dispatch`, the
//! same door the phone's protocol handler and the desktop's `/api/ingest` both go through.
//!
//! # Why this file exists
//!
//! On 2026-07-20 a photo taken on a real phone ingested cleanly — a hash came back, a blob landed,
//! the reference was inserted — and then **rendered as its own filename**, because looking the
//! asset back up said it was not there. Every existing asset test passed throughout.
//!
//! They passed because they call `commands::asset_status(vault_path, reference)` with a path they
//! already hold. That is not what the app does. The app calls **`dispatch`**, which resolves a
//! vault out of the registry, and it does so **twice with different rules**: `ingest` writes into
//! `config(vault)`, one named vault, while `asset_status` searches `configs()` and falls back to
//! `config("")`. Nothing exercised those two resolutions *against each other*, so a disagreement
//! between them was invisible — and a disagreement between them is precisely a blob that is
//! written and then cannot be found.
//!
//! So these tests are deliberately written as the round trip and never as a single call: ingest,
//! then look up **whatever reference the UI would build from the reply**, and require a hit. The
//! reference is rebuilt here the way `assetRef` in `NotePanel.svelte` builds it, because the
//! format is a seam between two languages and a mismatch there fails exactly this way.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

/// The smallest thing a camera could plausibly hand back: a real JPEG header, so MIME sniffing
/// has something true to read rather than being handed a byte string that is not an image.
const JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xFF, 0xD9,
];

fn call(app: &App, cmd: &str, args: Value, body: &[u8]) -> Result<Value, String> {
    dispatch(cmd, &args, body, app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
}

/// An app holding `vaults`, registered, with its config in a tempdir so the real one is untouched.
fn app_with(vaults: &[(&str, PathBuf)]) -> (TempDir, App) {
    let home = tempdir().unwrap();
    for (_, path) in vaults {
        std::fs::create_dir_all(path.join("notes")).unwrap();
    }
    let owned: Vec<(String, PathBuf)> =
        vaults.iter().map(|(n, p)| ((*n).to_string(), p.clone())).collect();
    let configs: Vec<VaultConfig> = vaults
        .iter()
        .map(|(n, p)| VaultConfig { name: (*n).to_string(), path: p.clone(), restic: None })
        .collect();
    let app = App::new(
        MultiStore::open(&owned).unwrap(),
        configs,
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, app)
}

/// The Markdown reference `NotePanel.svelte`'s `assetRef` builds from an ingest reply.
///
/// Kept as one function because it is the format both sides must agree on: the UI writes
/// `asset:sha256-<hex>` (a dash, not a colon, so the Markdown link parses) and `parse_ref` on this
/// side has to accept it. Rebuilding it here means a change to either half fails a test rather
/// than a photo.
fn reference_the_ui_would_write(meta: &Value) -> String {
    let first = meta["assets"][0].as_str().expect("an ingest reply carries the blob in `assets`");
    format!("asset:sha256-{}", first.strip_prefix("sha256:").unwrap_or(first))
}

/// **The whole point of the file**: ingest, then find it again by the reference the editor writes.
#[test]
fn a_captured_photo_is_found_again_by_the_reference_the_editor_inserts() {
    let dir = tempdir().unwrap();
    let (_home, app) = app_with(&[("notes", dir.path().join("notes-vault"))]);

    // The phone sends the filename its camera chose and an *empty* vault — `note?.vault ?? ''` —
    // which is the ordinary case for a note that has not been filed anywhere in particular.
    let meta = call(&app, "ingest", json!({ "name": "JPEG_20260720_0042.jpg", "vault": "" }), JPEG)
        .expect("ingest should accept a photo");

    let reference = reference_the_ui_would_write(&meta);
    let status = call(&app, "asset_status", json!({ "reference": reference }), &[])
        .expect("looking up a just-ingested asset is not an error");

    assert_eq!(
        status["has_blob"], true,
        "ingest stored the blob but asset_status cannot find it — the two resolve vaults \
         differently. This is the 2026-07-20 phone bug: reference {reference}, status {status}"
    );
    assert_eq!(status["mime"], "image/jpeg", "the MIME is what decides <img> vs a download link");
}

/// The same round trip **into a named vault**, because `ingest` takes the name from the note and
/// `asset_status` does not take one at all — it searches. With more than one vault registered,
/// that search is the thing being tested.
#[test]
fn a_photo_ingested_into_a_named_vault_is_found_across_the_others() {
    let dir = tempdir().unwrap();
    let (_home, app) = app_with(&[
        ("first", dir.path().join("first")),
        ("work", dir.path().join("work")),
        ("third", dir.path().join("third")),
    ]);

    // Deliberately not the first vault, so a lookup that only ever checks the default fails here.
    let meta = call(&app, "ingest", json!({ "name": "photo.jpg", "vault": "work" }), JPEG)
        .expect("ingest into a named vault");

    let status =
        call(&app, "asset_status", json!({ "reference": reference_the_ui_would_write(&meta) }), &[])
            .unwrap();
    assert_eq!(
        status["has_blob"], true,
        "a blob in a non-default vault must still be found — asset_status searches every vault \
         because a reference names bytes, not a place"
    );
}

/// **Both spellings resolve.** The editor writes `sha256-` so the Markdown link parses; older
/// notes and the CLI write `sha256:`. Neither may stop working, and a wrong answer here looks
/// exactly like a missing file.
#[test]
fn both_the_dash_and_colon_spellings_find_the_same_blob() {
    let dir = tempdir().unwrap();
    let (_home, app) = app_with(&[("notes", dir.path().join("v"))]);
    let meta = call(&app, "ingest", json!({ "name": "photo.jpg", "vault": "" }), JPEG).unwrap();
    let hash = meta["assets"][0].as_str().unwrap().strip_prefix("sha256:").unwrap().to_string();

    for reference in
        [format!("asset:sha256-{hash}"), format!("asset:sha256:{hash}"), format!("sha256:{hash}")]
    {
        let status = call(&app, "asset_status", json!({ "reference": reference }), &[]).unwrap();
        assert_eq!(status["has_blob"], true, "`{reference}` should resolve to the same blob");
    }
}

/// A reference to something never ingested is a **warning, not an error** — the note renders a
/// placeholder. This is the case the phone bug was mistaken for, and it must stay distinguishable
/// from it: `has_blob: false` and `Ok`, never an `Err`.
#[test]
fn an_absent_blob_answers_honestly_instead_of_failing() {
    let dir = tempdir().unwrap();
    let (_home, app) = app_with(&[("notes", dir.path().join("v"))]);
    let absent = "asset:sha256-".to_string() + &"ab".repeat(32);

    let status = call(&app, "asset_status", json!({ "reference": absent }), &[])
        .expect("media absence is a warning, never an error");
    assert_eq!(status["has_blob"], false);
}
