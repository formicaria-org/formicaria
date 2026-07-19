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

/// Mirrors [`crate::git::probe`]: ask a remote whether we could clone it, without cloning.
///
/// `create_detached` is the point — there is no repository yet when a user is typing a URL into
/// the new-vault form, and every other remote call here needs one. `connect` fetches the ref
/// advertisement and no objects.
pub fn probe(url: &str) -> crate::git::Probe {
    use crate::git::Probe;
    let mut rem = match git2::Remote::create_detached(url.trim()) {
        Ok(r) => r,
        Err(e) => return Probe::Unreachable(e.message().to_string()),
    };
    let cbs = credentials();
    // The connection borrows `rem`, so the outcome is reduced to a plain value inside this
    // block and the borrow ends before anything else touches the remote.
    let outcome = match rem.connect_auth(git2::Direction::Fetch, Some(cbs), None) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };
    match outcome {
        Ok(()) => {
            let _ = rem.disconnect();
            Probe::Reachable
        }
        // libgit2 reports a missing/refused credential as its own class, which is a far better
        // signal than the message text the subprocess backend has to match on.
        Err(e) if e.class() == git2::ErrorClass::Http || e.code() == git2::ErrorCode::Auth => {
            Probe::NeedsAuth
        }
        Err(e) => Probe::from_stderr(e.message()),
    }
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

/// The raw push, with no squash — the primitive [`push_squashed`] is built on.
///
/// Not the one the app calls: on a shared remote, pushing raw means every few seconds of typing
/// becomes a commit somebody else has to read. Kept separate because the squash needs a push it
/// can retry-and-roll-back around, and because a caller that genuinely wants "send what I have"
/// should have to say so.
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

/// The tracking ref's commit, or `None` when nothing of ours has ever been pushed.
fn tracking_oid(repo: &Repository) -> Option<(String, git2::Oid)> {
    let head = repo.head().ok()?;
    // git2 0.21 returns a `Result` here, not an `Option` — same shape as `unpushed` uses.
    let Ok(branch) = head.shorthand() else { return None };
    let branch = branch.to_string();
    let up = repo
        .find_branch(&format!("{}/{branch}", crate::git::REMOTE), git2::BranchType::Remote)
        .ok()?;
    let oid = up.get().target()?;
    Some((branch, oid))
}

/// Mirrors [`crate::git::newest_foreign`]: the newest unpushed commit this app did **not**
/// write, or `None` when every unpushed commit is ours.
///
/// The squash boundary, and the reason it is a *boundary* rather than a flag: collapsing an
/// `auto:` window is what this feature is for, but collapsing three hand-written commits into
/// one `backup:` is data loss of the quiet kind — the work survives, its shape does not.
///
/// **By message prefix, deliberately not by author** — we commit *as* the user, so an author
/// test classifies everything as ours. Identical rule to the subprocess backend, and
/// `git_differential.rs` is what keeps them from drifting apart.
fn newest_foreign(repo: &Repository, tracked: git2::Oid) -> Result<Option<git2::Oid>, StoreError> {
    let mut walk = repo.revwalk().map_err(map)?;
    walk.push_head().map_err(map)?;
    walk.hide(tracked).map_err(map)?;
    // Newest first, matching `git log`: the first foreign commit met walking back from HEAD is
    // the floor the squash must not go below.
    walk.set_sorting(git2::Sort::TOPOLOGICAL).map_err(map)?;
    for oid in walk {
        let oid = oid.map_err(map)?;
        let commit = repo.find_commit(oid).map_err(map)?;
        let subject = commit.summary().ok().flatten().unwrap_or("");
        if !subject.starts_with("auto:") && !subject.starts_with("backup:") {
            return Ok(Some(oid));
        }
    }
    Ok(None)
}

