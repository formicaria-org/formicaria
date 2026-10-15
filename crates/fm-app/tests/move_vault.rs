//! **Moving a vault's folder from inside the app** (`decisions.md#vault`, 2026-10-15).
//!
//! The owner wanted the main vault out of the source checkout; moving it by hand loses it, and
//! removing and re-adding it moves it to the end of the list — and the first vault is the default
//! for every new note. Pinned here, through the real commands:
//!
//! - the moved vault keeps its **name, its place (still the default), its snapshot repo** and its
//!   notes, and the vault list names the new folder;
//! - every refusal the form shows as you type: a folder with files in it, inside the vault itself,
//!   overlapping another vault, a vault that is not there;
//! - **the way back**: if the vault list cannot be saved, the move is undone and the vault reopens
//!   where it was.
//!
//! The cross-disk copy, its byte-for-byte verification and a copy that fails halfway are pinned
//! one level down, in `fm_core::relocate`'s own tests, where the copy path can be forced.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::{FileStore, MultiStore, Store};
use fm_model::{Kind, Object};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn call(app: &App, cmd: &str, args: Value) -> Result<Value, String> {
    let out = dispatch(cmd, &args, &[], app, &NoHost)?;
    Ok(serde_json::from_slice(&out.into_bytes()).unwrap_or(Value::Null))
}

/// Two vaults, `main` (the default, with a snapshot repo) and `lab`, each holding one note, and
/// the vault list on disk that names them.
fn setup(home: &Path) -> (App, PathBuf) {
    let (main, lab) = (home.join("main"), home.join("lab"));
    for (dir, name, body) in [(&main, "main", "a note in main"), (&lab, "lab", "a note in lab")] {
        let mut s = FileStore::named(dir, name).unwrap();
        let mut o = Object::new(Kind::Note, body);
        o.vault = name.into();
        s.put(&o).unwrap();
    }
    let list = home.join("vaults.json");
    std::fs::write(
        &list,
        serde_json::to_string_pretty(&json!({ "vaults": [
            { "name": "main", "path": main, "restic": "/backup/main" },
            { "name": "lab", "path": lab },
        ]}))
        .unwrap(),
    )
    .unwrap();
    let configs = vec![
        VaultConfig {
            name: "main".into(),
            path: main.clone(),
            restic: Some("/backup/main".into()),
        },
        VaultConfig { name: "lab".into(), path: lab.clone(), restic: None },
    ];
    let store = MultiStore::open(&[("main".to_string(), main), ("lab".to_string(), lab)]).unwrap();
    (App::new(store, configs, Some(list.clone()), true), list)
}

fn listed(list: &Path) -> Vec<Value> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(list).unwrap()).unwrap();
    v["vaults"].as_array().unwrap().clone()
}

#[test]
fn the_default_vault_moves_and_stays_the_default_with_its_settings() {
    let home = tempfile::tempdir().unwrap();
    let (app, list) = setup(home.path());
    let to = home.path().join("elsewhere/main-vault");

    let check = call(&app, "check_move_vault", json!({ "vault": "main", "path": to })).unwrap();
    assert_eq!(check["ok"], true, "{check}");
    assert!(check["files"].as_u64().unwrap() > 0, "it says how much will move: {check}");

    let r = call(&app, "move_vault", json!({ "vault": "main", "path": to })).unwrap();
    assert_eq!(r["oldRemoved"], true, "{r}");
    assert!(!home.path().join("main").exists(), "the old folder is gone");
    assert!(to.join("notes").is_dir(), "the vault is in the new folder");

    // The list, on disk: same entries, same order, the snapshot repo kept, the new path.
    let v = listed(&list);
    assert_eq!(v[0]["name"], "main", "still first, so still the default");
    assert_eq!(v[0]["path"], to.to_string_lossy().as_ref());
    assert_eq!(v[0]["restic"], "/backup/main");
    assert_eq!(v[1]["name"], "lab");

    // Live, with no restart: its notes are there, and a new note still lands in it.
    let recent = call(&app, "recent", json!({})).unwrap().to_string();
    assert!(recent.contains("a note in main") && recent.contains("a note in lab"), "{recent}");
    let made = call(&app, "capture", json!({ "text": "written after the move" })).unwrap();
    assert_eq!(made["vault"], "main", "the default is unchanged: {made}");
    let disk = std::fs::read_dir(to.join("notes")).unwrap().count();
    assert_eq!(disk, 2, "the new note is a file in the new folder");
}

