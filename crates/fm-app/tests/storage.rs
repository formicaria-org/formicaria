//! **Unused attachments** (`storage.rs`, 2026-11-21): which files no note uses, told apart from those
//! only their own attachment entry keeps; a note in the Deleted list still uses its files; removed
//! files wait, come back on request, and go for good after their time.

use fm_app::{dispatch, storage, vaults::VaultConfig, App, Host};
use fm_core::{blob::BlobStore, MultiStore};
use serde_json::{json, Value};
use std::path::Path;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn app() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let store = MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: vault.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

fn call(app: &App, cmd: &str, args: Value, body: &[u8]) -> Value {
    dispatch(cmd, &args, body, app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
        .unwrap()
}

fn hex(h: &str) -> String {
    h.trim_start_matches("sha256:").to_string()
}

#[test]
fn unused_files_are_found_put_aside_put_back_and_let_go() {
    let (_h, vault, app) = app();
    let blobs = BlobStore::new(vault.path());
    // A song's own file (a property) and a picture in a note's text: used.
    let song = hex(&blobs.put_bytes(b"ID3\x04\x00\x00 a song").unwrap().hash);
    let pic = hex(&blobs.put_bytes(b"\x89PNG\r\n\x1a\n a picture").unwrap().hash);
    call(
        &app,
        "create_notes",
        json!({ "notes": [
            { "title": "A song", "props": { "log": "music", "audio": format!("sha256:{song}") } },
            { "title": "A walk", "body": format!("![the path](asset:sha256-{pic})\n") }
        ] }),
        b"",
    );
    // Nothing points at this one.
    let loose = hex(&blobs.put_bytes(b"%PDF-1.7 left over").unwrap().hash);
    // Only its own attachment entry points at this one (uploaded, the note that used it gone).
    let entry = call(&app, "ingest", json!({ "name": "old.mp3" }), b"ID3\x04\x00\x00 an old track");
    let kept = hex(entry["assets"][0].as_str().unwrap());
    let r = call(&app, "storage_unused", json!({ "vault": "notes" }), b"");
    let hashes = |k: &str| {
        r[k].as_array()
            .unwrap()
            .iter()
            .map(|u| u["hash"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(hashes("nothing"), vec![loose.clone()], "{r}");
    assert_eq!(hashes("attachment_only"), vec![kept.clone()], "{r}");
    assert_eq!(r["attachment_only"][0]["name"], "old.mp3");
    assert_eq!(r["attachment_only"][0]["kind"], "sound");
    assert_eq!(r["nothing"][0]["kind"], "document");

    let done = call(
        &app,
        "storage_remove",
        json!({ "vault": "notes", "hashes": [loose, kept, song] }),
        b"",
    );
    assert_eq!(done["files"], 2, "a used file is never put aside, even when asked");
    assert!(blobs.exists(&song) && blobs.exists(&pic));
    assert!(!blobs.exists(&loose) && !blobs.exists(&kept));
    let entry_id = entry["id"].as_str().unwrap();
    assert!(call(&app, "get", json!({ "id": entry_id }), b"").is_null(), "its entry is deleted");
    let r = call(&app, "storage_unused", json!({ "vault": "notes" }), b"");
    assert_eq!(r["held"].as_array().unwrap().len(), 2);

    let back = call(&app, "storage_restore", json!({ "vault": "notes", "hashes": [loose] }), b"");
    assert_eq!(back["files"], 1);
    assert!(blobs.exists(&loose));
    assert_eq!(storage::purge_older_than(vault.path(), 0), 0, "nothing has waited yet");
    assert_eq!(storage::purge_older_than(vault.path(), u64::MAX), 1);
    assert!(storage::held(vault.path()).is_empty());
}
