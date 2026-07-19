//! **Turning a repository into a vault, the way a user does it** — through `dispatch`, the same
//! door the desktop and the phone both go through.
//!
//! # Why this file exists
//!
//! An evening was spent on bugs that every existing test was structurally unable to see:
//!
//! - `clone` attached no credentials callback. The differential test cloned over `file://`, and
//!   **a local path never authenticates**, so it passed identically with the callback missing.
//! - The trust store could not load at all on Android, which no host test can reach.
//! - `naturalise` had never been exercised through the command surface, only in isolation.
//!
//! The lesson is the shape of the tests, not the bugs: *a test that exercises a code path proves
//! nothing about a property that path only has against a real remote*. So this file is in two
//! halves.
//!
//! **Offline (always runs, part of `pixi run ci`)** — a real bare repo on disk, cloned and
//! adopted through the real commands. Catches registration, adoption, the per-machine state that
//! must not travel, and every refusal.
//!
//! **Online (opt-in, skipped when unreachable)** — the same flow against a real public repo over
//! HTTPS. This is the half with teeth for anything to do with transport: TLS, redirects,
//! credential plumbing. It is skipped rather than failed when there is no network, so `pixi run
//! ci` on a train stays green — but it is *not* hidden behind a feature flag, because a test
//! nobody runs is a test that does not exist.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

/// The public repo this exercises. Small, stable, and the owner's own — so it can be relied on
/// without asking anything of a third party.
const PUBLIC_REPO: &str = "https://github.com/singhbal-baljinder/teaching-scripts.git";

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// An app with an empty vault list in its own tempdir, so nothing touches the real config.
fn app() -> (TempDir, App) {
    let home = tempdir().unwrap();
    let config = home.path().join("vaults.json");
    let app = App::new(MultiStore::open(&[] as &[(String, PathBuf)]).unwrap(), Vec::<VaultConfig>::new(), Some(config), true);
    (home, app)
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<String, String> {
    dispatch(cmd, &args, &[], app, &NoHost).map(|o| String::from_utf8(o.into_bytes()).unwrap())
}

/// A bare repo holding one note, as a collaborator would leave it.
fn origin_with_a_note(at: &Path) -> String {
    let work = at.join("work");
    std::fs::create_dir_all(work.join("notes")).unwrap();
    std::fs::write(
        work.join("notes/01KXGCF14BWT0XPKN8TH1YWRC0.md"),
        "---\nschema: 1\nid: 01KXGCF14BWT0XPKN8TH1YWRC0\ntype: note\ntitle: theirs\n\
         created: 2026-07-15T06:07:19.859303633Z\nupdated: 2026-07-16T07:37:23.775709462Z\n\
         ---\nsomething they wrote\n",
    )
    .unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "-A"],
        vec!["-c", "user.name=T", "-c", "user.email=t@e.org", "commit", "-qm", "theirs"],
    ] {
        assert!(Command::new("git").current_dir(&work).args(args).status().unwrap().success());
    }
    work.to_string_lossy().into_owned()
}

// ───────────────────────────── offline: always runs ─────────────────────────────

/// **Join a shared vault**, end to end through the command surface: clone, naturalise, register,
/// open — and the collaborator's note is readable afterwards.
#[test]
fn cloning_a_repo_registers_it_and_the_notes_are_there() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    let (home, app) = app();
    let url = origin_with_a_note(home.path());
    let dest = home.path().join("joined");

    let out = call(
        &app,
        "clone_vault",
        json!({ "name": "joined", "path": dest.to_string_lossy(), "url": url,
                "gitName": "Ada", "gitEmail": "ada@example.org" }),
    )
    .expect("clone should succeed");
    assert!(out.contains("joined"), "the new vault is in the returned list: {out}");

    // It is a *vault*, not merely a clone: the ignore rules and merge attribute are what make
    // the difference, and a collaborator who never gets them hits the `updated:` conflict.
    assert!(dest.join(".gitattributes").exists(), "a clone must be made a vault");
    assert!(
        std::fs::read_to_string(dest.join(".gitignore")).unwrap().contains("index.sqlite"),
        "the per-machine index must be ignored in an acquired vault"
    );

    // The identity we asked for signs it — not the placeholder.
    let id = fm_core::vcs::identity(&dest).expect("a joined vault has a real committer");
    assert_eq!(id.email, "ada@example.org");

    // And the note is visible through the app, which is the only claim a user cares about.
    let notes = call(&app, "recent", json!({ "limit": 10 })).unwrap();
    assert!(notes.contains("theirs"), "their note should be readable: {notes}");
}

/// **Adopt a directory that is already a repo** — the "I have a folder of notes" route. It must
/// take the notes as they are, with no import step, and must not `git init` a nested repo inside
/// the one already there.
#[test]
fn adopting_an_existing_repo_keeps_its_history_and_notes() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    let (home, app) = app();
    let existing = PathBuf::from(origin_with_a_note(home.path()));
    let git_before = std::fs::read_to_string(existing.join(".git/HEAD")).unwrap();

    call(
        &app,
        "create_vault",
        json!({ "name": "adopted", "path": existing.to_string_lossy() }),
    )
    .expect("adopting a directory that already has notes should succeed");

    let notes = call(&app, "recent", json!({ "limit": 10 })).unwrap();
    assert!(notes.contains("theirs"), "adopted notes are visible with no import: {notes}");
    assert_eq!(
        std::fs::read_to_string(existing.join(".git/HEAD")).unwrap(),
        git_before,
        "adopting must not re-init or otherwise disturb the repo already there"
    );
}

