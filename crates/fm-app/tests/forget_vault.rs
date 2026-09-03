//! **Removing a vault from the list — the fourth verb the vault list was missing.**
//!
//! Three commands brought a vault into being (`create_vault`, `clone_vault`, `restore_vault`) and
//! none took one away. On the phone that is not a papercut: `configure_paths` auto-creates an empty
//! default vault on first launch, so the owner ended up with one they never asked for, could not use,
//! and could not remove from inside the product — and the product is the only way in (owner,
//! 2026-07-31: *"creates only confusion"*).
//!
//! The property every test here defends: **forgetting a vault never touches a file.** "Forget" and
//! "destroy" are different verbs and only one is reversible — a vault dropped from the list can be
//! added back by pointing at the same directory, so a mistaken click costs a retyped path. That is
//! what makes it safe to put behind one button.

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

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<String, String> {
    dispatch(cmd, &args, &[], app, &NoHost).map(|o| String::from_utf8(o.into_bytes()).unwrap())
}

/// A vault directory holding `notes` note files.
fn vault_dir(at: &Path, name: &str, notes: usize) -> PathBuf {
    let p = at.join(name);
    std::fs::create_dir_all(p.join("notes")).unwrap();
    for i in 0..notes {
        let id = format!("01AAAAAAAAAAAAAAAAAAAAAA{i:02}");
        std::fs::write(
            p.join(format!("notes/{id}.md")),
            format!("---\nschema: 1\nid: {id}\ntype: note\ntitle: n{i}\ncreated: 2026-07-21T00:00:00Z\nupdated: 2026-07-21T00:00:00Z\n---\nbody\n"),
        )
        .unwrap();
    }
    p
}

/// An app over the given vaults, with its own vault-list file — **and the file is written first.**
///
/// **That write is the whole point, and its absence hid a shipped bug for as long as this test has
/// existed.** Pointing `App` at a `vaults.json` that does not exist puts every assertion below on a
/// path no real installation is ever on: `vaults::save` starts from `{}` and appends, so "the entry
/// is not in the file afterwards" was true because it was never in the file to begin with. On a
/// real machine the entry *is* there, `save` skips it as "theirs", and the forgotten vault comes
/// back on the next start. Seeding the file is what makes this test able to fail.
fn app_over(home: &TempDir, dirs: &[(String, PathBuf)]) -> App {
    let config = home.path().join("vaults.json");
    let list: Vec<VaultConfig> = dirs
        .iter()
        .map(|(n, p)| VaultConfig { name: n.clone(), path: p.clone(), restic: None })
        .collect();
    // Written through `save` rather than hand-rolled JSON, so the fixture is by construction the
    // shape the app actually persists — including anything a later change makes it write.
    fm_app::vaults::save(&list, &config).expect("the fixture vault list must be writable");
    let store = MultiStore::open(dirs).unwrap();
    App::new(store, list, Some(config), true)
}

#[test]
fn forgetting_an_empty_vault_removes_it_from_the_list() {
    let home = tempdir().unwrap();
    let dummy = vault_dir(home.path(), "notes", 0);
    let real = vault_dir(home.path(), "vault", 2);
    let app = app_over(&home, &[("notes".into(), dummy.clone()), ("vault".into(), real.clone())]);

    let out = call(&app, "forget_vault", serde_json::json!({ "name": "notes" })).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["forgotten"], "notes");
    assert_eq!(v["notes"], 0, "it was empty, and the answer says so");

    // The live list, and the persisted one, both agree it is gone.
    let listed = call(&app, "list_vaults", serde_json::json!({})).unwrap();
    assert!(listed.contains("vault"), "the real vault is untouched: {listed}");
    assert!(!listed.contains("\"notes\""), "the dummy is gone: {listed}");
    let saved = std::fs::read_to_string(home.path().join("vaults.json")).unwrap();
    assert!(!saved.contains("\"notes\""), "and it does not come back on restart: {saved}");
}

#[test]
fn forgetting_never_deletes_a_file() {
    let home = tempdir().unwrap();
    let real = vault_dir(home.path(), "vault", 3);
    let app = app_over(&home, &[("vault".into(), real.clone())]);

    let out = call(&app, "forget_vault", serde_json::json!({ "name": "vault" })).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();

    // **The load-bearing assertion.** The notes are still exactly where they were, and the answer
    // said how many and where — so "removed from the list" can never be read as "erased".
    assert_eq!(v["notes"], 3, "it counted what is being left behind");
    assert_eq!(v["path"], real.to_string_lossy().to_string(), "and said where");
    assert_eq!(
        std::fs::read_dir(real.join("notes")).unwrap().count(),
        3,
        "every note file survives"
    );
}

#[test]
fn forgetting_the_last_vault_lands_on_the_first_run_state() {
    let home = tempdir().unwrap();
    let only = vault_dir(home.path(), "notes", 0);
    let app = app_over(&home, &[("notes".into(), only)]);

    call(&app, "forget_vault", serde_json::json!({ "name": "notes" })).unwrap();
    // `[]` is how the UI already detects the first run — the honest state for a machine with no
    // vaults. Refusing here would leave someone stuck with a vault they cannot use or remove.
    let listed = call(&app, "list_vaults", serde_json::json!({})).unwrap();
    assert_eq!(listed.trim(), "[]", "no vaults, which is a state the app knows: {listed}");
}

#[test]
fn an_unknown_vault_is_an_error_not_a_silent_success() {
    let home = tempdir().unwrap();
    let real = vault_dir(home.path(), "vault", 1);
    let app = app_over(&home, &[("vault".into(), real)]);

    let err = call(&app, "forget_vault", serde_json::json!({ "name": "typo" })).unwrap_err();
    assert!(err.contains("no vault named 'typo'"), "says what was wrong: {err}");
    // Nothing moved.
    assert!(call(&app, "list_vaults", serde_json::json!({})).unwrap().contains("vault"));
}

#[test]
fn a_missing_name_is_refused() {
    let home = tempdir().unwrap();
    let app = app_over(&home, &[("vault".into(), vault_dir(home.path(), "vault", 1))]);
    // A blank arg is what a client bug looks like (`api()` turns malformed JSON into empty strings —
    // see known-issues.md), and "forget the vault called nothing" must never guess.
    let err = call(&app, "forget_vault", serde_json::json!({})).unwrap_err();
    assert!(err.contains("needs a name"), "{err}");
}
