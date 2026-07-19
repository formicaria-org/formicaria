//! In-process git, for platforms with no `git` binary to shell out to.
//!
//! **Why this exists at all, and why it is not the default.** Verified on a real device
//! (`docs/context/sessions/2026-07-19-the-core-on-the-phone.md`): Android ships no `git`, and
//! `command -v git` on the owner's phone answers `inaccessible or not found`. iOS forbids
//! executing one outright. So the subprocess stance in [`crate::git`] — which is what keeps
//! *"git is a capability, not a dependency"* true, and what keeps the `.md` merge driver
//! working for a collaborator's terminal `git pull` — simply cannot work there.
//!
//! Behind the non-default `native-git` feature. The desktop still shells out, unchanged.
//!
//! # The rule this module lives under
//!
//! **Every function here must be indistinguishable from its [`crate::git`] twin**, and that is
//! not an aspiration — `tests/git_differential.rs` runs both against the same repository and
//! compares. The same discipline the body-merge engine is held to
//! (`tests/merge_differential.rs`), for the same reason: two implementations of the sync path
//! that disagree is the corruption this project exists to prevent, and "it looked right" is how
//! that ships.
//!
//! # What is deliberately absent
//!
//! `pull` and the merge path. libgit2 **cannot invoke an external merge driver** — it contains
//! no process spawn at all — so a native pull must resolve conflicted paths itself by calling
//! [`crate::merge::merge_texts`], the same engine the desktop driver calls. That is a larger
//! piece with its own correctness argument, and shipping it half-done on the one path that must
//! never corrupt is exactly the move `decisions.md` forbids. This slice covers repository
//! setup, identity, remotes and committing: everything M0–M2 needs, and nothing that merges.

use crate::StoreError;
use std::path::Path;

use git2::Repository;

/// Mirrors [`crate::git::available`]. Always true once compiled in — the library is linked, so
/// there is nothing to detect.
///
/// **This is the reversal to keep an eye on.** On the desktop `available()` answers "is there a
/// git binary", and callers use it to decide whether history is possible at all. Here it can
/// only ever say yes, so a caller asking it to mean "can this vault have history" is asking the
/// wrong question — the honest one is whether an identity and a remote are configured. Callers
/// were left pointing at the desktop meaning on purpose: changing them is a separate, visible
/// change, not something this module should do quietly.
pub fn available() -> bool {
    true
}

fn map(e: git2::Error) -> StoreError {
    StoreError::Io(e.message().to_string())
}

/// Mirrors [`crate::git::ensure_repo`]: initialise if needed, and make it a *vault* — the
/// ignore rules and the merge attribute — returning true only when the repo was created here.
///
/// Probes `.git` directly rather than asking libgit2 to discover a repository, for the same
/// reason the subprocess version does not use `git rev-parse`: discovery walks *upward* and
/// would report the parent repo when a vault sits inside one, then happily write our ignore
/// rules into someone else's project.
pub fn ensure_repo(vault: &Path) -> Result<bool, StoreError> {
    let created = if vault.join(".git").exists() {
        false
    } else {
        Repository::init(vault).map_err(map)?;
        true
    };
    // Append-never-skip, exactly as the subprocess path does: a repo that already has a
    // `.gitignore` (i.e. every real one) must still gain these lines, or the auto-commit
    // sweeps `blobs/` and `index.sqlite` into history and the next push ships every PDF.
    crate::git::write_vault_files(vault)?;
    ensure_identity(vault);
    Ok(created)
}

/// Give a brand-new vault the placeholder committer when the machine has none.
///
/// **Found by writing the differential test, not by reading the code.** The subprocess
/// `ensure_repo` does this and the first version here did not — so a native commit on a fresh
/// repo failed for want of a signature, on a platform where there is no global git config to
/// fall back on and therefore *never* an identity. Exactly the class of divergence two
/// implementations of one path produce, and exactly why they are compared rather than trusted.
///
/// libgit2 *does* read `~/.gitconfig` — measured, and identical to the subprocess backend, so
/// there is no divergence here to worry about on a desktop. The point is the phone: there is no
/// global config there, so without this a fresh vault has no signature and cannot commit at all.
/// Written repo-locally and only when absent, so a real identity is never overwritten.
fn ensure_identity(vault: &Path) {
    let Ok(repo) = Repository::open(vault) else { return };
    let Ok(mut cfg) = repo.config() else { return };
    if cfg.get_string("user.email").is_ok() {
        return;
    }
    let _ = cfg.set_str("user.name", crate::git::PLACEHOLDER_NAME);
    let _ = cfg.set_str("user.email", crate::git::PLACEHOLDER_EMAIL);
}

/// Mirrors [`crate::git::identity`]. `None` when nobody real signs this vault — including the
/// placeholder, which is a sentinel and not a person.
pub fn identity(vault: &Path) -> Option<crate::git::Identity> {
    let repo = Repository::open(vault).ok()?;
    let cfg = repo.config().ok()?;
    let name = cfg.get_string("user.name").ok()?;
    let email = cfg.get_string("user.email").ok()?;
    crate::git::identity_from(name, email)
}

