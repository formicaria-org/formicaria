//! **One fact, one producer** — the discipline the settings consolidation is about.
//!
//! Three commands report what this machine can do: `ping` (every heartbeat), `config` (the settings
//! screen) and `backup_status` (the backup panel). Each probed `vcs::available()`,
//! `backup::available()` and `secrets::has_restic_password()` for itself, and `restic` was exposed
//! under two names — `restic` on `BackupStatus` and `Ping`, `restic_installed` on `Config`. Three
//! probes of one fact are three chances to answer differently.
//!
//! `ui/src/lib/mock.contract.test.ts` exists because two of them already did diverge, on the
//! browser side, over a vault's restic repo. This is the same assertion for the real backend, which
//! had no such test at all.
//!
//! It cannot fail today by construction — they call one function — and that is the point: it is the
//! test that notices when a fourth command grows its own probe.

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

fn call(app: &App, cmd: &str) -> serde_json::Value {
    let out = dispatch(cmd, &serde_json::json!({}), &[], app, &NoHost)
        .unwrap_or_else(|e| panic!("{cmd}: {e}"));
    serde_json::from_slice(&out.into_bytes()).unwrap_or(serde_json::Value::Null)
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

#[test]
fn ping_config_and_backup_status_cannot_disagree_about_this_machine() {
    let home = tempdir().unwrap();
    let v = home.path().join("v");
    std::fs::create_dir_all(v.join("notes")).unwrap();
    let app = app_over(&home, &[("v".into(), v)]);

    let ping = call(&app, "ping");
    let cfg = call(&app, "config");
    let backup = call(&app, "backup_status");

    // Whatever the answer is on this machine, it is the *same* answer three times.
    assert_eq!(ping["git"], cfg["git"], "ping and config disagree about git");
    assert_eq!(ping["git"], backup["git"], "ping and backup_status disagree about git");

    // Two field names, one fact — `restic_installed` on `Config`, `restic` on the other two.
    assert_eq!(ping["restic"], cfg["restic_installed"], "…about restic being installed");
    assert_eq!(ping["restic"], backup["restic"], "…about restic being installed");

    assert_eq!(
        cfg["restic_password_set"], backup["restic_password_set"],
        "…about whether the restic password is held"
    );

    // And they are booleans, not absent — an assertion that two `null`s match would pass for the
    // wrong reason, which is the trap this repo keeps re-learning.
    for (name, value) in
        [("ping.git", &ping["git"]), ("config.git", &cfg["git"]), ("backup.git", &backup["git"])]
    {
        assert!(value.is_boolean(), "{name} is not a bool: {value}");
    }
}
