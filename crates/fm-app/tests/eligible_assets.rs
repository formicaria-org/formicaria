//! **How much a backup is about to send, because a phone cannot be asked.**
//!
//! When the libgit2 backend learned to stage attachments (2026-09-09) the selection rule came with
//! it unchanged: every blob at or under `git_assets_max`, not a diff against what the remote
//! already holds. So a device that has been accumulating photos and has never sent one stages the
//! whole backlog the first time it can.
//!
//! That is what happened. The owner's phone had been taking photos since July with attachments
//! never travelling; the first backup after the fix died with `SSL error: error:80000020: system
//! library::Broken pipe`, and again on a retry — and nothing in the app could say whether five
//! megabytes were pending or five hundred. The phone's logs are unreadable (MIUI suppresses them),
//! so the number had to come from the app itself.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn blob(vault: &Path, hash: &str, bytes: usize) {
    let p = vault.join("blobs/sha256").join(&hash[0..2]).join(&hash[2..4]).join(hash);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, vec![7u8; bytes]).unwrap();
}

fn app_over(home: &TempDir, dirs: &[(String, PathBuf)]) -> App {
    let config = home.path().join("vaults.json");
    let list: Vec<VaultConfig> = dirs
        .iter()
        .map(|(n, p)| VaultConfig { name: n.clone(), path: p.clone(), restic: None })
        .collect();
    fm_app::vaults::save(&list, &config).expect("the fixture vault list must be writable");
    let store = MultiStore::open(dirs).unwrap();
    App::new(store, list, Some(config), true)
}

/// The measurement counts what the rule selects, and says nothing when the vault has not opted in.
#[test]
fn a_vault_reports_the_attachments_its_limit_selects() {
    let home = tempdir().unwrap();
    let dir = home.path().join("v");
    std::fs::create_dir_all(dir.join("notes")).unwrap();

    blob(&dir, "aabb000000000000000000000000000000000000000000000000000000000001", 1000);
    blob(&dir, "ccdd000000000000000000000000000000000000000000000000000000000002", 2000);
    // Over any limit set below, and the case that must never be counted: an oversized file is
    // exactly what a person is trying to find when a push keeps dying.
    blob(&dir, "eeff000000000000000000000000000000000000000000000000000000000003", 9_000_000);

    // **No `vault.json` yet: nothing is eligible, so nothing is claimed.** The default is "notes
    // only", and a count here would tell a user their next backup carries files it will not.
    let (count, bytes) = fm_core::blob::eligible_assets(&dir).unwrap();
    assert_eq!((count, bytes), (0, 0), "a vault that has not opted in sends no attachments");

    std::fs::write(dir.join("vault.json"), r#"{"git_assets_max": "1MB"}"#).unwrap();
    let (count, bytes) = fm_core::blob::eligible_assets(&dir).unwrap();
    assert_eq!(count, 2, "the two under the limit are selected, the 9 MB one is not");
    assert_eq!(bytes, 3000, "and the size is the sum of exactly those");

    // It reaches the panel that has to explain a stuck push.
    let app = app_over(&home, &[("v".into(), dir.clone())]);
    let out = dispatch("backup_status", &serde_json::json!({}), &[], &app, &NoHost).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.into_bytes()).unwrap();
    let row = &v["vaults"][0];
    assert_eq!(row["assets_pending"].as_u64(), Some(2));
    assert_eq!(row["assets_pending_bytes"].as_u64(), Some(3000));
}
