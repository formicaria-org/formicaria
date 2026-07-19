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

// ═══════════════════════════════════════════════════════════════════════════════════════════
// The sync half.
//
// **This is the part libgit2 cannot do for us, and the reason it needed its own argument.**
// libgit2 contains no process spawn at all, so it cannot invoke the `.md` merge driver that
// makes two people editing one note a non-event rather than a conflict on the `updated:` line.
// Porting `pull` naively would silently disable that — a collaborator running `git pull` in a
// terminal would still get the structural merge while the app quietly did a worse one. Two
// merge semantics in one vault, and ours the wrong one.
//
// So the app resolves conflicted paths itself, by calling [`crate::merge::merge_texts`] — the
// **same engine** the desktop driver calls through `fm merge-md`. One engine, two call sites,
// which is what makes them unable to diverge. That is the whole design, and it is why
// `merge_texts` had to be extracted from its path-shaped wrapper before any of this was safe.
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// Authentication for the network operations.
///
/// PAT over HTTPS, read from the environment. **The app stores no secret of its own** — on the
/// desktop that is the ambient credential helper, and here it is a value the shell supplies,
/// which on Android will come from the Keystore. Keeping it in the environment rather than in
/// a config file is the same stance `RESTIC_PASSWORD` already takes.
///
/// **Fails after one attempt.** libgit2 will call this callback in a loop while it keeps
/// getting credentials, so returning a bad token forever is a hang rather than an error — the
/// exact trap the mobile design flagged. The counter makes the second call an error.
fn credentials() -> git2::RemoteCallbacks<'static> {
    let mut cb = git2::RemoteCallbacks::new();
    let mut tried = 0u8;
    cb.credentials(move |_url, username, allowed| {
        tried += 1;
        if tried > 1 {
            return Err(git2::Error::from_str(
                "authentication failed — check the token for this remote",
            ));
        }
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let Ok(token) = std::env::var("FM_GIT_TOKEN") {
                if !token.is_empty() {
                    // A fine-grained PAT is the *password*; every host accepts any non-empty
                    // username alongside it, so the remote's own username wins when present.
                    return git2::Cred::userpass_plaintext(username.unwrap_or("x-access-token"), &token);
                }
            }
        }
        // No token: fall back to whatever the URL itself carries (a `file://` remote, or a
        // credential already baked into the URL). Anything else is an honest failure.
        git2::Cred::default()
    });
    cb
}

