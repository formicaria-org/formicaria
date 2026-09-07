//! **A captured photo, from bytes to the reference that displays it** — through `dispatch`, the
//! same door the phone's protocol handler and the desktop's `/api/ingest` both go through.
//!
//! # Why this file exists
//!
//! On 2026-07-20 a photo taken on a real phone ingested cleanly — a hash came back, a blob landed,
//! the reference was inserted — and then **rendered as its own filename**. Every existing asset
//! test passed throughout.
//!
//! **What it actually was:** the bytes never left the page. `fetch(url, { body: file })` is
//! correct in a browser and sends an *empty* body through the Android shell's custom-scheme
//! handler, because a `File` is stream-backed. So `ingest` hashed zero bytes and stored the empty
//! blob, and every photo ever taken produced the *same* reference —
//! `e3b0c442…b855`, the SHA-256 of the empty string. That constant is what finally identified it,
//! after two wrong hypotheses about vaults and reference formats.
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

    let status = call(
        &app,
        "asset_status",
        json!({ "reference": reference_the_ui_would_write(&meta) }),
        &[],
    )
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

/// **An ingest with no bytes is refused**, and this is the test that would have caught the bug
/// this file was written for.
///
/// A photo picked on Android was sent as `fetch(url, { body: file })`, which is correct in a
/// browser and delivers an *empty* body through the shell's custom-scheme handler. Nothing
/// errored: ingest hashed zero bytes, stored the empty blob, and returned a reference whose hash
/// is `e3b0c442…b855` — the SHA-256 of the empty string. Every capture produced that same
/// reference, so every one of them rendered a placeholder.
///
/// Accepting an empty file buys nothing; accepting it *silently* hides a broken byte path behind
/// a success message, which cost several days.
#[test]
fn an_ingest_with_no_bytes_is_refused_rather_than_stored() {
    let dir = tempdir().unwrap();
    let (_home, app) = app_with(&[("notes", dir.path().join("v"))]);

    let err = call(&app, "ingest", json!({ "name": "photo.jpg", "vault": "" }), &[])
        .expect_err("an empty body must not be stored as a blob");
    assert!(
        err.contains("no bytes"),
        "the refusal should say the bytes never arrived, not blame the file: {err}"
    );

    // And the empty blob is not left behind for a note to point at.
    let empty_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let status =
        call(&app, "asset_status", json!({ "reference": format!("sha256:{empty_hash}") }), &[])
            .unwrap();
    assert_eq!(status["has_blob"], false, "nothing should have been written");
}

/// Walk every file under `root`, relative to it, sorted. Used to prove *where* an ingest wrote.
fn tree(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// **The bytes land inside the vault, and nowhere else.**
///
/// The vault directory is the unit that gets synced, backed up and carried between machines, so
/// an attachment written beside it rather than inside it is a file that quietly does not travel —
/// and on a phone, where the vault is app-private storage, one that quietly does not exist after
/// a reinstall. "Files-as-truth" means the vault directory *is* the truth.
///
/// This watches the whole parent directory, not just the vault, so a stray write to a sibling
/// path (a cache dir, a temp file left behind, an app-data folder) fails the test rather than
/// going unnoticed.
#[test]
fn an_ingested_file_is_written_inside_the_vault_and_nowhere_else() {
    let home = tempdir().unwrap();
    let vault = home.path().join("the-vault");
    // A sibling that must stay empty: anything written here is an escape.
    std::fs::create_dir_all(home.path().join("elsewhere")).unwrap();
    let (_cfg, app) = app_with(&[("notes", vault.clone())]);

    let before = tree(home.path());
    let meta = call(&app, "ingest", json!({ "name": "photo.jpg", "vault": "" }), JPEG).unwrap();
    let hash = meta["assets"][0].as_str().unwrap().strip_prefix("sha256:").unwrap().to_string();

    // The blob is exactly where the content-addressed layout says, *under the vault*.
    let expected =
        vault.join("blobs").join("sha256").join(&hash[0..2]).join(&hash[2..4]).join(&hash);
    assert!(expected.is_file(), "the blob should be at {}", expected.display());
    assert_eq!(std::fs::read(&expected).unwrap(), JPEG, "and it should be the bytes we sent");

    // Everything new is inside the vault. Nothing landed beside it.
    let new: Vec<String> = tree(home.path()).into_iter().filter(|p| !before.contains(p)).collect();
    assert!(!new.is_empty(), "the ingest wrote nothing at all");
    for path in &new {
        assert!(
            path.starts_with("the-vault/"),
            "ingest wrote outside the vault: {path} (all new files: {new:?})"
        );
    }
}

/// **A zero-byte blob reports as absent**, so notes written during the broken-transport window
/// degrade to the ordinary placeholder rather than a broken image icon.
///
/// Ingest refuses an empty body now, so these can only be debris — but the debris is real: every
/// photo taken on a phone before the fix was stored as zero bytes, and they all collide on the
/// empty string's hash, so one leftover file stands behind every one of those notes.
#[test]
fn a_zero_byte_blob_from_the_broken_window_reports_as_absent() {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("v");
    let (_home, app) = app_with(&[("notes", vault.clone())]);

    // Plant the artefact exactly as the broken build left it.
    let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let at = vault.join("blobs").join("sha256").join("e3").join("b0").join(empty);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, b"").unwrap();

    let status =
        call(&app, "asset_status", json!({ "reference": format!("asset:sha256-{empty}") }), &[])
            .unwrap();
    assert_eq!(
        status["has_blob"], false,
        "a 0-byte blob is not media; reporting it present renders a broken image icon"
    );
}
