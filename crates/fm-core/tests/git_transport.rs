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

fn have_git() -> bool {
    std::process::Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// One test, not several: it manipulates process-wide environment, so it must not race a sibling
/// running in another thread of the same binary.
#[test]
fn ext_transport_is_refused_even_when_the_users_own_config_allows_it() {
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
