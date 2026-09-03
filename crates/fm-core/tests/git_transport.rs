//! **A remote URL is caller-supplied text that we hand to `git`.**
//!
//! `probe_remote` runs `git ls-remote <url>` on every keystroke while someone types a remote into
//! the backup panel, and `clone_vault` reaches the same transports. Git's `ext::` transport exists
//! to run a command — `ext::sh -c whoami` is a URL.
//!
//! **Git's own default already refuses it** (`protocol.ext.allow=never`; measured on 2.53, a bare
//! `ls-remote ext::…` fails with "transport 'ext' not allowed"). So this is not a live hole, and
//! the test below does not pretend otherwise — a test asserting "a default URL does not execute"
//! would pass with or without our guard and prove nothing, which is exactly what the first draft
//! of this file did.
//!
//! What is actually ours to guarantee is that the refusal does not depend on the *user's* config.
//! Someone with `protocol.ext.allow=always` in `~/.gitconfig` — or `GIT_CONFIG_*` in the
//! environment, which is how this test simulates it — would otherwise hand command execution to a
//! string typed into a form. That is the property tested here, and it fails without the `-c` flag
//! in `git_cmd()`.

use std::path::PathBuf;
use std::sync::Mutex;

fn have_git() -> bool {
    std::process::Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// **Serialises every test in this file, because one of them edits the process environment.**
///
/// `std::env::set_var`/`remove_var` are process-global while cargo runs a binary's tests on
/// concurrent threads, so `ext_transport_is_refused…` and `ordinary_remotes_still_work` raced:
/// the second would call `git init` in the window where `GIT_CONFIG_COUNT=1` and `GIT_CONFIG_KEY_0`
/// were still set but `GIT_CONFIG_VALUE_0` had already been removed, and git refuses that with
/// `missing config value GIT_CONFIG_VALUE_0 / fatal: unable to parse command-line config`.
/// **Measured 2026-09-03: 1 failure in 8 default runs, 0 in 4 with `--test-threads=1`.**
///
/// This file's header used to hold the invariant as a note — *"One test, not several"* — and a
/// second test was added anyway, which is the failure mode a comment has and a lock does not.
///
/// Poisoning is recovered from rather than propagated: a failing assertion in one test should
/// report *that* assertion, not turn every sibling into an unwrap panic on a poisoned mutex.
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn ext_transport_is_refused_even_when_the_users_own_config_allows_it() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let marker: PathBuf = dir.path().join("executed");

    // Make the *user's* configuration as permissive as it can be. `GIT_CONFIG_*` is read exactly
    // like a config file, so this is a faithful stand-in for a line in `~/.gitconfig`.
    std::env::set_var("GIT_CONFIG_COUNT", "1");
    std::env::set_var("GIT_CONFIG_KEY_0", "protocol.ext.allow");
    std::env::set_var("GIT_CONFIG_VALUE_0", "always");

    // `%` is how `ext::` escapes the space between the command and its argument.
    let url = format!("ext::sh -c touch% {}", marker.display());
    let outcome = fm_core::git::probe(&url);

    // The assertion is a **side effect that must not happen**, rather than an error string:
    // messages vary by git version, and matching on one is how a test starts passing for the
    // wrong reason.
    let executed = marker.exists();

    std::env::remove_var("GIT_CONFIG_COUNT");
    std::env::remove_var("GIT_CONFIG_KEY_0");
    std::env::remove_var("GIT_CONFIG_VALUE_0");

    assert!(
        !executed,
        "a URL executed a command: `git_cmd()` is missing `-c protocol.ext.allow=never`, so the \
         refusal was left to the user's own git config. Probe returned: {outcome:?}"
    );
    assert!(
        !matches!(outcome, fm_core::git::Probe::Reachable),
        "a command-execution URL must never read as a reachable remote: {outcome:?}"
    );
}

/// The guard must not have been bought by breaking ordinary use. Cloning from a local path is a
/// supported way to acquire a vault, so `file`/local transports have to keep working.
#[test]
fn ordinary_remotes_still_work() {
    // Takes the same lock: this test reads no `GIT_CONFIG_*` itself, but `git init` does, and a
    // half-removed set from the sibling is what broke it.
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    fm_core::git::ensure_repo(dir.path()).unwrap();

    assert!(
        matches!(
            fm_core::git::probe(&dir.path().display().to_string()),
            fm_core::git::Probe::Reachable
        ),
        "a real local repository must still probe as reachable"
    );
    assert!(
        !matches!(
            fm_core::git::probe("https://example.invalid/nope.git"),
            fm_core::git::Probe::Reachable
        ),
        "an unreachable remote is still unreachable"
    );
}
