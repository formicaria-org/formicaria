//! **`fm backup` / `fm restore` / `fm check` — the three commands nobody had ever tested.**
//!
//! `outstanding.md` §2.10 listed this among the backup tier's residues, and it is the sharper half
//! of the pair: `fm-core`'s `backup.rs` has six real-restic tests, so the *library* is covered — but
//! the CLI arms that wrap it were never executed by anything. They are three lines each, which is
//! exactly the shape of code that gets a flag name wrong and nobody notices, because the failure is
//! a backup that did not happen rather than one that crashed.
//!
//! **They drive the real binary against a real restic repository**, like `cli.rs` does, because the
//! point is the wiring: argument parsing, the password reaching restic, the exit code, and the line
//! printed to a human. A test against the library would prove none of that.
//!
//! Skipped rather than failed when restic is absent — the house rule, so a contributor without it
//! still gets a useful run. `ci/checks.sh` now refuses to *gate* without restic, so the gate cannot
//! quietly cover this file.
//!
//! Both tests serialise on `restic_cache`: `RESTIC_CACHE_DIR` is process-global, and the first
//! version of this file did not, which reproduced the exact race documented in `known-issues.md`
//! within minutes of it being written down.

use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const PASSWORD: &str = "correct horse battery staple";

fn fm(vault: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_fm"));
    c.arg("--vault").arg(vault);
    c
}

/// **`RESTIC_CACHE_DIR` is process-global, and both tests below set it.**
///
/// Without this they race: one points it at a `TempDir` the other is about to drop, and restic then
/// fails on a cache directory that vanished underneath it —
/// `mkdir …/data/67: no such file or directory`, which reads like a permissions problem and is not.
/// Caught here on the first run, in a file whose own header had just claimed one test was enough.
/// Third instance of this trap in the repo; see `known-issues.md`.
static CACHE_ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the guard and give restic a cache directory this test owns. Hold **both** returns: dropping
/// the directory while restic is still running is the bug above.
fn restic_cache() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = CACHE_ENV.lock().unwrap_or_else(|e| e.into_inner());
    let cache = tempdir().unwrap();
    std::env::set_var("RESTIC_CACHE_DIR", cache.path());
    (guard, cache)
}

fn have_restic() -> bool {
    Command::new("restic").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// The whole tier, through the CLI: back up, check, restore, and read the bytes back.
///
/// One test rather than three, deliberately — `restore` has nothing to restore until `backup` has
/// run, and splitting them would mean either three restic repositories or a shared fixture between
/// tests that must then be ordered. The failure messages name which step went wrong.
#[test]
fn backup_check_and_restore_round_trip_through_the_cli() {
    if !have_restic() {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();

    // A note, through the CLI, so the vault is one the app would recognise.
    let out = fm(vault.path()).args(["capture", "the durable knowledge"]).output().unwrap();
    assert!(out.status.success(), "capture failed: {}", String::from_utf8_lossy(&out.stderr));

    // ---- backup -------------------------------------------------------------------------
    // **`repo` is positional, not `--repo`.** Discovered by this test on its first run, which is
    // the point of having it: three-line CLI arms are exactly where a flag name goes unchecked.
    let out = fm(vault.path())
        .arg("backup")
        .arg(repo.path())
        .args(["--password", PASSWORD])
        .output()
        .unwrap();
    assert!(out.status.success(), "fm backup failed: {}", String::from_utf8_lossy(&out.stderr));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("backed up"), "it must say what it did: {said}");

    // ---- check --------------------------------------------------------------------------
    // `--read-data` re-reads and re-hashes every pack, which is the off-site scrub — the whole
    // reason this command exists rather than trusting that a write succeeded.
    let out = fm(vault.path())
        .arg("check")
        .arg(repo.path())
        .args(["--password", PASSWORD, "--read-data"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "fm check --read-data failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("data re-read"));

    // ---- restore ------------------------------------------------------------------------
    let dest = tempdir().unwrap();
    // Both paths positional, in order: `fm restore <repo> <dest>`.
    let out = fm(vault.path())
        .arg("restore")
        .arg(repo.path())
        .arg(dest.path())
        .args(["--password", PASSWORD])
        .output()
        .unwrap();
    assert!(out.status.success(), "fm restore failed: {}", String::from_utf8_lossy(&out.stderr));

    // **And the bytes came back.** A restore that exits 0 and produces nothing is the failure this
    // whole tier exists to prevent, so the assertion reads a file rather than a status.
    let mut found = None;
    let mut stack = vec![dest.path().to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("md") {
                let text = std::fs::read_to_string(&p).unwrap();
                if text.contains("the durable knowledge") {
                    found = Some(p);
                }
            }
        }
    }
    assert!(found.is_some(), "the restored tree holds no note with the original text");
}

/// **A wrong password must fail loudly.** The one failure mode that matters here: restic refusing
/// to open the repository has to reach the user as a non-zero exit, not as a cheerful line. A
/// backup command that reports success on a repository it could not open is worse than no command.
#[test]
fn a_wrong_password_is_an_error_not_a_cheerful_line() {
    if !have_restic() {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    fm(vault.path()).args(["capture", "a note"]).output().unwrap();
    fm(vault.path())
        .arg("backup")
        .arg(repo.path())
        .args(["--password", PASSWORD])
        .output()
        .unwrap();

    let out = fm(vault.path())
        .arg("check")
        .arg(repo.path())
        .args(["--password", "not the password"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a wrong password must not exit 0");
}