/// Mirrors [`crate::git::push_squashed`]: collapse the not-yet-pushed `auto:` window into one
/// commit and push it. Returns how many commits were squashed (0 when there was nothing to
/// squash, or on the first push).
///
/// **Every guard the subprocess version has, for the same reasons**, because this now runs on a
/// real remote that other machines share and the failure modes are not hypothetical:
///
/// - **Never squash the first push.** With no tracking ref, "unpushed" is the *entire* history,
///   and destroying history that has never left the machine is exactly backwards.
/// - **Only our commits.** `newest_foreign` is the floor; a hand-written commit stops the squash.
/// - **Only when the remote's tip is an ancestor of ours.** Otherwise someone else's commits are
///   on that ref, and resetting onto it would put *their* tip under *our* tree — a commit that
///   deletes their work and pushes as a clean fast-forward, which nothing rejects. A count of
///   unpushed commits cannot catch this: it is >0 in exactly the divergent case, so it reads as
///   normal. **Ancestry is the question; "how many" never was.**
/// - **Roll back if the push fails.** The squash is a bet that the push lands; losing the bet
///   must not also cost the user their granular undo.
/// - **Ask the remote, not the pusher.** On the desktop this is belt-and-braces because git's
///   exit code is trustworthy. *Here it is the point.* Push is spoken by libgit2 and its error
///   mapping is ours, so "success" is our own parser's opinion — and a **false** success is the
///   one failure this cannot survive, because the history was already collapsed on the strength
///   of it. This is the check the subprocess version's comment says exists "for the client that
///   replaces it". This is that client.
pub fn push_squashed(vault: &Path, message: &str) -> Result<u32, StoreError> {
    ensure_repo(vault)?;
    if remote(vault)?.is_none() {
        return Err(StoreError::Io(
            "no remote configured — set one to push your notes off this machine".into(),
        ));
    }
    let repo = Repository::open(vault).map_err(map)?;
    // Where history stood before we collapsed it — what we put back if the push does not land.
    let head_before = repo.head().ok().and_then(|h| h.target());

    let squashed = match tracking_oid(&repo) {
        None => 0, // the first push: send it whole
        Some((_branch, tracked)) => {
            let head = repo.head().map_err(map)?.target().ok_or_else(|| {
                StoreError::Io("this vault has no commits yet, so there is nothing to push".into())
            })?;
            // `graph_descendant_of` is false for an identical oid, which is the up-to-date case
            // and must count as "they are an ancestor of us" exactly as `--is-ancestor` does.
            let ours_contains_theirs =
                head == tracked || repo.graph_descendant_of(head, tracked).map_err(map)?;
            if !ours_contains_theirs {
                return Err(StoreError::Io(
                    "the remote has changes you don't have — pull first, then back up".into(),
                ));
            }
            let base = newest_foreign(&repo, tracked)?.unwrap_or(tracked);
            let (ahead, _) = repo.graph_ahead_behind(head, base).map_err(map)?;
            if ahead > 1 {
                let base_obj = repo.find_object(base, None).map_err(map)?;
                // Soft: HEAD moves, the index keeps every squashed change, so the commit below
                // reproduces exactly the same tree.
                repo.reset(&base_obj, git2::ResetType::Soft, None).map_err(map)?;

                // A net-zero window — write something, then undo it — leaves an index identical
                // to the base's tree. Committing that would be an empty commit; skipping it
                // leaves us already at the remote's state, and the push below is a no-op.
                let mut index = repo.index().map_err(map)?;
                let tree_id = index.write_tree().map_err(map)?;
                let base_commit = repo.find_commit(base).map_err(map)?;
                if tree_id != base_commit.tree_id() {
                    let sig = repo.signature().map_err(map)?;
                    let tree = repo.find_tree(tree_id).map_err(map)?;
                    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&base_commit])
                        .map_err(map)?;
                }
                ahead as u32
            } else {
                0
            }
        }
    };

    let rollback = |repo: &Repository| {
        if squashed > 0 {
            if let Some(h) = head_before {
                if let Ok(obj) = repo.find_object(h, None) {
                    let _ = repo.reset(&obj, git2::ResetType::Soft, None);
                }
            }
        }
    };

    if let Err(e) = push(vault) {
        rollback(&repo);
        return Err(e);
    }

    // The push says it worked. Ask the remote. See the doc comment: on this backend a false
    // success is the failure that costs the user history they already paid for.
    if squashed > 0 {
        if let (Ok(head_ref), Some((branch, _))) = (repo.head(), tracking_oid(&repo)) {
            if let Some(ours) = head_ref.target() {
                // A *definite* mismatch, and nothing else. If the remote cannot be asked — it
                // went away, or does not publish this branch — the answer is unknown, and
                // unknown must not roll back: undoing a push that actually landed leaves us
                // behind a remote that already has the work, and every later push is then
                // rejected as divergent. Failing to verify is not failing to push.
                if let Ok(Some(there)) = remote_head(&repo, &branch) {
                    if there != ours {
                        rollback(&repo);
                        return Err(StoreError::Io(format!(
                            "the push reported success but {} still points at {} — your history \
                             has been put back the way it was, and nothing was backed up",
                            crate::git::REMOTE,
                            &there.to_string()[..8]
                        )));
                    }
                }
            }
        }
    }
    Ok(squashed)
}

/// What the remote says a branch points at, straight from the wire — the `ls-remote` question.
/// `Ok(None)` when it does not publish that branch, which is an honest "cannot tell".
///
/// `connect` + `list` rather than a fetch: this asks for refs only and downloads no objects,
/// which is what makes it cheap enough to sit behind a poll on a phone's data connection.
fn remote_head(repo: &Repository, branch: &str) -> Result<Option<git2::Oid>, StoreError> {
    let mut rem = repo.find_remote(crate::git::REMOTE).map_err(map)?;
    let mut cbs = credentials();
    // Connect borrows the callbacks, so the connection is scoped tightly and always closed.
    rem.connect_auth(git2::Direction::Fetch, Some(std::mem::take(&mut cbs)), None).map_err(map)?;
    let wanted = format!("refs/heads/{branch}");
    let found = rem
        .list()
        .map_err(map)?
        .iter()
        .find(|h| h.name() == wanted)
        .map(|h| h.oid());
    let _ = rem.disconnect();
    Ok(found)
}

