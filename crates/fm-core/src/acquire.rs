//! Turning a directory that just arrived from somewhere into a vault this machine can use.
//!
//! # Why this module exists
//!
//! A vault is a directory of Markdown files, and git is a capability layered on top rather
//! than part of the definition ([`crate::git::available`] says so, and
//! `create_vault` deliberately never calls [`crate::git::ensure_repo`]). It is tempting to
//! conclude that any transport which copies a directory therefore moves a vault. **It does
//! not**, and the gap is this module.
//!
//! Vault state falls into three tiers:
//!
//! 1. **Portable** — `notes/` (wherever `vault.json` puts it), `blobs/`, `manifest.json`,
//!    `vault.json`, `.gitattributes`, `.gitignore`, and `.git` if the vault has history.
//!    This is what should travel.
//! 2. **Machine-local, and actively harmful when it travels** — `index.sqlite` (per-machine;
//!    the DB-corruption-by-sync lesson, stated at [`crate::backup`] and in the ignore rules),
//!    the `merge.fm` driver in `.git/config` (an *absolute* path to an `fm` binary that does
//!    not exist over here), and the committer identity in `.git/config` (which would sign
//!    this machine's commits as the sender).
//! 3. **Registration** — the entry in `vaults.json`, which lives in the OS config dir and is
//!    not inside the vault at all. That tier belongs to `fm-app` and is not this module's job.
//!
//! [`naturalise`] is tier 2. It is the one step every acquisition method shares, and keeping
//! it here — rather than in each transport — is what makes adding a transport safe: a new
//! method has to move bytes and nothing else, so it cannot invent a new way to corrupt a
//! vault. That is the whole reason the seam is shaped this way.
//!
//! # What this module is not
//!
//! It is not a transport registry and there is no `Transport` trait. Transports are ordinary
//! functions ([`crate::git::clone`], [`crate::backup::restore_vault`]) that put files in a
//! folder; the extension point is the filesystem, which is also why a folder someone synced
//! by other means is already adoptable with no code at all.

use crate::StoreError;
use std::path::Path;

/// What [`naturalise`] actually had to do, so a caller can tell the user rather than guess.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Naturalised {
    /// A per-machine index came with the copy and was removed. Always false for a `git clone`
    /// (it is gitignored) and for a restic restore (it is not snapshotted) — but true for any
    /// transport that moves the directory verbatim, which is the point of checking.
    pub dropped_index: bool,
    /// The sender's committer identity was in `.git/config` and has been cleared, so this
    /// machine will be asked who it is instead of silently signing as them.
    pub cleared_identity: bool,
    /// The directory has history, so it can pull and push. False means a working vault with
    /// no history — which is a real vault, just not a collaboration.
    pub has_history: bool,
}

/// Make a freshly-arrived directory safe to open as a vault on *this* machine.
///
/// # Only ever call this on something that just arrived
///
/// This clears state that is correct to clear for a copy and destructive to clear for a vault
/// someone has been using: in particular it drops `index.sqlite` and forgets the repo-local
/// committer identity. On a directory the user already works in, that would throw away a
/// warm index and the name on their commits for no reason. Acquisition paths only.
///
/// # What it does
///
/// - **Drops any `index.sqlite` that came along.** It is per-machine by design and rebuilt on
///   open (`FileStore::named` always reindexes), so removing it costs a reindex and avoids
///   the failure mode where a copy taken mid-write arrives corrupt and the vault will not
///   open with nothing but a bare `sqlite:` error to go on.
/// - **Forgets the sender's committer identity**, so this machine is asked who it is rather
///   than attributing its commits to whoever sent the vault. `.git/config` is per-machine —
///   git itself takes that view, which is why a clone does not carry it; only a verbatim
///   directory copy does, and that is exactly the case this handles.
/// - **Re-runs [`crate::git::ensure_repo`]** on a directory that has history, which re-points
///   the `merge.fm` driver at *this* machine's `fm` — or removes it when there is none. An
///   inherited driver naming a path that does not exist here is the silent-data-loss case
///   documented at `install_merge_driver`, and it is the single most important thing this
///   function prevents.
/// - **Creates `blobs/` and `derived/`**, so the ignore rules refer to something visible.
///
/// It deliberately does **not** `git init` a directory that has no history. A vault without
/// git is a first-class vault, and quietly making one a repo would also risk `git init`ing a
/// nested repo inside one the user already owns.
pub fn naturalise(vault: &Path) -> Result<Naturalised, StoreError> {
    let mut out = Naturalised::default();

    let index = vault.join("index.sqlite");
    if index.exists() {
        std::fs::remove_file(&index).map_err(|e| {
            StoreError::Io(format!(
                "could not remove the per-machine index that came with this copy ({}): {e}",
                index.display()
            ))
        })?;
        out.dropped_index = true;
    }

    out.has_history = vault.join(".git").exists();
    if out.has_history {
        out.cleared_identity = crate::git::forget_identity(vault);
        // Re-points the merge driver at this machine, or unsets it when there is no `fm`
        // here to point at. Also appends the ignore rules and the merge attribute if the
        // sender's copy predates them.
        crate::git::ensure_repo(vault)?;
    }

    for d in ["blobs", "derived"] {
        let p = vault.join(d);
        std::fs::create_dir_all(&p)
            .map_err(|e| StoreError::Io(format!("could not create {}: {e}", p.display())))?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn a_plain_folder_of_notes_becomes_a_vault_with_no_history() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();

        let out = naturalise(dir.path()).unwrap();

        assert!(!out.has_history, "nothing should have made this a repo");
        assert!(!dir.path().join(".git").exists(), "naturalise must never git init");
        assert!(dir.path().join("blobs").is_dir());
        assert!(dir.path().join("derived").is_dir());
    }

    /// The tier-2 case that matters: a verbatim directory copy brings the sender's index,
    /// and an index taken mid-write is the one that will not open.
    #[test]
    fn an_index_that_travelled_with_the_copy_is_dropped() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("index.sqlite"), b"not really a database").unwrap();

        let out = naturalise(dir.path()).unwrap();

        assert!(out.dropped_index);
        assert!(!dir.path().join("index.sqlite").exists());
    }

    #[test]
    fn naturalising_twice_is_the_same_as_once() {
        let dir = tempdir().unwrap();
        let first = naturalise(dir.path()).unwrap();
        let second = naturalise(dir.path()).unwrap();
        assert_eq!(first, second);
    }
}