#[test]
fn a_move_is_refused_where_it_would_lose_or_mix_anything() {
    let home = tempfile::tempdir().unwrap();
    let (app, _) = setup(home.path());
    let busy = home.path().join("busy");
    std::fs::create_dir_all(&busy).unwrap();
    std::fs::write(busy.join("theirs.txt"), "x").unwrap();

    for (path, why) in [
        (busy.clone(), "already has files"),
        (home.path().join("main/inner"), "inside itself"),
        (home.path().join("lab/sub"), "overlaps the 'lab' vault"),
        (home.path().to_path_buf(), "overlaps"),
        (home.path().join("main"), "already there"),
    ] {
        let c = call(&app, "check_move_vault", json!({ "vault": "main", "path": path })).unwrap();
        assert_eq!(c["ok"], false, "{path:?}: {c}");
        assert!(c["problem"].as_str().unwrap().contains(why), "{path:?}: {c}");
        let r = call(&app, "move_vault", json!({ "vault": "main", "path": path }));
        assert!(r.is_err(), "{path:?} must be refused by the move too, not only the form");
    }
    assert!(home.path().join("main/notes").is_dir(), "nothing moved");
    assert_eq!(std::fs::read_to_string(busy.join("theirs.txt")).unwrap(), "x");
    let none =
        call(&app, "check_move_vault", json!({ "vault": "nope", "path": "/tmp/x" })).unwrap();
    assert_eq!(none["ok"], false);
}

/// **The way back.** The vault list's folder is made read-only after the app has loaded, so the new
/// path cannot be saved: the move must be undone and the vault keep working where it was.
#[cfg(unix)]
#[test]
fn when_the_list_cannot_be_saved_the_move_is_undone() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let conf = home.path().join("conf");
    std::fs::create_dir_all(&conf).unwrap();
    let (app, list) = setup(home.path());
    // The list lives in its own folder, so only that folder can be locked.
    let moved_list = conf.join("vaults.json");
    std::fs::rename(&list, &moved_list).unwrap();
    let configs = vec![
        VaultConfig {
            name: "main".into(),
            path: home.path().join("main"),
            restic: Some("/backup/main".into()),
        },
        VaultConfig { name: "lab".into(), path: home.path().join("lab"), restic: None },
    ];
    drop(app);
    let store = MultiStore::open(&[
        ("main".to_string(), home.path().join("main")),
        ("lab".to_string(), home.path().join("lab")),
    ])
    .unwrap();
    let app = App::new(store, configs, Some(moved_list.clone()), true);
    std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores permissions; then there is no failure to provoke.
    if std::fs::write(conf.join("probe"), "x").is_ok() {
        return;
    }

    let to = home.path().join("elsewhere/main");
    let r = call(&app, "move_vault", json!({ "vault": "main", "path": to }));
    std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o755)).unwrap();

    let err = r.unwrap_err();
    assert!(err.contains("undone"), "{err}");
    assert!(home.path().join("main/notes").is_dir(), "the vault is back where it was");
    assert!(!to.exists(), "and not also in the new place");
    let recent = call(&app, "recent", json!({})).unwrap().to_string();
    assert!(recent.contains("a note in main"), "and it is open and working: {recent}");
    assert_eq!(listed(&moved_list)[0]["path"], home.path().join("main").to_string_lossy().as_ref());
}
