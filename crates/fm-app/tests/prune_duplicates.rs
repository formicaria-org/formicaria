//! **Removing duplicate copies — and refusing to when it would be unrecoverable.**
//!
//! One prompt to the study assistant was written ~142 times on the owner's phone inside a minute
//! (2026-07-31). They are junk copies and want removing, but *how* matters more than *that*: deleting
//! an **untracked** note is unrecoverable — there is no commit to restore it from — while deleting a
//! tracked one is one `git checkout` away.
//!
//! So the order is forced, and enforced here rather than documented: **record first, prune second.**
//! `prune_duplicates` refuses while any copy is still outside git history. The oldest copy of every
//! family always stays, so the content itself is never lost — only the repetition.

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

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap()
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(serde_json::Value::Null))
}

fn open_app(home: &TempDir, vault: &Path) -> App {
    App::new(
        MultiStore::open(&[("v".to_string(), vault.to_path_buf())]).unwrap(),
        vec![VaultConfig { name: "v".into(), path: vault.to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    )
}

/// Three copies of one body — different ids, different `created`, as a loop leaves them — plus a
/// note of its own. ULIDs are Crockford base32: no I, L, O or U.
fn vault_with_copies(home: &TempDir) -> PathBuf {
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);
    for (i, id) in
        ["01CCCCCCCCCCCCCCCCCCCCCC01", "01CCCCCCCCCCCCCCCCCCCCCC02", "01CCCCCCCCCCCCCCCCCCCCCC03"]
            .iter()
            .enumerate()
    {
        std::fs::write(
            vault.join(format!("notes/{id}.md")),
            // **Discussion messages**, carrying `thread_of` — which is what the real copies were.
            // The first version of this test used plain notes, so it passed while the command was
            // filtering messages out with `notes_base()` and finding nothing on the actual device.
            // A fixture that avoids the real shape is a test that certifies the wrong thing.
            format!("---\nschema: 1\nid: {id}\ntype: note\ncreated: 2026-07-31T07:44:0{i}Z\nupdated: 2026-07-31T07:44:0{i}Z\nthread_of: note:01RTRTRTRTRTRTRTRTRTRTRTRT\n---\n@lfm2.5-230m does mRNA change DNA? /search\n"),
        )
        .unwrap();
    }
    std::fs::write(
        vault.join("notes/01PZPZPZPZPZPZPZPZPZPZPZPZ.md"),
        "---\nschema: 1\nid: 01PZPZPZPZPZPZPZPZPZPZPZPZ\ntype: note\ntitle: Keep me\ncreated: 2026-07-31T09:00:00Z\nupdated: 2026-07-31T09:00:00Z\n---\nsomething else entirely\n",
    )
    .unwrap();
    vault
}

#[test]
fn duplicates_are_grouped_with_the_oldest_kept() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = vault_with_copies(&home);
    let app = open_app(&home, &vault);

    let fams = call(&app, "duplicates", serde_json::json!({})).unwrap();
    assert_eq!(fams.as_array().unwrap().len(), 1, "one family, not four notes: {fams}");
    let f = &fams[0];
    // The oldest by `created` is the keeper, deterministically — never offered for deletion.
    assert_eq!(f["keep"], "01CCCCCCCCCCCCCCCCCCCCCC01", "{f}");
    assert_eq!(f["extras"].as_array().unwrap().len(), 2, "{f}");
    assert!(f["preview"].as_str().unwrap().contains("mRNA"), "recognisable: {f}");
}

#[test]
fn pruning_is_refused_while_the_copies_are_not_in_history() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = vault_with_copies(&home);
    let app = open_app(&home, &vault);

    // **The safety property.** Nothing has been committed, so deleting a copy would destroy the only
    // instance of those bytes. The refusal is what makes the button safe to press.
    let err = call(&app, "prune_duplicates", serde_json::json!({ "vault": "v" })).unwrap_err();
    assert!(err.contains("not in git history"), "says why: {err}");
    assert!(err.contains("Record them first"), "and what to do: {err}");
    // And it changed nothing.
    assert_eq!(std::fs::read_dir(vault.join("notes")).unwrap().count(), 4);
}

#[test]
fn once_recorded_the_extras_go_and_the_oldest_stays() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = vault_with_copies(&home);
    let app = open_app(&home, &vault);

    // Record first — now every copy is recoverable from history.
    call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    let out = call(&app, "prune_duplicates", serde_json::json!({ "vault": "v" })).unwrap();
    assert_eq!(out["removed"], 2, "{out}");
    assert_eq!(out["kept"], 1, "{out}");

    // The content survives: one copy of the message, plus the unrelated note.
    assert!(vault.join("notes/01CCCCCCCCCCCCCCCCCCCCCC01.md").exists(), "the oldest copy stays");
    assert!(!vault.join("notes/01CCCCCCCCCCCCCCCCCCCCCC02.md").exists());
    assert!(!vault.join("notes/01CCCCCCCCCCCCCCCCCCCCCC03.md").exists());
    assert!(vault.join("notes/01PZPZPZPZPZPZPZPZPZPZPZPZ.md").exists(), "unrelated note untouched");
    // Nothing left to group.
    assert_eq!(call(&app, "duplicates", serde_json::json!({})).unwrap(), serde_json::json!([]));
    // **And it is undoable**, which is the whole point of the ordering: the deleted copies are in
    // history, one `git checkout` away.
    let show = git(&vault, &["cat-file", "-e", "HEAD:notes/01CCCCCCCCCCCCCCCCCCCCCC02.md"]);
    assert!(show.status.success(), "the removed copy is still in history and restorable");
}