/// Refusals, which are the half of acquisition that protects someone's data. Each of these was
/// a real hazard before it was a rule.
#[test]
fn acquisition_refuses_what_it_should_before_touching_the_disk() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    let (home, app) = app();
    let url = origin_with_a_note(home.path());

    // A clone into a non-empty directory would leave a half-made vault somewhere the user
    // cannot retry into, so it is refused before anything is written.
    let occupied = home.path().join("occupied");
    std::fs::create_dir_all(&occupied).unwrap();
    std::fs::write(occupied.join("keep.txt"), "mine").unwrap();
    let err = call(
        &app,
        "clone_vault",
        json!({ "name": "x", "path": occupied.to_string_lossy(), "url": url,
                "gitName": "Ada", "gitEmail": "ada@example.org" }),
    )
    .unwrap_err();
    assert!(!err.is_empty());
    assert!(occupied.join("keep.txt").exists(), "their file must survive a refusal");

    // A shared vault has an audience by definition, so it may not be signed by nobody.
    let err = call(
        &app,
        "clone_vault",
        json!({ "name": "y", "path": home.path().join("y").to_string_lossy(), "url": url,
                "gitName": "", "gitEmail": "" }),
    )
    .unwrap_err();
    assert!(err.contains("name and email"), "must ask who you are: {err}");

    // An email that is not one is the mistake that is invisible afterwards, signed into history.
    let err = call(
        &app,
        "clone_vault",
        json!({ "name": "z", "path": home.path().join("z").to_string_lossy(), "url": url,
                "gitName": "Ada", "gitEmail": "ada" }),
    )
    .unwrap_err();
    assert!(err.contains("not an email"), "must refuse a non-address: {err}");
}

// ───────────────────────── online: opt-in, skipped when offline ─────────────────────────

/// Whether the public repo is reachable right now. Anything else — DNS, TLS, a proxy — means
/// this machine cannot run the online half, and saying so beats a red suite on a train.
fn online() -> bool {
    Command::new("git")
        .args(["ls-remote", "--exit-code", PUBLIC_REPO, "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// **The half with teeth.** A real repository over real HTTPS, through the real command.
///
/// This is the shape of test that the `file://` differential clone could never be: a local path
/// negotiates no TLS, follows no redirect and never authenticates, so it proved the code path
/// ran while saying nothing about whether it could talk to a forge. Everything that broke on the
/// phone lived in precisely that gap.
#[test]
fn a_real_public_repo_becomes_a_vault_over_https() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    if !online() {
        eprintln!("skipping: {PUBLIC_REPO} is not reachable from here");
        return;
    }
    let (home, app) = app();
    let dest = home.path().join("teaching-scripts");

    call(
        &app,
        "clone_vault",
        json!({ "name": "teaching", "path": dest.to_string_lossy(), "url": PUBLIC_REPO,
                "gitName": "Ada", "gitEmail": "ada@example.org" }),
    )
    .expect("a public repo should clone over HTTPS and register as a vault");

    // A repo that was never a formicaria vault still becomes one: it keeps its own files, and
    // gains the ignore rules and merge attribute that make merging safe.
    assert!(dest.join(".git").exists(), "history came with it");
    assert!(dest.join(".gitattributes").exists(), "and it was made a vault");
    assert!(
        std::fs::read_to_string(dest.join(".gitignore")).unwrap().contains("index.sqlite"),
        "the per-machine index must never travel from here"
    );

    // It opens. A repo with no `notes/` is an empty vault, not a broken one — which is the
    // honest outcome for turning an arbitrary repo into a notebook.
    let listed = call(&app, "list_vaults", json!({})).unwrap();
    assert!(listed.contains("teaching"), "registered and open: {listed}");
}

/// The probe, against something real. It answers the question the clone form asks on every
/// keystroke, and its three states are what turn an unusable git error into a next step.
#[test]
fn probing_a_real_repo_reports_it_reachable() {
    if !have_git() || !online() {
        eprintln!("skipping: offline");
        return;
    }
    let (_home, app) = app();

    let out = call(&app, "probe_remote", json!({ "url": PUBLIC_REPO })).unwrap();
    assert!(out.contains("reachable"), "a public repo answers: {out}");

    // A URL that is simply wrong must NOT be reported as an authentication problem — that is
    // the misclassification that sends someone to configure credentials for a typo.
    let out = call(
        &app,
        "probe_remote",
        json!({ "url": "https://github.com/singhbal-baljinder/no-such-repo-here.git" }),
    )
    .unwrap();
    assert!(!out.contains("needs_auth"), "a typo is not an auth problem: {out}");
}