/// Mirrors [`crate::git::remote_moved`]: has someone else pushed since we last saw the remote?
/// `None` means "cannot tell" — offline, no remote, nothing pushed yet — and never an error,
/// because this runs on a timer and a phone that loses signal must not show a failure.
pub fn remote_moved(vault: &Path) -> Result<Option<bool>, StoreError> {
    if !vault.join(".git").exists() || remote(vault)?.is_none() {
        return Ok(None);
    }
    let repo = Repository::open(vault).map_err(map)?;
    let Some((branch, tracked)) = tracking_oid(&repo) else { return Ok(None) };
    match remote_head(&repo, &branch) {
        Ok(Some(there)) => Ok(Some(there != tracked)),
        // No such branch there yet: nothing has moved.
        Ok(None) => Ok(Some(false)),
        // Offline or refused. Not knowing is not an error.
        Err(_) => Ok(None),
    }
}

/// Mirrors [`crate::git::activity`]: who last touched each note.
///
/// **`since` is deliberately ignored here, and the result is a superset rather than a subset.**
/// The desktop passes git a human string (`"1 year ago"`) that only git's own approxidate parser
/// understands; libgit2 has no equivalent, and reimplementing that parser to *narrow* a result
/// would be work spent making the answer smaller. Since every note keeps only its newest touch,
/// walking further back can only fill in notes the desktop would have left blank — "last edited
/// by" for an old note, instead of nothing. Capped so a long history cannot stall a phone.
pub fn activity(vault: &Path, _since: &str) -> Result<Vec<crate::git::Touch>, StoreError> {
    use crate::git::Touch;
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    let repo = Repository::open(vault).map_err(map)?;
    let mut walk = repo.revwalk().map_err(map)?;
    if walk.push_head().is_err() {
        return Ok(Vec::new()); // a repo with no commits is "no history", not a failure
    }
    walk.set_sorting(git2::Sort::TIME).map_err(map)?;

    let mut seen = std::collections::HashSet::new();
    let mut touches = Vec::new();
    for oid in walk.take(2000) {
        let Ok(oid) = oid else { continue };
        let Ok(commit) = repo.find_commit(oid) else { continue };
        if commit.parent_count() > 1 {
            continue; // --no-merges: a merge's diff is not authorship
        }
        let Ok(tree) = commit.tree() else { continue };
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) else {
            continue;
        };
        let author = commit.author();
        let (name, email) = (
            author.name().unwrap_or_default().to_string(),
            author.email().unwrap_or_default().to_string(),
        );
        // Same wire format as the subprocess backend's `%aI`, so the two are comparable and the
        // UI's sort is stable across them.
        let time = format_iso(author.when());
        for delta in diff.deltas() {
            let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) else {
                continue;
            };
            let Some(id) = note_id_from_path(&path.to_string_lossy()) else { continue };
            // First seen wins, and the walk is newest-first — so this is the newest touch.
            if seen.insert(id.clone()) {
                touches.push(Touch {
                    id,
                    author: name.clone(),
                    email: email.clone(),
                    time: time.clone(),
                });
            }
        }
    }
    Ok(touches)
}

/// `notes/<ULID>.md` → `<ULID>`; blobs, manifest, `.view` files and anything nested → `None`.
/// Duplicated from [`crate::git`] rather than shared because it is four lines and the
/// differential test is what actually keeps the two backends honest.
fn note_id_from_path(path: &str) -> Option<String> {
    let stem = path.strip_prefix("notes/")?.strip_suffix(".md")?;
    (!stem.is_empty() && !stem.contains('/')).then(|| stem.to_string())
}

/// git2's `Time` as the `%aI` string git prints: `2026-07-19T17:43:52+08:00`.
///
/// Hand-rolled because the offset is minutes-from-UTC and the crate's own formatting does not
/// produce this shape. The format is load-bearing: the UI sorts these as strings.
fn format_iso(t: git2::Time) -> String {
    let offset_minutes = t.offset_minutes();
    let secs = t.seconds() + i64::from(offset_minutes) * 60;
    let (sign, off) = if offset_minutes < 0 { ('-', -offset_minutes) } else { ('+', offset_minutes) };
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        off / 60,
        off % 60
    )
}

/// Days since the Unix epoch → civil date. Howard Hinnant's `civil_from_days`, which is exact
/// for the whole proleptic Gregorian range and needs no calendar table.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
