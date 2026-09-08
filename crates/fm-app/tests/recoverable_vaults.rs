//! **"Which vaults are on this device that I have not added?"**
//!
//! `forget_vault.rs`'s header states the assumption this file exists to repair: *"a vault dropped
//! from the list can be added back by pointing at the same directory, so a mistaken click costs a
//! retyped path. That is what makes it safe to put behind one button."* On a desktop that holds.
//! On a phone there is no path to retype and no file picker to find one with — the address is the
//! *name* (`resolve_path` turns an empty path into `<root>/<name>`), and the app shows labels
//! rather than names on every other screen. So the one identity needed to come back was the one
//! the UI had stopped showing.
//!
//! The owner removed the wrong of two vaults on 2026-09-08 and had nothing to click. This is the
//! answer: the app already knows its managed root and already knows what is registered, so it can
//! simply offer the difference.
//!
//! **Its own test binary on purpose.** `FM_VAULT_ROOT` is process-global and `vault_root()` reads
//! it on every call, so a second test running beside this one would see a root it did not set —
//! the same reason `merge_on_a_device_without_git.rs` is one test in one binary.

use fm_app::{dispatch, dispatch_as, vaults::VaultConfig, App, Host, Scope};
use fm_core::MultiStore;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> serde_json::Value {
    let out = dispatch(cmd, &args, &[], app, &NoHost).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    serde_json::from_slice(&out.into_bytes()).unwrap_or(serde_json::Value::Null)
}

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
fn a_forgotten_vault_is_offered_back_by_name_without_anyone_remembering_it() {
    let home = tempdir().unwrap();
    let root = home.path().join("vaults");
    std::fs::create_dir_all(&root).unwrap();
    // The managed root a phone has. Set before anything asks, and never unset: one test, one binary.
    std::env::set_var("FM_VAULT_ROOT", &root);

    // Two vaults in the root, as the owner's phone had — the big one and the idle one.
    let notes = vault_dir(&root, "notes", 5);
    let spare = vault_dir(&root, "vault", 1);
    // And something in the root that is not a vault, which must never be offered.
    std::fs::create_dir_all(root.join("blobs-cache")).unwrap();

    // **A vault that keeps its notes somewhere else.** `vault.json` may say `notes: docs`, and the
    // first version of this scan hardcoded `notes/` — so such a vault was neither detected nor
    // counted, and the one screen that offers a vault back silently would not offer it.
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(elsewhere.join("docs")).unwrap();
    std::fs::write(elsewhere.join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
    std::fs::write(
        elsewhere.join("docs/01AAAAAAAAAAAAAAAAAAAAAA99.md"),
        "---\nschema: 1\nid: 01AAAAAAAAAAAAAAAAAAAAAA99\ntype: note\ntitle: e\ncreated: 2026-07-21T00:00:00Z\nupdated: 2026-07-21T00:00:00Z\n---\nbody\n",
    )
    .unwrap();

    let app = app_over(&home, &[("notes".into(), notes.clone()), ("vault".into(), spare)]);

    // Nothing to recover while both are registered — the list is the difference, not a directory
    // listing, and a screen that always has something on it is a screen nobody reads.
    let none = call(&app, "recoverable_vaults", serde_json::json!({}));
    let offered: Vec<&str> =
        none.as_array().unwrap().iter().filter_map(|v| v["name"].as_str()).collect();
    assert_eq!(
        offered,
        vec!["elsewhere"],
        "both registered vaults are absent, and the one with notes in `docs/` is found:\n{none}"
    );
    assert_eq!(none[0]["notes"], 1, "and counted where it actually keeps them:\n{none}");

    // The accident.
    call(&app, "forget_vault", serde_json::json!({ "name": "notes" }));

    let back = call(&app, "recoverable_vaults", serde_json::json!({}));
    let rows = back.as_array().unwrap();
    // The one that left the list, plus `elsewhere`, which was never in it. Sorted by note count,
    // so the one someone is looking for leads.
    assert_eq!(rows.len(), 2, "the forgotten vault joins the list:\n{back}");
    assert_eq!(rows[0]["name"], "notes", "named by the folder, which is what to type");
    assert_eq!(rows[0]["path"], notes.to_string_lossy().to_string());
    // **The field that answers the real question.** Nobody asks "which slug"; they ask "which one
    // has my work in it", and two folder names tell you nothing.
    assert_eq!(rows[0]["notes"], 5, "and says how much is in it");

    // **A paired device is told nothing** — asserted here, at the one moment there is something to
    // disclose. These folders are not vaults yet, so `Scope` has no opinion about them and cannot
    // be asked, which makes enumerating them a disclosure of what is on the machine to a caller
    // that could not add one anyway. Not in `scoped_dispatch.rs`: that binary sets no
    // `FM_VAULT_ROOT`, so the arm answers empty there whatever the guard does — a test that would
    // pass for the wrong reason.
    let scoped = dispatch_as(
        "recoverable_vaults",
        &serde_json::json!({}),
        &[],
        &app,
        &NoHost,
        &Scope::Only(vec!["vault".into()]),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(scoped.into_bytes()).unwrap(),
        "[]",
        "a scoped caller learned what is on this machine"
    );

    // And the offer is true: adding it back by the name given returns the notes to the app.
    call(
        &app,
        "create_vault",
        serde_json::json!({ "name": "notes", "path": notes.to_string_lossy() }),
    );
    let recent = call(&app, "recent", serde_json::json!({}));
    assert_eq!(
        recent.as_array().unwrap().len(),
        6,
        "five recovered notes plus the one that never left:\n{recent}"
    );
    let after = call(&app, "recoverable_vaults", serde_json::json!({}));
    let still: Vec<&str> =
        after.as_array().unwrap().iter().filter_map(|v| v["name"].as_str()).collect();
    assert_eq!(still, vec!["elsewhere"], "and it stops being offered once it is back:\n{after}");
}
