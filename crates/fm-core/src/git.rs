//! Version the vault with git (subprocess) — the same "shell out to the tool
//! that already solves this" stance as [`crate::backup`]. Git is the durable
//! safety net: the notes are plain text, so a full history of every edit lives in
//! an ordinary repo the user can push to GitHub. Heavy blobs and the disposable
//! index are kept out (they sync out-of-band), so the notes repo stays small and
//! clonable forever.
//!
//! The vault is its **own** repo, independent of the application's source tree —
//! detected by a real `.git` under the vault, never by `git rev-parse` (which
//! would walk up and find a parent repo when the vault sits inside one).

use crate::StoreError;
use std::path::Path;
use std::process::{Command, Output};

/// The one remote we manage. Git's own default name, so a vault stays an
/// ordinary repo that behaves as expected in a terminal.
const REMOTE: &str = "origin";

fn git(vault: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C").arg(vault);
    // Never let git stop to ask a human. fm-serve is thread-per-connection and
    // has no TTY, so a credential or host-key prompt would hang the request
    // forever rather than fail. Authentication is whatever the user's ssh-agent
    // or credential helper already provides — this app holds no secret of its own.
    c.env("GIT_TERMINAL_PROMPT", "0");
    c.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    c
}

/// Initialize the vault as its own git repo if it isn't one yet, giving it a
/// committer identity and a `.gitignore` that keeps the per-machine index,
/// regenerable thumbnails, and heavy blobs out of history. Returns true if the
/// repo was created.
pub fn ensure_repo(vault: &Path) -> Result<bool, StoreError> {
    // A repo root always has a `.git` entry (dir or, for worktrees, a file). This
    // is the *only* correct probe here: `git rev-parse` walks upward and would
    // report the parent repo when the vault is nested inside one.
    if vault.join(".git").exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(vault).map_err(io)?;
    let out = git(vault).arg("init").output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git init", &out));
    }
    ensure_identity(vault);
    write_gitignore(vault)?;
    Ok(true)
}

/// Set a repo-local committer identity when none is configured, so the first
/// commit never fails on a fresh machine with no global git config. Repo-local,
/// so it never touches the user's global settings. Best-effort.
fn ensure_identity(vault: &Path) {
    let missing = |k: &str| {
        !git(vault).arg("config").arg(k).output().map(|o| o.status.success()).unwrap_or(false)
    };
    if missing("user.email") {
        let _ = git(vault).args(["config", "user.email", "formicarium@localhost"]).output();
    }
    if missing("user.name") {
        let _ = git(vault).args(["config", "user.name", "formicarium"]).output();
    }
}

fn write_gitignore(vault: &Path) -> Result<(), StoreError> {
    let path = vault.join(".gitignore");
    if path.exists() {
        return Ok(());
    }
    // index.sqlite is per-machine and must never travel (the DB-corruption-by-sync
    // lesson); derived/ thumbnails regenerate; blobs sync out-of-band, never here.
    std::fs::write(&path, "index.sqlite\nderived/\nblobs/\n").map_err(io)
}

/// Stage everything and commit. Returns false — not an error — when the working
/// tree is already clean; "nothing to commit" is the common case for a debounced
/// auto-commit and must not surface as a failure.
pub fn commit_all(vault: &Path, message: &str) -> Result<bool, StoreError> {
    ensure_repo(vault)?;
    let status = git(vault).arg("status").arg("--porcelain").output().map_err(spawn)?;
    if !status.status.success() {
        return Err(failed("git status", &status));
    }
    if status.stdout.is_empty() {
        return Ok(false);
    }
    let add = git(vault).arg("add").arg("-A").output().map_err(spawn)?;
    if !add.status.success() {
        return Err(failed("git add", &add));
    }
    let out = git(vault).arg("commit").arg("-m").arg(message).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git commit", &out));
    }
    Ok(true)
}

/// Where the vault pushes to, or None when no remote is configured yet. Absence
/// is the normal state of a fresh vault, never an error.
pub fn remote(vault: &Path) -> Result<Option<String>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(None);
    }
    let out = git(vault).args(["remote", "get-url", REMOTE]).output().map_err(spawn)?;
    if !out.status.success() {
        return Ok(None); // "no such remote" is absence, not failure
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((!url.is_empty()).then_some(url))
}

