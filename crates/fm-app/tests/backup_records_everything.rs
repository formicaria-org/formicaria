//! **"When I press backup it should commit and push. I do not commit as a user."**
//!
//! The owner's phone showed **180 notes not in history** in a vault that *has* a remote, and
//! pressing Backup did not clear them. That is the whole complaint, and it is a fair one: `Backup`
//! is the only durability affordance in the product, so whatever it does has to be sufficient.
//! "Record all N in history" existing as a second, separate button the user has to know about is
//! git's model leaking into the interface.
//!
//! `sync.svelte.ts` already runs `commit → push` for every entry point including that button, so
//! the sequence is right. What this file pins is the half underneath it: **does a commit, made by a
//! freshly started process, actually record notes an earlier process wrote and never staged?**
//!
//! That distinction is the bug's home. `commit_all` stages exactly the paths `FileStore::put`
//! recorded, deliberately — a vault may also be a project repo, and `add -A` would make this app a
//! second author of someone's index. But that record is *per-process memory*, and Android kills
//! backgrounded apps constantly, so almost every relaunch orphans whatever the previous run wrote.
//! `App::load` is supposed to close that hole by re-adopting them at open (`adoptable` +
//! `seed_written`).
//!
//! The existing `unrecorded_after_restart.rs` cannot see whether that works: it builds the app with
//! `App::new`, which skips adoption entirely. This one goes through **`App::load`** — the real boot
//! path, the one `mobile/src-tauri`'s `boot()` calls — by pointing `FM_VAULTS` at a temp config.
//!
//! Serialised and env-driven on purpose: `FM_VAULTS` is process-global, so these tests must not run
//! beside each other. They share one lock.

use fm_app::{dispatch, App, Host};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use tempfile::{tempdir, TempDir};

/// `FM_VAULTS` is process-global; two of these in parallel would read each other's config.
static ENV: Mutex<()> = Mutex::new(());

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

/// A note file exactly as `FileStore` writes one: `<ULID>.md` with real frontmatter.
fn write_note(notes: &Path, body: &str) -> String {
    let o = fm_model::Object::new(fm_model::Kind::Note, body.to_string());
    let id = o.id.to_string();
    std::fs::write(notes.join(format!("{id}.md")), fm_core::frontmatter::to_file(&o).unwrap())
        .unwrap();
    id
}

/// A git vault with one committed note, and `orphans` more written straight to disk — which is what
/// a process killed before its debounce leaves behind.
fn vault_with_orphans(orphans: usize) -> (TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("v");
    let notes = vault.join("notes");
    std::fs::create_dir_all(&notes).unwrap();

    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "Tester"]);
    git(&vault, &["config", "user.email", "tester@example.com"]);
    write_note(&notes, "the note that did get committed");
    git(&vault, &["add", "-A"]);
    git(&vault, &["commit", "-q", "-m", "first"]);

    for i in 0..orphans {
        write_note(&notes, &format!("orphan {i}: written, never staged"));
    }
    (dir, vault)
}

/// Point `FM_VAULTS` at a config naming this vault, then boot the app the way the phone does.
fn boot(home: &TempDir, vault: &Path) -> App {
    // Stand in for a device with no `git` binary — i.e. the phone. Without this every `vcs::` call
    // resolves to the subprocess backend and the Android code path is never executed, which is the
    // blind spot `vcs.rs` documents on `force_native` itself.
    #[cfg(feature = "native-git")]
    fm_core::vcs::force_native(true);
    let cfg = home.path().join("vaults.json");
    std::fs::write(
        &cfg,
        serde_json::json!({ "vaults": [{ "name": "v", "path": vault }] }).to_string(),
    )
    .unwrap();
    // SAFETY: guarded by `ENV`; these tests are the only writers.
    unsafe { std::env::set_var("FM_VAULTS", &cfg) };
    App::load().expect("the app must open").0
}

fn unrecorded_count(app: &App) -> u64 {
    let v = call(app, "unrecorded", serde_json::json!({})).unwrap();
    v.as_array().map(|a| a.iter().filter_map(|u| u["count"].as_u64()).sum()).unwrap_or(0)
}

/// **The complaint, reproduced end to end.**
///
/// Notes written by a previous process; a fresh boot; one `commit` — the step Backup runs before it
/// pushes. Afterwards nothing may be outstanding. If this fails, pressing Backup reports `synced`
/// (see `commitStep`: `committed: false` with no conflicts carries straight on to the push) while
/// the vault records nothing, which is precisely the shape of "180 not in history that Backup will
/// not clear".
#[test]
fn a_commit_after_a_restart_records_notes_an_earlier_process_wrote() {
    if !have_git() {
        eprintln!("skipping: no git on PATH");
        return;
    }
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempdir().unwrap();
    let (_dir, vault) = vault_with_orphans(12);

    let app = boot(&home, &vault);
    assert_eq!(unrecorded_count(&app), 12, "the orphans must be visible before we try to record");

    call(&app, "commit", serde_json::json!({ "vault": "v", "message": "auto: test" }))
        .expect("commit must not error");

    assert_eq!(
        unrecorded_count(&app),
        0,
        "after a restart and one commit, notes an earlier process wrote are STILL not in history. \
         Backup runs exactly this commit and then pushes, and `commitStep` treats \
         `committed: false` with no conflicts as success — so the user is told the vault is synced \
         while nothing was recorded. That is the '180 not in history' the owner reported.",
    );
}

