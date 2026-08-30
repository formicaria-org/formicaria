//! **"Saved as a file in your vault's `views` folder, so it travels with your notes."**
//!
//! That sentence is on screen every time someone saves a view (`ui/src/App.svelte`), and until this
//! test existed it was not true. `views::save_view` does a bare `std::fs::write`, and `commit`
//! stages *"exactly the files this app wrote or deleted — not a directory, and certainly not
//! `-A`"* — where "wrote" means the store's write list plus files matching `<notes>/<ULID>.md`. A
//! `.view` file matches neither, so it was written to disk and never recorded: it survived on the
//! machine that made it and existed nowhere else.
//!
//! The failure is invisible in the only way that matters — the view *works*, right up until you
//! pull on the other machine and it is not there, which is indistinguishable from the app having
//! lost it.
//!
//! So this asserts against **real git**, not against the write list: the question is not whether we
//! remembered to enrol the path, it is whether the file is in the commit.
//!
//! Themes ride the same mechanism and are covered here for the same reason — `themes/*.css` is the
//! second thing the app writes into a vault that is not a note, and it would have inherited the
//! identical defect.

use fm_app::{dispatch, vaults::VaultConfig, App, Host, Output};
use fm_core::MultiStore;
use serde_json::{json, Value};
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
    Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn call(app: &App, cmd: &str, args: Value) -> Value {
    let out: Output =
        dispatch(cmd, &args, &[], app, &NoHost).unwrap_or_else(|e| panic!("{cmd} failed: {e}"));
    match out {
        Output::Json(b) if b.is_empty() => Value::Null,
        o => serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null),
    }
}

/// A git-backed vault with one committed note, and an `App` over it.
fn vault() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["config", "user.name", "Tester"]);
    git(dir.path(), &["config", "user.email", "tester@example.com"]);

    let owned: Vec<(String, PathBuf)> = vec![("personal".into(), dir.path().to_path_buf())];
    let configs = vec![VaultConfig {
        name: "personal".into(),
        path: dir.path().to_path_buf(),
        restic: None,
    }];
    let app = App::new(
        MultiStore::open(&owned).unwrap(),
        configs,
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, dir, app)
}

#[test]
fn a_view_saved_from_the_app_is_in_the_commit() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();

    // A note first, so the commit has ordinary work in it too — the realistic case is a view saved
    // during a session that also wrote notes, and a fix that only works on an otherwise-empty
    // commit would pass a narrower test and fail in life.
    call(&app, "capture", json!({ "body": "# Battery\n\nrough notes\n" }));
    call(
        &app,
        "save_view",
        json!({ "vault": "personal", "name": "Papers", "view": "board", "group_by": "status" }),
    );

    // It is on disk. That much already worked.
    assert!(
        dir.path().join("views/papers.view").is_file(),
        "save_view must write the file"
    );

    call(
        &app,
        "commit",
        json!({ "vault": "personal", "message": "backup: test" }),
    );

    // The only question that matters: is it in git? `ls-tree` reads the commit, not the disk.
    let tracked = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        tracked.lines().any(|l| l == "views/papers.view"),
        "the view the app saved is not in the commit, so it does not travel with the notes.\n\
         HEAD holds:\n{tracked}"
    );
}

#[test]
fn deleting_a_view_is_recorded_too() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    call(&app, "capture", json!({ "body": "# Note\n" }));
    call(
        &app,
        "save_view",
        json!({ "vault": "personal", "name": "Papers", "view": "board" }),
    );
    call(&app, "commit", json!({ "vault": "personal", "message": "backup: add" }));

    // Assert it landed first. Without this the test passes trivially against the old code, where
    // the view was never committed at all and "not in HEAD" was true for the wrong reason.
    let after_add = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        after_add.lines().any(|l| l == "views/papers.view"),
        "the view must be in the commit before deleting it proves anything:\n{after_add}"
    );

    call(&app, "delete_view", json!({ "vault": "personal", "name": "Papers" }));
    call(&app, "commit", json!({ "vault": "personal", "message": "backup: remove" }));

    // A deletion that is not recorded is worse than a save that is not: the file comes back on the
    // next pull, and the user deletes it again, and again.
    let tracked = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        !tracked.lines().any(|l| l == "views/papers.view"),
        "the view was deleted from disk but is still in the commit:\n{tracked}"
    );
}

#[test]
fn a_theme_saved_from_the_app_is_in_the_commit_too() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    call(&app, "capture", json!({ "body": "# Note\n" }));
    call(
        &app,
        "save_theme",
        json!({ "vault": "personal", "name": "Writing Desk", "css": ":root { --bg: #f4f1ea; }" }),
    );
    call(&app, "commit", json!({ "vault": "personal", "message": "backup: theme" }));

    let tracked = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        tracked.lines().any(|l| l == "themes/writing-desk.css"),
        "a theme must travel with the vault, or it is not a theme, it is a local preference.\n\
         HEAD holds:\n{tracked}"
    );

    // And it reads back through the same door the UI uses.
    let css = call(&app, "read_theme", json!({ "vault": "personal", "name": "Writing Desk" }));
    assert_eq!(css.as_str(), Some(":root { --bg: #f4f1ea; }"));
}

#[test]
fn a_theme_is_listed_with_the_vault_that_holds_it() {
    let (_home, _dir, app) = vault();
    call(
        &app,
        "save_theme",
        json!({ "vault": "personal", "name": "Writing Desk", "css": "/* x */" }),
    );
    let listed = call(&app, "list_themes", json!({}));
    let first = &listed.as_array().expect("a list")[0];
    // Without this the UI has a theme and no idea where it lives, and deleting it resolves against
    // the default vault — reporting success having deleted nothing.
    assert_eq!(first["vault"], json!("personal"), "{listed}");
    assert_eq!(first["name"], json!("writing-desk"), "{listed}");
}
