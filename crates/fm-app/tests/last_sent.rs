//! **The alert that measures saving cannot see sending.**
//!
//! The owner, 2026-09-08: *"I had no icon saying that I had any uncommitted/unbacked notes."* They
//! were right, and the gap was six days and 125 changes wide — their vault last reached GitHub on
//! 2026-09-02 and went on committing locally every few minutes throughout.
//!
//! Nothing was broken. Every always-visible alert in the app measures **local liveness**, and the
//! auto-commit loop is what keeps local liveness perfect: `unrecorded` is `git status`, which a
//! successful commit empties; the quiet chip is `git log -1`, which a successful commit resets to
//! zero; and the "someone sent changes" alert compares the remote against the tracking ref, so it
//! is silent by construction about *our own* unsent work. The one number that was abnormal —
//! how much has never left this device — was computed only inside `backup_status`, the app's one
//! network call, and rendered only inside the Backup panel, which the owner had no reason to open
//! *because nothing had told them to*.
//!
//! **The fix is packaging, not new data.** `unpushed` never needed the network: it counts against
//! `refs/remotes/origin/<branch>`, which is on disk. So both halves of the fact belong on
//! `last_commits` — the arm that is already scoped, already cheap, already per-vault, and already
//! feeds the only alert that reports an *absence*. This is the argument `dispatch.rs` already
//! makes verbatim for `identity`, which was put on `VaultInfo` for the same reason.
//!
//! Proven red before the fields existed.

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

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn call(app: &App, cmd: &str) -> serde_json::Value {
    let out = dispatch(cmd, &serde_json::json!({}), &[], app, &NoHost)
        .unwrap_or_else(|e| panic!("{cmd}: {e}"));
    serde_json::from_slice(&out.into_bytes()).unwrap_or(serde_json::Value::Null)
}

fn note(at: &Path, n: usize) {
    let id = format!("01AAAAAAAAAAAAAAAAAAAAAA{n:02}");
    std::fs::create_dir_all(at.join("notes")).unwrap();
    std::fs::write(
        at.join(format!("notes/{id}.md")),
        format!("---\nschema: 1\nid: {id}\ntype: note\ntitle: n{n}\ncreated: 2026-07-21T00:00:00Z\nupdated: 2026-07-21T00:00:00Z\n---\nbody\n"),
    )
    .unwrap();
}

fn commit(at: &Path, n: usize, when: &str) {
    note(at, n);
    git(at, &["add", "-A"]);
    let msg = format!("c{n}");
    let out = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["commit", "-m", &msg])
        .env("GIT_COMMITTER_DATE", when)
        .env("GIT_AUTHOR_DATE", when)
        .env("GIT_AUTHOR_NAME", "T")
        .env("GIT_AUTHOR_EMAIL", "t@example.org")
        .env("GIT_COMMITTER_NAME", "T")
        .env("GIT_COMMITTER_EMAIL", "t@example.org")
        .output()
        .unwrap();
    assert!(out.status.success(), "commit: {}", String::from_utf8_lossy(&out.stderr));
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

/// `last_commits` reports **both** clocks, and they must be able to disagree.
///
/// The assertion that matters is `last_sent != last_commit`: a test where a vault is fully sent
/// would pass with `last_sent` wired to the wrong ref, or to `HEAD`, and prove nothing. So the
/// fixture is deliberately the owner's situation in miniature — sent once, then committed again —
/// and the two instants are pinned weeks apart so no clock luck can make them agree.
#[test]
fn last_commits_says_when_notes_last_left_the_device_and_how_much_has_not() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let home = tempdir().unwrap();
    let dir = home.path().join("v");
    std::fs::create_dir_all(&dir).unwrap();
    git(home.path(), &["init", "-q", "-b", "main", "v"]);

    commit(&dir, 0, "2026-01-02T03:04:05+0530");

    // Sent once, at this commit — the tracking ref is planted rather than pushed, because what is
    // under test is reading it, not git's transport.
    let sent_at = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["update-ref", "refs/remotes/origin/main", &sent_at]);

    // …and then two more days of writing that never left.
    commit(&dir, 1, "2026-03-04T05:06:07+0000");
    commit(&dir, 2, "2026-03-05T05:06:07+0000");

    let sent: i64 = git(&dir, &["log", "-1", "--format=%ct", &sent_at]).parse().unwrap();
    let saved: i64 = git(&dir, &["log", "-1", "--format=%ct"]).parse().unwrap();
    assert_ne!(sent, saved, "the fixture must make the two clocks disagree, or it proves nothing");

    let app = app_over(&home, &[("v".into(), dir.clone())]);
    let rows = call(&app, "last_commits");
    let row = &rows.as_array().expect("an array of vaults")[0];

    assert_eq!(row["vault"], "v");
    assert_eq!(row["last_commit"].as_i64(), Some(saved), "when anything was last saved here");
    assert_eq!(row["last_sent"].as_i64(), Some(sent), "when notes last left this device");
    assert_eq!(row["unsent"].as_u64(), Some(2), "…and how much has not left since");
}

/// **Never sent is not "sent long ago".** A vault that has never reached anywhere must report
/// `null`, not an enormous age — otherwise the first vault a person makes greets them with the
/// loudest alert in the app, for having done nothing wrong. `unsent` is `null` for the same state
/// and for the same reason: there is nothing to count against.
#[test]
fn a_vault_that_has_never_been_sent_reports_no_moment_rather_than_a_huge_one() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let home = tempdir().unwrap();
    let dir = home.path().join("fresh");
    std::fs::create_dir_all(&dir).unwrap();
    git(home.path(), &["init", "-q", "-b", "main", "fresh"]);
    commit(&dir, 0, "2026-01-02T03:04:05+0530");

    let app = app_over(&home, &[("fresh".into(), dir.clone())]);
    let rows = call(&app, "last_commits");
    let row = &rows.as_array().unwrap()[0];

    assert!(row["last_commit"].as_i64().is_some(), "it has certainly been saved");
    assert!(row["last_sent"].is_null(), "never sent is null, not an age");
    assert!(row["unsent"].is_null(), "and there is nothing to count against");
}