/// Mirrors [`crate::git::conflicts`]: the notes with an unfinished merge in them.
pub fn conflicts(vault: &Path) -> Result<Vec<String>, StoreError> {
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    let repo = Repository::open(vault).map_err(map)?;
    let index = repo.index().map_err(map)?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for c in index.conflicts().map_err(map)?.flatten() {
        // `our` is the side we keep the path from; a delete/modify conflict may have only one.
        if let Some(entry) = c.our.or(c.their).or(c.ancestor) {
            out.push(String::from_utf8_lossy(&entry.path).into_owned());
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Mirrors [`crate::git::unpushed`]: how many commits are ahead of the tracking ref, or `None`
/// when nothing has ever been pushed.
pub fn unpushed(vault: &Path) -> Result<Option<u32>, StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let Ok(head) = repo.head() else { return Ok(None) };
    let Ok(name) = head.shorthand() else { return Ok(None) };
    let Ok(upstream) = repo.find_branch(&format!("{}/{name}", crate::git::REMOTE), git2::BranchType::Remote)
    else {
        return Ok(None);
    };
    let (local_oid, up_oid) = match (head.target(), upstream.get().target()) {
        (Some(a), Some(b)) => (a, b),
        _ => return Ok(None),
    };
    let (ahead, _behind) = repo.graph_ahead_behind(local_oid, up_oid).map_err(map)?;
    Ok(Some(ahead as u32))
}

/// Mirrors [`crate::git::pull`]: fetch, then merge their work into ours.
///
/// The merge is where this backend earns its keep. Every conflicted path is handed to
/// [`crate::merge::merge_texts`] — frontmatter merged structurally, the body through the same
/// 3-way engine — so a note two people edited in different paragraphs comes back clean, exactly
/// as it does on a desktop. A note they genuinely disagreed about comes back with markers **in
/// the body**, which is what keeps it parseable and openable in the editor.
pub fn pull(vault: &Path) -> Result<crate::git::Pulled, StoreError> {
    use crate::git::Pulled;

    ensure_repo(vault)?;
    if remote(vault)?.is_none() {
        return Err(StoreError::Io(
            "no remote configured — set one before pulling anyone's work".into(),
        ));
    }
    if !conflicts(vault)?.is_empty() {
        return Err(StoreError::Io(
            "there is already a merge to finish here — resolve the conflicts first".into(),
        ));
    }

    let repo = Repository::open(vault).map_err(map)?;
    {
        let mut rem = repo.find_remote(crate::git::REMOTE).map_err(map)?;
        let mut opts = git2::FetchOptions::new();
        opts.remote_callbacks(credentials());
        // Empty refspec list = the remote's configured default, same as `git fetch origin`.
        rem.fetch(&[] as &[&str], Some(&mut opts), None).map_err(map)?;
    }

    let head = repo.head().map_err(map)?;
    let branch = head.shorthand().unwrap_or("main").to_string();
    let Ok(upstream) =
        repo.find_branch(&format!("{}/{branch}", crate::git::REMOTE), git2::BranchType::Remote)
    else {
        // Nothing of ours is up there yet, so there is nothing of theirs to merge into.
        return Ok(Pulled::UpToDate);
    };
    let their_oid = upstream.get().target().ok_or_else(|| StoreError::Io("remote ref has no target".into()))?;
    let our_oid = head.target().ok_or_else(|| StoreError::Io("HEAD has no target".into()))?;

    if repo.graph_descendant_of(our_oid, their_oid).map_err(map)? || our_oid == their_oid {
        // We already hold everything they have.
        return Ok(Pulled::UpToDate);
    }

    let (_ahead, behind) = repo.graph_ahead_behind(our_oid, their_oid).map_err(map)?;
    let incoming = behind as u32;

    let their_commit = repo.find_commit(their_oid).map_err(map)?;
    let our_commit = repo.find_commit(our_oid).map_err(map)?;
    let annotated = repo.find_annotated_commit(their_oid).map_err(map)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).map_err(map)?;

    if analysis.is_fast_forward() {
        // Nothing of ours to preserve: move the branch and check their tree out.
        let mut r = repo.find_reference(&format!("refs/heads/{branch}")).map_err(map)?;
        r.set_target(their_oid, "pull: fast-forward").map_err(map)?;
        repo.set_head(&format!("refs/heads/{branch}")).map_err(map)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(map)?;
        return Ok(Pulled::Merged(0));
    }

    // A real merge.
    //
    // `repo.merge` rather than `merge_trees`: it merges into the **repository** index and
    // working tree, which is what real `git merge` leaves behind — a repo-backed index whose
    // conflicts survive for `conflicts()` to report, and files on disk a human can edit.
    // `merge_trees` hands back a standalone in-memory index instead, which cannot be written,
    // cannot be `add_path`'d into ("Index is not backed up by an existing repository"), and
    // would leave a conflicted pull invisible to every later call.
    repo.merge(&[&annotated], None, None).map_err(map)?;

    let mut index = repo.index().map_err(map)?;
    let mut unresolved: Vec<String> = Vec::new();

    if index.has_conflicts() {
        let items: Vec<_> = index.conflicts().map_err(map)?.flatten().collect();
        for c in items {
            // `as_ref`, not `clone`: git2 0.21's `IndexEntry` is not `Clone`, and the path
            // bytes are all that is needed.
            let Some(our_entry) = c.our.as_ref().or(c.their.as_ref()).or(c.ancestor.as_ref())
            else {
                continue;
            };
            let path = String::from_utf8_lossy(&our_entry.path).into_owned();
            let blob = |e: &Option<git2::IndexEntry>| -> String {
                e.as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| String::from_utf8_lossy(b.content()).into_owned())
                    .unwrap_or_default()
            };
            let (base_txt, our_txt, their_txt) = (blob(&c.ancestor), blob(&c.our), blob(&c.their));

            // THE call. Same engine as `fm merge-md`, so a phone and a desktop cannot disagree
            // about what a merged note is.
            let (merged, outcome) = crate::merge::merge_texts(&base_txt, &our_txt, &their_txt, 7)?;

            let full = vault.join(&path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::write(&full, &merged).map_err(io)?;

            if outcome == crate::merge::Merged::Clean {
                // Clear the three conflict stages and stage the resolution, which is exactly
                // what `git add` does after a human fixes a conflict by hand.
                index.conflict_remove(Path::new(&path)).map_err(map)?;
                index.add_path(Path::new(&path)).map_err(map)?;
            } else {
                // Left conflicted on purpose: the markers are in the body, the note still
                // parses and still opens, and a human decides. Reported, never swallowed —
                // and the index keeps the conflict so `conflicts()` still sees it.
                unresolved.push(path);
            }
        }
        index.write().map_err(map)?;
    }

    if !unresolved.is_empty() {
        unresolved.sort();
        // MERGE_HEAD is deliberately left in place, exactly as an interrupted `git merge` does:
        // the next `pull` refuses rather than starting a second merge over an unfinished one.
        return Ok(Pulled::Conflicted(unresolved));
    }

    // Everything resolved: record the merge with both parents so history says what happened.
    let tree_oid = index.write_tree().map_err(map)?;
    let tree = repo.find_tree(tree_oid).map_err(map)?;
    let sig = repo.signature().map_err(map)?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("merge {}/{branch}", crate::git::REMOTE),
        &tree,
        &[&our_commit, &their_commit],
    )
    .map_err(map)?;
    // Clears MERGE_HEAD; without it the repo stays "mid-merge" forever and the next pull
    // refuses.
    repo.cleanup_state().map_err(map)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).map_err(map)?;
    Ok(Pulled::Merged(incoming))
}

/// Mirrors [`crate::git::push_squashed`]'s *push* half — without the squash.
///
/// **The squash is deliberately absent.** Collapsing history is a destructive optimisation that
/// exists because the desktop auto-commits every few seconds; it is guarded by an ancestry check
/// and a rollback that verifies the remote ref actually moved. Reimplementing that here, on the
/// backend that has never run against a real remote, is exactly the "half-shipped on the path
/// that must never corrupt" the design forbids. A phone pushes what it has.
pub fn push(vault: &Path) -> Result<(), StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let head = repo.head().map_err(map)?;
    let branch = head.shorthand().map_err(|_| StoreError::Io("detached HEAD".into()))?.to_string();

    let mut rem = repo.find_remote(crate::git::REMOTE).map_err(map)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(credentials());
    rem.push(&[format!("refs/heads/{branch}:refs/heads/{branch}")], Some(&mut opts))
        .map_err(map)?;

    // `push -u`'s other half: without a tracking ref the next `unpushed` has nothing to compare
    // against and reports "never pushed" forever.
    let mut b = repo.find_branch(&branch, git2::BranchType::Local).map_err(map)?;
    let _ = b.set_upstream(Some(&format!("{}/{branch}", crate::git::REMOTE)));
    Ok(())
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
