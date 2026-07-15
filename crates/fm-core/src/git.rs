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

fn git(vault: &Path) -> Command {
    let mut c = Command::new("git");
    c.arg("-C").arg(vault);
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