/// The same, through the button the UI actually offers for it — so that if the two paths ever
/// diverge again, it is visible which one is wrong rather than only that something is.
#[test]
fn record_unrecorded_also_clears_them() {
    if !have_git() {
        eprintln!("skipping: no git on PATH");
        return;
    }
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempdir().unwrap();
    let (_dir, vault) = vault_with_orphans(7);

    let app = boot(&home, &vault);
    assert_eq!(unrecorded_count(&app), 7);

    let r = call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" }))
        .expect("record_unrecorded must not error");
    assert_eq!(r["committed"], serde_json::json!(true), "it should report that it committed: {r}");
    assert_eq!(unrecorded_count(&app), 0, "record_unrecorded must leave nothing outstanding");
}

/// **The owner's actual complaint, as an assertion: Backup must be sufficient on its own.**
///
/// *"When I press backup it should commit and push. I do not commit as a user, that is a background
/// concept."* — and they are right. So this models a session that has been running since **before**
/// the outstanding notes appeared, which is the case boot-time adoption cannot help and the one the
/// phone was in: the app opens on a clean vault, notes then arrive behind its back (a pull, a
/// crashed earlier run, a sync tool), and the user presses Backup.
///
/// Nothing here calls `record_unrecorded`. If that button is ever *required* to reach zero, this
/// fails — which is the point: its necessity is the leak, not its existence.
#[test]
fn backup_alone_clears_the_backlog_without_a_record_step() {
    if !have_git() {
        eprintln!("skipping: no git on PATH");
        return;
    }
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempdir().unwrap();
    let (_dir, vault) = vault_with_orphans(0);

    // Boot on a clean vault: there is nothing to adopt, so the write list starts empty and stays
    // empty. This is a long-lived session, not a fresh relaunch.
    let app = boot(&home, &vault);
    assert_eq!(unrecorded_count(&app), 0, "nothing outstanding at boot");

    // Now notes appear that this process never wrote and was never told about.
    let notes = vault.join("notes");
    for i in 0..9 {
        write_note(&notes, &format!("arrived behind the app's back {i}"));
    }
    assert_eq!(unrecorded_count(&app), 9, "they must show up as outstanding");

    // Exactly what the Backup button runs first (`sync.svelte.ts`: commit → push).
    call(&app, "commit", serde_json::json!({ "vault": "v", "message": "auto: backup" }))
        .expect("commit must not error");

    assert_eq!(
        unrecorded_count(&app),
        0,
        "Backup left notes unrecorded. The commit step records only what this *process* remembers \
         writing, so a session older than the backlog commits nothing and — because `commitStep` \
         treats `committed: false` with no conflicts as success — reports the vault synced anyway. \
         The user is then told to press a second button they should never have needed to know \
         about.",
    );
}

/// **`outstanding.md` §2.9, put to the test: "a backup with no git reports success and records
/// nothing".**
///
/// The report was that on a device with no `git` binary, `commit` answers `{"committed":true}` and
/// creates **no repository at all** — the failure class this repo names repeatedly, *a surface that
/// will not say what it knows*. It was diagnosed but never root-caused, and it long predates
/// Windows getting libgit2.
///
/// This is that exact shape, on the backend such a device actually uses: a vault that has never
/// been a repository, one note on disk, one press of Backup. Either a repository appears and the
/// answer is true, or no repository appears and the answer must not be `true`. Both halves are
/// asserted, because the bug is the *pair* — a truthful `false` here would be almost as bad, since
/// `commitStep` carries a no-conflict `false` straight on to the push and reports "synced".
#[test]
fn a_first_backup_on_a_device_with_no_git_binary_creates_the_repository() {
    if !cfg!(feature = "native-git") && !have_git() {
        eprintln!("skipping: no git on PATH and no libgit2 in this build");
        return;
    }
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let home = tempdir().unwrap();
    let dir = tempdir().unwrap();
    let vault = dir.path().join("v");
    let notes = vault.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    write_note(&notes, "the only note, in a vault that has never been a repo");

    let app = boot(&home, &vault);
    assert!(!vault.join(".git").exists(), "the premise: no repository yet");

    let out = call(&app, "commit", serde_json::json!({ "vault": "v", "message": "backup" }))
        .expect("a first backup must not error");

    assert!(
        vault.join(".git").exists(),
        "pressing Backup reported {out} and created no repository — the notes are files on disk \
         and nothing else, while the user has been told their vault was backed up",
    );
    assert_eq!(out["committed"], serde_json::json!(true), "a first commit records the note");
    assert_eq!(unrecorded_count(&app), 0, "nothing may be left outstanding after a first backup");
}
