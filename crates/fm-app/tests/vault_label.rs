//! **What a vault is *called* versus what it *is*.**
//!
//! The same repository cloned on two devices can carry two different local names — the owner's laptop
//! and phone did, so one audience looked like two different vaults depending on which screen you were
//! on. The label now follows the remote, which is the thing both devices agree about.
//!
//! **The identity does not move.** A vault's `name` is the write routing key (`MultiStore::route`),
//! the argument seventeen dispatch arms take, and the key behind the UI's persisted view preferences.
//! Renaming to match the remote would reset all of those silently. So these tests check that the
//! label is derived *and* that the name is untouched.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn call(app: &App, cmd: &str) -> serde_json::Value {
    let out = dispatch(cmd, &serde_json::json!({}), &[], app, &NoHost).unwrap();
    serde_json::from_slice(&out.into_bytes()).unwrap()
}

/// A vault directory, optionally a git repo with `origin` set to `remote`.
fn vault(at: &Path, name: &str, remote: Option<&str>) -> PathBuf {
    let p = at.join(name);
    std::fs::create_dir_all(p.join("notes")).unwrap();
    if let Some(url) = remote {
        for args in [vec!["init", "-q"], vec!["remote", "add", "origin", url]] {
            Command::new("git").arg("-C").arg(&p).args(args).output().unwrap();
        }
    }
    p
}

fn app_over(home: &TempDir, dirs: &[(String, PathBuf)]) -> App {
    let config = home.path().join("vaults.json");
    let list = dirs
        .iter()
        .map(|(n, p)| VaultConfig { name: n.clone(), path: p.clone(), restic: None })
        .collect();
    App::new(MultiStore::open(dirs).unwrap(), list, Some(config), true)
}

#[test]
fn a_vault_with_a_remote_is_labelled_with_the_repository_name() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    // The owner's real case: local name `vault`, remote `formicarium-vault`.
    let a = vault(home.path(), "vault", Some("https://github.com/someone/formicarium-vault.git"));
    let app = app_over(&home, &[("vault".into(), a)]);

    let v = call(&app, "list_vaults");
    assert_eq!(v[0]["label"], "formicarium-vault", "shown as the repo, not the folder");
    // **The identity is unchanged.** Everything that routes, persists or commands keys on this.
    assert_eq!(v[0]["name"], "vault");
}

#[test]
fn a_vault_with_no_remote_has_no_label_and_keeps_its_own_name() {
    let home = tempdir().unwrap();
    let a = vault(home.path(), "scratch", None);
    let app = app_over(&home, &[("scratch".into(), a)]);
    let v = call(&app, "list_vaults");
    assert!(v[0]["label"].is_null(), "nothing to derive from: {}", v[0]);
    assert_eq!(v[0]["name"], "scratch");
}

#[test]
fn ssh_and_trailing_slash_urls_still_yield_the_repository_name() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let a = vault(home.path(), "a", Some("git@github.com:someone/personal-notes.git"));
    let b = vault(home.path(), "b", Some("https://example.org/team/lab-notes/"));
    let app = app_over(&home, &[("a".into(), a), ("b".into(), b)]);
    let v = call(&app, "list_vaults");
    assert_eq!(v[0]["label"], "personal-notes", "scp-style remote");
    assert_eq!(v[1]["label"], "lab-notes", "trailing slash");
}

#[test]
fn two_clones_of_one_repo_fall_back_to_their_local_names() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let url = "https://github.com/someone/formicarium-vault.git";
    let a = vault(home.path(), "mine", Some(url));
    let b = vault(home.path(), "theirs", Some(url));
    let app = app_over(&home, &[("mine".into(), a), ("theirs".into(), b)]);

    let v = call(&app, "list_vaults");
    // **Both fall back, not just the second.** Two identical labels would make the vault filter
    // ambiguous — one chip, two audiences — and hiding notes from the wrong vault is worse than
    // showing a local folder name.
    assert!(v[0]["label"].is_null(), "{}", v[0]);
    assert!(v[1]["label"].is_null(), "{}", v[1]);
    assert_eq!(v[0]["name"], "mine");
    assert_eq!(v[1]["name"], "theirs");
}

/// **A vault with no name of its own is named the same way wherever it is asked.**
///
/// `infos` used to take the open stores' names and index them **positionally** against the config
/// list — but `MultiStore::open_with` *skips* a vault it cannot open, so the two lists had different
/// lengths the moment any vault was unopenable, and the fallback then named a vault after a
/// different one. Both callers now ask `descriptor::vault_name`, which is the one place the rule
/// (caller > descriptor > directory) lives.
///
/// Proven red by restoring `store_names.get(i)`: with an unopenable vault ahead of it in the list,
/// the unnamed vault comes back under its neighbour's name.
#[test]
fn an_unnamed_vault_is_not_named_after_its_neighbour_when_one_fails_to_open() {
    let home = tempdir().unwrap();

    // A vault that cannot be opened: `vault.json` is not JSON, so `Descriptor::read` errors and
    // `MultiStore` files it under `unopened` rather than opening it.
    let broken = home.path().join("broken");
    std::fs::create_dir_all(broken.join("notes")).unwrap();
    std::fs::write(broken.join("vault.json"), "{ this is not json").unwrap();

    // And one with an EMPTY name, so the fallback is what answers.
    let unnamed = home.path().join("my-notes");
    std::fs::create_dir_all(unnamed.join("notes")).unwrap();

    let app =
        app_over(&home, &[("broken".into(), broken.clone()), (String::new(), unnamed.clone())]);
    let out = call(&app, "list_vaults");
    let names: Vec<&str> =
        out.as_array().unwrap().iter().filter_map(|v| v["name"].as_str()).collect();

    assert!(
        names.contains(&"my-notes"),
        "the unnamed vault is named after its own directory, not the list position:\n{out}"
    );
    assert!(!names.contains(&""), "and it is not left nameless:\n{out}");
}