/// Mirrors [`crate::git::set_identity`], including its validation — an empty field, a value
/// without `@`, or the placeholder itself are all refused. The validation is shared rather than
/// re-typed: two copies of a rule are two rules.
pub fn set_identity(vault: &Path, name: &str, email: &str) -> Result<(), StoreError> {
    let (name, email) = crate::git::check_identity(name, email)?;
    ensure_repo(vault)?;
    let repo = Repository::open(vault).map_err(map)?;
    let mut cfg = repo.config().map_err(map)?;
    cfg.set_str("user.name", &name).map_err(map)?;
    cfg.set_str("user.email", &email).map_err(map)?;
    Ok(())
}

/// Mirrors [`crate::git::remote`]: the URL of the one remote we manage, or `None`.
pub fn remote(vault: &Path) -> Result<Option<String>, StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    // Bound to a local: the `Remote` borrows the `Repository`, so returning straight out of
    // the `match` would drop the repo while the borrow is still live.
    let url = match repo.find_remote(crate::git::REMOTE) {
        // git2 0.21's `url()` is a `Result`, not an `Option` — `Err` means the URL is not
        // valid UTF-8, which is a different thing from having no remote. Both surface as
        // `None` because every caller asks the same question ("where does this push to, if
        // anywhere?") and neither answer is a URL — but they are folded deliberately, not by
        // an `ok()` that quietly hides one inside the other.
        Ok(r) => r.url().ok().map(str::to_string),
        // Not an error: "no remote yet" is the ordinary state of a vault nobody shares.
        Err(_) => None,
    };
    Ok(url)
}

/// Mirrors [`crate::git::set_remote`], **including its refusal to give a vault an audience
/// while nobody real signs it.** That guard is the whole provenance model: a shared vault on
/// the placeholder attributes everyone's commits to one fake person, and history is not
/// something you fix afterwards.
pub fn set_remote(vault: &Path, url: &str) -> Result<(), StoreError> {
    crate::git::check_remote_allowed(vault, url, identity(vault).is_some())?;
    let repo = Repository::open(vault).map_err(map)?;
    // Idempotent, last URL wins — same as `git remote set-url` after `git remote add`.
    match repo.find_remote(crate::git::REMOTE) {
        Ok(_) => repo.remote_set_url(crate::git::REMOTE, url).map_err(map)?,
        Err(_) => {
            repo.remote(crate::git::REMOTE, url).map_err(map)?;
        }
    }
    Ok(())
}

/// Mirrors [`crate::git::commit_all`]: stage **exactly** the given paths and commit them,
/// returning false — not an error — when there is nothing of ours to commit.
///
/// **Scoped staging is load-bearing, not tidiness.** A vault may also be a repo holding source
/// or a manuscript; `add -A` here means the five-second debounce commits a file the user was
/// hand-editing in Vim, mid-sentence. libgit2 makes this easier than the subprocess does — the
/// index is an object we add explicit paths to — but the *rule* is the same and must not drift.
pub fn commit_all(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
) -> Result<bool, StoreError> {
    ensure_repo(vault)?;
    let repo = Repository::open(vault).map_err(map)?;

    let mut index = repo.index().map_err(map)?;
    let mut staged = 0usize;
    for p in paths {
        // Index paths are repo-relative; an absolute path silently matches nothing.
        let rel = p.strip_prefix(vault).unwrap_or(p);
        if p.exists() {
            index.add_path(rel).map_err(map)?;
            staged += 1;
        } else if index.get_path(rel, 0).is_some() {
            // Deleted by us since the last commit — the removal is the change.
            index.remove_path(rel).map_err(map)?;
            staged += 1;
        }
    }
    // The vault files `ensure_repo` writes travel with the repo and must be committed, or a
    // collaborator's clone arrives without the merge attribute.
    for f in [".gitignore", ".gitattributes"] {
        if vault.join(f).exists() {
            index.add_path(Path::new(f)).map_err(map)?;
        }
    }
    if staged == 0 && repo.head().is_ok() {
        // Nothing of ours moved. The common case for a debounced auto-commit, and it must not
        // surface as a failure.
        return Ok(false);
    }
    index.write().map_err(map)?;
    let tree_id = index.write_tree().map_err(map)?;
    let tree = repo.find_tree(tree_id).map_err(map)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    // An unchanged tree is not a commit. Without this a quiet vault accretes empty commits
    // every debounce, which is the same "nothing to commit" case the subprocess path reports
    // as false.
    if let Some(p) = &parent {
        if p.tree_id() == tree_id {
            return Ok(false);
        }
    }

    let sig = repo.signature().map_err(map)?;
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).map_err(map)?;
    Ok(true)
}

/// Mirrors [`crate::git::clone`]: clone, then make the result a vault.
///
/// Calls [`ensure_repo`] itself for the same reason the subprocess version does — the
/// `.gitattributes` merge attribute travels but the driver *definition* deliberately does not,
/// and leaving that to the caller is how it gets forgotten. On this backend there is no driver
/// to install at all, which is precisely why a native `pull` will have to call
/// [`crate::merge::merge_texts`] directly.
pub fn clone(url: &str, dest: &Path) -> Result<(), StoreError> {
    crate::git::check_clone_dest(url, dest)?;
    Repository::clone(url.trim(), dest).map_err(map)?;
    ensure_repo(dest)?;
    Ok(())
}