/// Point `origin` at `url`, creating it when it doesn't exist yet. Idempotent,
/// so the panel can simply save whatever the user typed.
pub fn set_remote(vault: &Path, url: &str) -> Result<(), StoreError> {
    // `git remote add origin ""` *succeeds*, and the resulting remote then
    // reports its own name as its URL — so an empty value would leave the vault
    // claiming a push destination it does not have. Refuse it here: a backup that
    // lies about where it went is the one failure worth being strict about.
    let url = url.trim();
    if url.is_empty() {
        return Err(StoreError::Io("a remote needs a URL".into()));
    }
    ensure_repo(vault)?;
    let sub = if remote(vault)?.is_some() { "set-url" } else { "add" };
    let out = git(vault).args(["remote", sub, REMOTE]).arg(url).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git remote", &out));
    }
    Ok(())
}

fn branch(vault: &Path) -> Result<String, StoreError> {
    let out = git(vault).args(["rev-parse", "--abbrev-ref", "HEAD"]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-parse", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The remote-tracking ref for the current branch — `Some` only once we have
/// pushed at least once. `None` therefore means "never pushed", which is the
/// signal [`push_squashed`] uses to refuse to squash.
///
/// This ref is updated only by fetch and push, and **nothing here ever fetches**.
/// That is deliberate: it means a remote another machine has moved stays ahead of
/// our stale ref, so our push is rejected and the user is told. Fetch first and
/// the squash below would rebase onto their tip and silently overwrite their
/// content with our tree.
fn tracking(vault: &Path) -> Result<Option<String>, StoreError> {
    // An unborn HEAD (no commits yet) has no branch to track.
    let Ok(b) = branch(vault) else { return Ok(None) };
    let r = format!("refs/remotes/{REMOTE}/{b}");
    let ok = git(vault)
        .args(["rev-parse", "--verify", "--quiet", &r])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    Ok(ok.then_some(r))
}

fn count_ahead(vault: &Path, base: &str) -> Result<u32, StoreError> {
    let out =
        git(vault).args(["rev-list", "--count", &format!("{base}..HEAD")]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git rev-list", &out));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0))
}

/// How many commits exist here but not on the remote. `None` when there is
/// nothing to compare against (no repo, no commits, or never pushed) — the
/// caller shows a count only when there is a real one.
pub fn unpushed(vault: &Path) -> Result<Option<u32>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(None);
    }
    match tracking(vault)? {
        None => Ok(None),
        Some(base) => Ok(Some(count_ahead(vault, &base)?)),
    }
}

/// Collapse the not-yet-pushed commits into one and push. Returns how many were
/// squashed (0 when there was nothing to squash, or on the first push).
///
/// Auto-commit produces a `auto:` commit every few seconds of editing; without
/// this the remote would accrue thousands of them. Squashing costs the granular
/// undo for the squashed window — after a push you can only step back to that
/// push — which is the accepted trade (see docs/context/decisions.md).
pub fn push_squashed(vault: &Path, message: &str) -> Result<u32, StoreError> {
    ensure_repo(vault)?;
    if remote(vault)?.is_none() {
        return Err(StoreError::Io(
            "no remote configured — set one to push your notes off this machine".into(),
        ));
    }
    let squashed = match tracking(vault)? {
        // The first push. Here "unpushed" means the *entire* history, and
        // destroying history that has never left the machine is exactly
        // backwards — send it as it stands. Every later push collapses to one.
        None => 0,
        Some(base) => {
            let n = count_ahead(vault, &base)?;
            if n > 1 {
                // --soft moves HEAD alone: the index still holds every squashed
                // change, so the commit below reproduces the same tree.
                let out = git(vault).args(["reset", "--soft", &base]).output().map_err(spawn)?;
                if !out.status.success() {
                    return Err(failed("git reset", &out));
                }
                // A net-zero window (write something, then undo it) leaves an
                // index identical to the remote's tree. Committing that would
                // fail and strand HEAD mid-squash, so skip it: we are already at
                // the remote's state and the push below is simply a no-op.
                let status = git(vault).args(["status", "--porcelain"]).output().map_err(spawn)?;
                if !status.stdout.is_empty() {
                    let out =
                        git(vault).arg("commit").arg("-m").arg(message).output().map_err(spawn)?;
                    if !out.status.success() {
                        return Err(failed("git commit", &out));
                    }
                }
                n
            } else {
                0
            }
        }
    };
    // -u so the tracking ref exists next time, which is what makes the squash
    // above possible at all.
    let out = git(vault).args(["push", "-u", REMOTE, "HEAD"]).output().map_err(spawn)?;
    if !out.status.success() {
        return Err(failed("git push", &out));
    }
    Ok(squashed)
}

fn spawn(e: std::io::Error) -> StoreError {
    StoreError::Io(format!("could not run git (is it installed?): {e}"))
}

fn failed(what: &str, out: &Output) -> StoreError {
    let stderr = String::from_utf8_lossy(&out.stderr);
    StoreError::Io(format!("{what} failed: {}", stderr.trim()))
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
