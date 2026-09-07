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
    commit_all_inner(vault, message, paths, None)
}

/// Mirrors [`crate::git::commit_all_as`]: the same commit, attributed to `(name, email)` — the
/// study agent's **model** identity, so its own messages are authored by it rather than by
/// whoever owns the vault (Ruling 14's provenance label, not a trust boundary).
///
/// **Why this exists at all, given `vcs::commit_all_as` used to call the subprocess
/// unconditionally.** That choice was made on the premise that `native-git` was "a
/// differential-test feature". It is not: it is the phone's shipped backend. So on a phone the
/// call reached `Command::new("git")` and ENOENTed — swallowed by a `let _ =` at the call site,
/// which is why nobody saw it. The reply still landed, committed by the later debounced batch,
/// but **under the vault's default identity** — the one thing this function exists to prevent.
pub fn commit_all_as(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
    name: &str,
    email: &str,
) -> Result<bool, StoreError> {
    commit_all_inner(vault, message, paths, Some((name, email)))
}

fn commit_all_inner(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
    author: Option<(&str, &str)>,
) -> Result<bool, StoreError> {
    ensure_repo(vault)?;
    // **A conflicted path is never staged, and it never stops the rest.** Mirrors the subprocess
    // backend, which is the only reason both halves of this are here.
    //
    // Staging one would be the disaster: `index.add_path` below stages whatever is on disk and
    // clears that path's conflict stages, so a debounced auto-commit five seconds after a
    // conflicting pull would commit the note **with its `<<<<<<<` markers as content**, drop the
    // merge's second parent, and push it. This backend had no such guard until the differential test
    // found it (2026-07-31) — the two backends were wrong in opposite directions on the same path.
    //
    // Refusing *wholesale* was the other half of the same mistake, and it is the one that cost 39
    // days on the owner's phone: two stuck notes stopped 202 from being committed, 196 of them brand
    // new and no part of any merge (`decisions.md`, 2026-09-07). So the conflicted paths are dropped
    // from what we commit and the rest goes through.
    let blocked: std::collections::HashSet<String> =
        conflicted(vault)?.into_iter().map(|c| c.path).collect();
    let repo = Repository::open(vault).map_err(map)?;

    let mut index = repo.index().map_err(map)?;
    let mut staged = 0usize;
    let mut ours: Vec<std::path::PathBuf> = Vec::new();
    for p in paths {
        // Index paths are repo-relative; an absolute path silently matches nothing.
        let rel = p.strip_prefix(vault).unwrap_or(p);
        if blocked.contains(rel.to_string_lossy().as_ref()) {
            continue;
        }
        if p.exists() {
            index.add_path(rel).map_err(map)?;
            staged += 1;
            ours.push(rel.to_path_buf());
        } else if index.get_path(rel, 0).is_some() {
            // Deleted by us since the last commit — the removal is the change.
            index.remove_path(rel).map_err(map)?;
            staged += 1;
            ours.push(rel.to_path_buf());
        }
    }
    // The vault files `ensure_repo` writes travel with the repo and must be committed, or a
    // collaborator's clone arrives without the merge attribute.
    for f in [".gitignore", ".gitattributes"] {
        if vault.join(f).exists() && !blocked.contains(f) {
            index.add_path(Path::new(f)).map_err(map)?;
            ours.push(Path::new(f).to_path_buf());
        }
    }
    if staged == 0 && repo.head().is_ok() {
        // Nothing of ours moved. The common case for a debounced auto-commit, and it must not
        // surface as a failure.
        return Ok(false);
    }
    index.write().map_err(map)?;
    if !blocked.is_empty() {
        // A conflict entry makes `write_tree` fail outright — that is libgit2's `GIT_EUNMERGED` and
        // it is not negotiable. The tree is assembled elsewhere instead; the index write above is
        // what keeps the real index in lockstep with the `HEAD` that call moves.
        return commit_around_a_conflict(&repo, vault, message, &ours, author);
    }
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

    // The same helper the proposal path uses, so an authored commit and an authored proposal
    // cannot disagree about what attribution means. Author and committer are both set, matching
    // the four `GIT_*` environment variables the subprocess backend exports.
    let sig = signature(&repo, author)?;
    // **A merge in flight is committed as a merge**, with `MERGE_HEAD` as the second parent, and the
    // state cleared afterwards. A single-parent commit here would silently drop the incoming side
    // from history, and a `MERGE_HEAD` left standing freezes the vault permanently: `push_squashed`
    // and `merge_proposal_branch` both refuse over an unfinished merge, as does the next `pull`. On a
    // phone — no shell, no `git` binary, no file manager — that is unrecoverable from inside the app.
    let merging = merge_head(vault)?.and_then(|oid| repo.find_commit(oid).ok());
    let mut parents: Vec<&git2::Commit> = parent.iter().collect();
    if let Some(their) = &merging {
        parents.push(their);
    }
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).map_err(map)?;
    if merging.is_some() {
        repo.cleanup_state().map_err(map)?;
    }
    Ok(true)
}

/// Commit `ours` **around** an unfinished merge, leaving every conflicted path exactly where it is.
///
/// The twin of `crate::git::commit_around_a_conflict`, and held to it by `tests/git_differential.rs`.
/// Read that one first: it carries the argument, and this carries only what libgit2 does differently.
///
/// **Which is: there is no temporary index file, there is a scratch index in memory.** `Index::new`
/// makes one that belongs to no repository, `read_tree` fills it from `HEAD`, and entries are added
/// by blob id — `add_path` is unavailable on an index with no repository behind it (libgit2 says so
/// in as many words: *"Index is not backed up by an existing repository"*), which is the same
/// constraint `merged_text` documents for a `merge_trees` index. `write_tree_to` then writes the
/// tree into the real object database. The real index is never touched here; the caller already
/// staged these paths into it, which is what keeps it in lockstep with the `HEAD` this moves.
///
/// **Single-parent, deliberately.** `commit_all_inner`'s ordinary path writes `MERGE_HEAD` as a
/// second parent and calls `cleanup_state`, because reaching it means the merge is *finished*.
/// Reaching here means it is not: claiming otherwise would end the merge with conflicts still in the
/// index, and `finish_merge_if_resolved` — which is what actually completes it — would never run.
fn commit_around_a_conflict(
    repo: &Repository,
    vault: &Path,
    message: &str,
    ours: &[std::path::PathBuf],
    author: Option<(&str, &str)>,
) -> Result<bool, StoreError> {
    let Ok(head) = repo.head().and_then(|h| h.peel_to_commit()) else {
        // No commit yet, so there is no merge in flight either — nothing this path can do.
        return Ok(false);
    };
    let head_tree = head.tree().map_err(map)?;

    let mut scratch = git2::Index::new().map_err(map)?;
    scratch.read_tree(&head_tree).map_err(map)?;
    for rel in ours {
        let full = vault.join(rel);
        if !full.exists() {
            let _ = scratch.remove_path(rel);
            continue;
        }
        let id = repo.blob_path(&full).map_err(map)?;
        // Keep the mode git already recorded for this path, so an executable file does not quietly
        // become a regular one; a path `HEAD` has never seen is an ordinary file.
        let mode = head_tree.get_path(rel).map(|e| e.filemode() as u32).unwrap_or(0o100644);
        let size = std::fs::metadata(&full).map(|m| m.len() as u32).unwrap_or(0);
        scratch
            .add(&git2::IndexEntry {
                // The stat fields are what git uses to skip re-hashing an unchanged file. This index
                // is written once and thrown away, so zeros cost nothing and invent nothing.
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode,
                uid: 0,
                gid: 0,
                file_size: size,
                id,
                flags: 0,
                flags_extended: 0,
                path: rel.to_string_lossy().into_owned().into_bytes(),
            })
            .map_err(map)?;
    }

    let tree_id = scratch.write_tree_to(repo).map_err(map)?;
    if tree_id == head_tree.id() {
        // Nothing of ours actually differs from HEAD — the debounced auto-commit's common case, and
        // it must not accrete an empty commit every five seconds for the length of a conflict.
        return Ok(false);
    }
    let tree = repo.find_tree(tree_id).map_err(map)?;
    let sig = signature(repo, author)?;
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head]).map_err(map)?;
    Ok(true)
}

/// The other side of an unfinished merge, if there is one. One reader, so the two callers
/// (`commit_all` and `finish_merge_if_resolved`) cannot disagree about what "mid-merge" means.
fn merge_head(vault: &Path) -> Result<Option<git2::Oid>, StoreError> {
    let path = vault.join(".git/MERGE_HEAD");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
    git2::Oid::from_str(text.trim()).map(Some).map_err(|e| StoreError::Io(e.to_string()))
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
    // **`Repository::clone` cannot be used here.** It builds its own default fetch options with
    // no callbacks, so a private remote fails with libgit2's "remote authentication required but
    // no callback set" — which reads like a missing token even when one is configured. Every
    // other network call in this module attached credentials; clone was the one that did not,
    // and a `file://` differential test cannot catch it because a local path never authenticates.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options());
    builder.clone(url.trim(), dest).map_err(map)?;
    ensure_repo(dest)?;
    Ok(())
}

/// Fetch options carrying this machine's credentials — **the only way any code here should
/// build them.**
///
/// Exists so the callback cannot be left off by accident: `clone` shipped without one and every
/// other operation had it, which is exactly the shape a per-call-site decision produces. Anything
/// that fetches goes through this.
fn fetch_options<'a>() -> git2::FetchOptions<'a> {
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(credentials());
    fo
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
                    return git2::Cred::userpass_plaintext(
                        username.unwrap_or("x-access-token"),
                        &token,
                    );
                }
            }
        }
        // **Say what is actually wrong.** `Cred::default()` was the fallback here, and it is a
        // NTLM/Negotiate credential — useless to a forge wanting HTTP basic auth. libgit2 then
        // reports "remote authentication required but no callback set", which points at missing
        // plumbing when the real answer is "nobody gave me a token". Measured: the callback runs,
        // is handed `USER_PASS_PLAINTEXT`, returns a default credential, and produces that
        // message anyway — an hour of debugging the wrong layer.
        //
        // `Cred::default()` is still right where the transport genuinely wants it, so it is kept
        // for everything that is not a username/password ask.
        if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            return Err(git2::Error::from_str(
                "this remote needs an access token, and none is configured on this device",
            ));
        }
        git2::Cred::default()
    });
    cb
}

/// libgit2's `GIT_OPT_ADD_SSL_X509_CERT`, by ordinal.
///
/// Not exposed by `libgit2-sys`, which binds the enum only as far as
/// `GIT_OPT_SET_SSL_CERT_LOCATIONS`. Counted from `include/git2/common.h` in the vendored
/// source: it is the 46th entry, so 45 zero-indexed. Asserted at runtime by checking the call's
/// return rather than trusted — a wrong constant would silently configure something else.
///
/// Gated to match [`add_certs_from_pem`], its only user: on Windows and iOS libgit2 speaks WinHTTP
/// and SecureTransport, so neither this constant nor the call it names exists there.
#[cfg(not(any(windows, target_os = "ios")))]
const GIT_OPT_ADD_SSL_X509_CERT: libc_int = 45;

#[allow(non_camel_case_types)]
#[cfg(not(any(windows, target_os = "ios")))]
type libc_int = i32;

/// Load CA certificates into libgit2's trust store **from memory**, never from a file.
///
/// # Why this exists, and why the obvious approach cannot work
///
/// `openssl-src` builds OpenSSL with `no-stdio` on every Android target. Without stdio there is
/// no `BIO_s_file`, so `BIO_new(BIO_s_file())` returns NULL and `X509_load_cert_file` raises
/// `X509_R_BIO_LIB` — which is precisely what a device reported after several rounds of
/// producing better and better bundle files:
///
/// ```text
/// error:05880020:x509 certificate routines::BIO lib
/// [path=… size=217790 mode=600 readable=true head="-----BEGIN CERTIFICATE-----"]
/// ```
///
/// A correct, readable, well-formed file that OpenSSL cannot open, in the same process that
/// wrote it. **So every file-based route is structurally unavailable here** — `SSL_CERT_FILE`,
/// `SSL_CERT_DIR` and `GIT_OPT_SET_SSL_CERT_LOCATIONS` all end in a file BIO.
///
/// `GIT_OPT_ADD_SSL_X509_CERT` does not. It takes an `X509 *` and calls `X509_STORE_add_cert` on
/// libgit2's own context. Parsing PEM out of a `&[u8]` uses a *memory* BIO, which `no-stdio`
/// leaves entirely intact — so the whole path touches no file.
///
/// Returns how many certificates the store accepted. A certificate the store rejects is skipped
/// rather than fatal: a trust store with 144 of 145 roots is useful, and one that refused to
/// build because a single entry was odd would be a regression on the file it replaces.
///
/// **Not compiled on Windows.** There, libgit2 speaks WinHTTP and trusts the system store, so the
/// two raw `-sys` crates this needs are not dependencies for that target at all — see the
/// `[target.'cfg(not(windows))'.dependencies]` note in `Cargo.toml`, which is what unbroke the
/// Windows release build.
#[cfg(not(any(windows, target_os = "ios")))]
pub fn add_certs_from_pem(pem: &[u8]) -> Result<usize, StoreError> {
    if pem.is_empty() {
        return Err(StoreError::Io("the CA bundle was empty".into()));
    }
    // **libgit2 must be initialised before this option is touched, or the app dies.**
    //
    // `git_openssl__add_x509_cert` does `SSL_CTX_get_cert_store(git__ssl_ctx)` after an
    // `openssl_ensure_initialized()` that is `return 0` in a non-`GIT_OPENSSL_DYNAMIC` build —
    // so it creates nothing. Reach it before `git_libgit2_init()` has run and `git__ssl_ctx` is
    // still NULL, which dereferences null inside OpenSSL: a hard crash on launch, not an error
    // code. Observed exactly that way on a device.
    //
    // The `git2` wrappers all call the crate's `init()` first; calling `git_libgit2_opts` raw
    // skips it, which is the cost of using an option `git2` does not wrap. Opening a path that
    // cannot exist is the cheapest way to run that same initialisation: it fails, harmlessly,
    // *after* libgit2 and its SSL context are up.
    let _ = git2::Repository::open(Path::new("/nonexistent/formicaria-libgit2-init"));

    let mut added = 0usize;
    let mut parsed = 0usize;

    // Safety: a memory BIO over a slice that outlives this block, read with the standard PEM
    // loop. Every certificate we obtain is freed here; `X509_STORE_add_cert` takes its own
    // reference, so libgit2 keeps the ones it accepts alive independently of ours.
    unsafe {
        let bio =
            openssl_sys::BIO_new_mem_buf(pem.as_ptr() as *const std::ffi::c_void, pem.len() as i32);
        if bio.is_null() {
            return Err(StoreError::Io("could not open a memory BIO for the CA bundle".into()));
        }
        loop {
            let cert = openssl_sys::PEM_read_bio_X509(
                bio,
                std::ptr::null_mut(),
                None,
                std::ptr::null_mut(),
            );
            if cert.is_null() {
                break; // end of the bundle, or an entry we cannot parse
            }
            parsed += 1;
            if libgit2_sys::git_libgit2_opts(GIT_OPT_ADD_SSL_X509_CERT, cert) == 0 {
                added += 1;
            }
            openssl_sys::X509_free(cert);
        }
        openssl_sys::BIO_free_all(bio);
        // The loop always ends on a PEM read that failed, which leaves an entry on OpenSSL's
        // error queue. Left there, it would surface as a spurious cause on the *next* unrelated
        // failure — the kind of misdirection that has already cost this bug several rounds.
        openssl_sys::ERR_clear_error();
    }

    if added == 0 {
        return Err(StoreError::Io(format!(
            "libgit2 accepted none of the {parsed} certificates parsed from the bundle"
        )));
    }
    Ok(added)
}

/// Point libgit2's OpenSSL at a CA bundle, explicitly.
///
/// # The ordering rule, corrected
///
/// An earlier version of this comment claimed libgit2 creates its OpenSSL context lazily on
/// first stream use. **That is false**, and believing it cost several debugging rounds.
/// `libgit2-sys`'s `build.rs` never defines `GIT_OPENSSL_DYNAMIC` (it emits only `GIT_OPENSSL`
/// off Windows/Apple), so `git_openssl_stream_global_init` takes the `#ifndef` branch and calls
/// `openssl_init()` **eagerly, inside `git_libgit2_init()`** — and `openssl_ensure_initialized`
/// is `return 0` with no context creation at all.
///
/// Two consequences:
///
/// - The context always exists by the time any git2 API can be reached, so retrying this later
///   is a no-op. There was never a window to close.
/// - `SSL_CTX_set_default_verify_paths` — and therefore `SSL_CERT_FILE` — is read **once**,
///   during that eager init. Setting the variable afterwards has no effect, and
///   `X509_STORE_set_default_paths` calls `ERR_clear_error()` and returns success regardless,
///   so a bundle that fails to load there fails **completely silently**.
///
/// Which is why this explicit call is the one that matters: it routes to
/// `SSL_CTX_load_verify_locations`, which actually reports whether the bundle loaded. Its
/// result must be surfaced, never discarded.
///
/// Only meaningful where libgit2 does the talking; a machine with a `git` binary uses the
/// system's own store and never reaches this.
pub fn set_cert_file(path: &Path) -> Result<(), StoreError> {
    // Safety: called once at startup, before any network operation and before other threads
    // exist. `git2::opts` mutates libgit2's process-global TLS context, which is exactly what
    // is wanted here and why the function is `unsafe`.
    unsafe { git2::opts::set_ssl_cert_file(path) }.map_err(map)
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

/// Mirrors [`crate::git::conflicted`]: every unmerged path **with its kind**, so a surface can tell
/// the one kind that has editable markers from the kinds where the only resolution is a side.
///
/// The kind comes from which of the three index stages exist, which is what git's two-letter codes
/// are derived from anyway: no `our` stage means we deleted it, no `their` stage means they did, and
/// an absent ancestor with both sides present is a both-added.
pub fn conflicted(vault: &Path) -> Result<Vec<crate::git::Conflict>, StoreError> {
    use crate::git::{Conflict, ConflictKind};
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    let repo = Repository::open(vault).map_err(map)?;
    let index = repo.index().map_err(map)?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }
    let mut out: Vec<Conflict> = Vec::new();
    for c in index.conflicts().map_err(map)?.flatten() {
        let Some(entry) = c.our.as_ref().or(c.their.as_ref()).or(c.ancestor.as_ref()) else {
            continue;
        };
        let path = String::from_utf8_lossy(&entry.path).into_owned();
        let kind = match (c.ancestor.is_some(), c.our.is_some(), c.their.is_some()) {
            (_, true, true) if c.ancestor.is_some() => ConflictKind::BothModified,
            (false, true, true) => ConflictKind::BothAdded,
            (true, false, true) => ConflictKind::DeletedByUs,
            (true, true, false) => ConflictKind::DeletedByThem,
            (true, false, false) => ConflictKind::BothDeleted,
            (false, true, false) => ConflictKind::AddedByUs,
            (false, false, true) => ConflictKind::AddedByThem,
            // No stage at all cannot happen — the `let Some(entry)` above needed one — but the
            // compiler is right to insist, and "both modified" is the conservative answer: it is the
            // kind whose resolution is *editing the note*, which never destroys a side.
            (true, true, true) | (false, false, false) => ConflictKind::BothModified,
        };
        out.push(Conflict { path, kind });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

/// Mirrors [`crate::git::resolve_conflict`] — keep a side, then finish the merge if that was the
/// last unmerged path (for the same reason: `commit_all` would otherwise leave `MERGE_HEAD` in place
/// with nothing left to resolve, and a `MERGE_HEAD` is what makes it refuse to commit).
///
/// In-index rather than by subprocess: staging *is* removing the path's conflict stages and writing
/// the chosen blob at stage 0, which is what `git add`/`git rm` do underneath.
pub fn resolve_conflict(vault: &Path, rel: &str, keep: crate::git::Keep) -> Result<(), StoreError> {
    use crate::git::{ConflictKind::*, Keep::*};
    let conflict = conflicted(vault)?
        .into_iter()
        .find(|c| c.path == rel)
        .ok_or_else(|| StoreError::Io(format!("{rel} is not in conflict")))?;
    let repo = Repository::open(vault).map_err(map)?;
    let mut index = repo.index().map_err(map)?;
    // `Edited` — the reconciliation on disk is the answer. Same guard as the subprocess backend, from
    // the same predicate: staging a still-marked file is what git reads as "resolved", and it would
    // publish `<<<<<<<` as the note's content.
    if keep == Edited {
        let full = vault.join(rel);
        let text = std::fs::read_to_string(&full)
            .map_err(|e| StoreError::Io(format!("{}: {e}", full.display())))?;
        if crate::merge::has_conflict_markers(&text) {
            return Err(StoreError::Io(format!(
                "{rel} still has conflict markers — remove them and keep the text you want, or choose a side"
            )));
        }
        let path = std::path::Path::new(rel);
        index.remove_path(path).map_err(map)?;
        index.add_path(path).map_err(map)?;
        index.write().map_err(map)?;
        return finish_merge_if_resolved(vault);
    }
    let entry = index
        .conflicts()
        .map_err(map)?
        .flatten()
        .find(|c| {
            let p = c.our.as_ref().or(c.their.as_ref()).or(c.ancestor.as_ref());
            p.map(|e| String::from_utf8_lossy(&e.path) == rel).unwrap_or(false)
        })
        .ok_or_else(|| StoreError::Io(format!("{rel} is not in conflict")))?;

    // `IndexEntry` is not `Clone` in git2 0.21, and the borrow ends with `entry` — so take the one
    // thing needed downstream, the blob id of the chosen side.
    let wanted: Option<git2::Oid> = match keep {
        Theirs => entry.their.as_ref().map(|e| e.id),
        Mine => entry.our.as_ref().map(|e| e.id),
        // Unreachable: `Edited` returned above. An error, not a panic — same reasoning as the
        // subprocess backend, and doubly so here, where a panic is invisible on a phone.
        Edited => return Err(StoreError::Io(format!("{rel}: 'edited' is handled before this"))),
    };
    let path = std::path::Path::new(rel);
    // Clear the conflict stages first: every resolution starts by saying "this path is settled".
    index.remove_path(path).map_err(map)?;
    match (conflict.kind, wanted) {
        // Nothing to keep on the chosen side: the resolution is that the note is gone.
        (BothDeleted, _) | (_, None) => {
            let _ = std::fs::remove_file(vault.join(rel));
        }
        // Keep a real blob: write it to disk *and* stage it, so the working tree and the index agree
        // — a staged blob with different bytes on disk is the shape that makes the next status lie.
        (_, Some(oid)) => {
            let blob = repo.find_blob(oid).map_err(map)?;
            let full = vault.join(rel);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|err| StoreError::Io(err.to_string()))?;
            }
            // Temp + rename in the same directory, the vault's own write discipline: a note must
            // never be observable half-written, and a resolution is a note write like any other.
            let tmp = full.with_extension("md.resolve");
            std::fs::write(&tmp, blob.content()).map_err(|err| StoreError::Io(err.to_string()))?;
            std::fs::rename(&tmp, &full).map_err(|err| StoreError::Io(err.to_string()))?;
            index.add_path(path).map_err(map)?;
        }
    }
    index.write().map_err(map)?;
    finish_merge_if_resolved(vault)
}

/// Commit the merge once nothing is unmerged. No-op while conflicts remain or no merge is in flight.
///
/// Two parents, from `HEAD` and `MERGE_HEAD`, so history records that a merge happened — the same
/// shape `pull` writes when it resolves everything itself. Then `cleanup_state`, which is the line
/// whose absence leaves a vault "mid-merge" forever with every commit refused.
fn finish_merge_if_resolved(vault: &Path) -> Result<(), StoreError> {
    if !conflicted(vault)?.is_empty() || !vault.join(".git/MERGE_HEAD").exists() {
        return Ok(());
    }
    let repo = Repository::open(vault).map_err(map)?;
    let Some(their_oid) = merge_head(vault)? else { return Ok(()) };
    let their_commit = repo.find_commit(their_oid).map_err(map)?;
    let our_commit = repo.head().and_then(|h| h.peel_to_commit()).map_err(map)?;
    let mut index = repo.index().map_err(map)?;
    let tree_oid = index.write_tree().map_err(map)?;
    let tree = repo.find_tree(tree_oid).map_err(map)?;
    let sig = repo.signature().map_err(map)?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "merge: conflicts resolved",
        &tree,
        &[&our_commit, &their_commit],
    )
    .map_err(map)?;
    repo.cleanup_state().map_err(map)?;
    Ok(())
}

/// Mirrors [`crate::git::unrecorded`]: notes on disk that git does not have, or has differently.
///
/// Same purpose and the same scoping to the vault's own notes directory. `STATUS_OPT` asks for
/// untracked files individually (not collapsed to their directory), which is the difference between
/// naming 95 notes and naming `notes/`.
pub fn unrecorded(
    vault: &Path,
    notes_rel: &str,
) -> Result<Vec<crate::git::UnrecordedNote>, StoreError> {
    use crate::git::{UnrecordedKind, UnrecordedNote};
    if !vault.join(".git").exists() {
        return Ok(Vec::new());
    }
    let repo = Repository::open(vault).map_err(map)?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false);
    opts.pathspec(notes_rel);
    let statuses = repo.statuses(Some(&mut opts)).map_err(map)?;
    let prefix = format!("{}/", notes_rel.trim_end_matches('/'));
    let mut out: Vec<UnrecordedNote> = Vec::new();
    for s in statuses.iter() {
        let st = s.status();
        if st.is_conflicted() {
            continue; // a conflict is not an unrecorded note; it has its own surface
        }
        let p = s.path().unwrap_or("");
        if !(p.starts_with(&prefix) && p.ends_with(".md")) {
            continue;
        }
        // The same three answers the porcelain codes give, from the flags they are derived from — and
        // in the same order of precedence, so the two backends cannot disagree about a file that is
        // both (a staged add later deleted on disk reads as `New` on both). A one-sided mislabel here
        // is what `fm-cli/tests/conflict_resolution_both_devices.rs` catches.
        let kind = if st.is_wt_new() || st.is_index_new() {
            UnrecordedKind::New
        } else if st.is_wt_deleted() || st.is_index_deleted() {
            UnrecordedKind::Deleted
        } else {
            UnrecordedKind::Modified
        };
        out.push(UnrecordedNote { path: p.to_string(), kind });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

/// Mirrors [`crate::git::unpushed`]: how many commits are ahead of the tracking ref, or `None`
/// when nothing has ever been pushed.
pub fn unpushed(vault: &Path) -> Result<Option<u32>, StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let Ok(head) = repo.head() else { return Ok(None) };
    let Ok(name) = head.shorthand() else { return Ok(None) };
    let Ok(upstream) =
        repo.find_branch(&format!("{}/{name}", crate::git::REMOTE), git2::BranchType::Remote)
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

/// Mirrors [`crate::git::last_commit`]: the committer time of `HEAD` in seconds since the epoch,
/// or `None` when there is no repo and no commits.
///
/// `Commit::time()` is git's *committer* time — the same clock `--format=%ct` prints — so the two
/// backends answer with the same number for the same repo, which `git_differential` checks. Every
/// step degrades to `None` rather than erroring: on the phone this is read to draw a chip, and a
/// vault that has never been committed must render as "never saved", not as a failure.
pub fn last_commit(vault: &Path) -> Result<Option<i64>, StoreError> {
    let Ok(repo) = Repository::open(vault) else { return Ok(None) };
    let Ok(head) = repo.head() else { return Ok(None) };
    let Ok(commit) = head.peel_to_commit() else { return Ok(None) };
    Ok(Some(commit.time().seconds()))
}

/// **What a merged file is, for one conflicted path — the decision, and nothing else.**
///
/// This is the piece [`pull`] and [`merge_proposal_branch`] must never disagree about, so it is
/// shared. Everything *around* it deliberately is not: `pull` resolves into the repo-backed index
/// (write the file, then `add_path`), while an accept resolves into the standalone index that
/// `merge_trees` hands back — which rejects `add_path` outright ("Index is not backed up by an
/// existing repository") and, more importantly, **must not touch the filesystem at all**, because
/// a conflicted accept promises to leave the vault untouched. Sharing the staging as well as the
/// decision would mean an accept that writes resolved notes to disk *before* deciding it is
/// conflicted. So the seam is here, at the text.
///
/// Same engine as `fm merge-md`, so a phone and a desktop cannot disagree about what a merged note
/// is — **except for `manifest.json`, which is not a note.** The desktop routes it by
/// `.gitattributes` to a second driver (`merge=fm-manifest`); there is no attribute machinery
/// here, so the path *is* the routing. Without this the manifest went through the note merger:
/// frontmatter parse fails, so it fell to a line-based 3-way over a pretty-printed JSON file where
/// every blob is its own line — and two people each attaching a file conflict on adjacent lines,
/// or on the comma of the last one. That is the *entire* bug the desktop driver exists to fix,
/// with the full consequence chain: the UI tells the user to resolve markers in a "note" that is
/// not one, and `commit_all` then refuses to commit anything in the vault until they do.
fn merged_text(
    path: &str,
    base: &str,
    ours: &str,
    theirs: &str,
) -> Result<(String, crate::merge::Merged), StoreError> {
    if path == "manifest.json" {
        let m = |t: &str| serde_json::from_str::<crate::Manifest>(t).ok().unwrap_or_default();
        let union = crate::Manifest::merge(Some(&m(base)), &m(ours), &m(theirs));
        let text =
            serde_json::to_string_pretty(&union).map_err(|e| StoreError::Io(e.to_string()))?;
        return Ok((text + "\n", crate::merge::Merged::Clean));
    }
    crate::merge::merge_texts(base, ours, theirs, 7)
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
        // Through the shared builder, so this cannot drift from the others.
        let mut opts = fetch_options();
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
    let their_oid =
        upstream.get().target().ok_or_else(|| StoreError::Io("remote ref has no target".into()))?;
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
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).map_err(map)?;
        return Ok(Pulled::Merged { incoming: 0, kept: Vec::new() });
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
    // Notes kept because the other side had deleted them — reported, never silent.
    let mut kept: Vec<String> = Vec::new();

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

            let full = vault.join(&path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }

            // **A deletion on one side and an edit on the other has no text to merge — so keep
            // the note and let the vault carry on.** One side has no blob at all, so
            // `merged_text` can only wrap markers around an empty half; the note stays
            // conflicted, and a conflicted index refuses *every* commit in the vault. That is
            // how two notes froze two hundred for thirty-nine days on a real phone.
            //
            // It discards a deletion, deliberately: somebody removed this note on one device and
            // it is coming back. The trade is stated in `decisions.md` (2026-09-07, *a merge
            // never stalls on a question whose safe answer is a note*) — a resurrected note is
            // visible and one tap from being deleted again, while a note deleted by fiat is gone
            // from the worktree and, on a phone with no shell, gone for good.
            //
            // **Narrow on purpose.** An ancestor plus exactly one missing side is a delete/modify
            // and nothing else: two present sides is a real text conflict and still waits for a
            // human, and no ancestor is an add/add, which this must not touch.
            if c.ancestor.is_some() && c.our.is_none() != c.their.is_none() {
                let keep = if c.our.is_some() { &our_txt } else { &their_txt };
                std::fs::write(&full, keep).map_err(io)?;
                index.conflict_remove(Path::new(&path)).map_err(map)?;
                index.add_path(Path::new(&path)).map_err(map)?;
                kept.push(path);
                continue;
            }

            // THE call — see [`merged_text`], which is this decision and nothing else.
            let (merged, outcome) = merged_text(&path, &base_txt, &our_txt, &their_txt)?;

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

    settle_the_paths_libgit2_merged_itself(
        &repo,
        vault,
        &mut index,
        our_oid,
        their_oid,
        &mut unresolved,
    )?;

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
    kept.sort();
    Ok(Pulled::Merged { incoming, kept })
}

/// **Run our merge over the paths libgit2 settled on its own, because the desktop's driver does.**
///
/// The asymmetry this closes, found 2026-09-07 and reproducible: on a desktop `git merge` invokes
/// `fm merge-md` for **every** path both sides changed, so `merge_texts` reparses the note and
/// re-emits its frontmatter whole. Here, `repo.merge` line-merges the file itself and [`pull`] only
/// revisits what it left *conflicted* — so a note libgit2 could merge never passed through our
/// rules at all.
///
/// That is fine while the two answers agree, and they stop agreeing the moment a note's frontmatter
/// is not already what `to_file` would write: a different key order, a different list indent, a
/// missing `schema:` — anything an external editor, an import or an older version left behind. Then
/// the desktop commits the canonical form and the phone commits the original, **from the same pair
/// of commits**, and the two devices conflict with each other for ever afterwards over a note
/// nobody edited. The trigger is narrow (both sides must have changed the file, and their `updated:`
/// lines must agree, or the path conflicts and the loop above already handles it) and the
/// consequence is not.
///
/// Which paths: those changed **on both sides** relative to the merge base — precisely the set git
/// hands to a merge driver. A path only one side touched is not a merge, and re-emitting it would
/// rewrite a file nobody asked us to touch.
fn settle_the_paths_libgit2_merged_itself(
    repo: &Repository,
    vault: &Path,
    index: &mut git2::Index,
    our_oid: git2::Oid,
    their_oid: git2::Oid,
    unresolved: &mut Vec<String>,
) -> Result<(), StoreError> {
    let base_oid = repo.merge_base(our_oid, their_oid).map_err(map)?;
    let tree = |oid: git2::Oid| repo.find_commit(oid).and_then(|c| c.tree()).map_err(map);
    let (base_tree, our_tree, their_tree) = (tree(base_oid)?, tree(our_oid)?, tree(their_oid)?);

    let changed = |against: &git2::Tree| -> Result<Vec<String>, StoreError> {
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(against), None).map_err(map)?;
        Ok(diff
            .deltas()
            .filter_map(|d| d.new_file().path().or_else(|| d.old_file().path()))
            .map(|p| p.to_string_lossy().into_owned())
            .collect())
    };
    let theirs: std::collections::HashSet<String> = changed(&their_tree)?.into_iter().collect();

    for path in changed(&our_tree)? {
        if !theirs.contains(&path) || index.get_path(Path::new(&path), 0).is_none() {
            // Only one side touched it, or it is still conflicted — the loop above owns that.
            continue;
        }
        let text = |t: &git2::Tree| -> Option<String> {
            let entry = t.get_path(Path::new(&path)).ok()?;
            let blob = repo.find_blob(entry.id()).ok()?;
            Some(String::from_utf8_lossy(blob.content()).into_owned())
        };
        // A side with no blob here is a delete/modify, which reaches the index as a conflict and
        // never gets this far. If one is missing anyway, leave the path exactly as libgit2 left it.
        let (Some(b), Some(o), Some(t)) = (text(&base_tree), text(&our_tree), text(&their_tree))
        else {
            continue;
        };

        let (merged, outcome) = merged_text(&path, &b, &o, &t)?;
        let full = vault.join(&path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        std::fs::write(&full, &merged).map_err(io)?;
        index.add_path(Path::new(&path)).map_err(map)?;
        if outcome != crate::merge::Merged::Clean {
            // Our engine conflicted where libgit2's whole-file merge did not — the same answer a
            // desktop would give, so it is reported the same way rather than quietly accepted.
            unresolved.push(path);
        }
    }
    index.write().map_err(map)?;
    Ok(())
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
    let found = rem.list().map_err(map)?.iter().find(|h| h.name() == wanted).map(|h| h.oid());
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
    let (sign, off) =
        if offset_minutes < 0 { ('-', -offset_minutes) } else { ('+', offset_minutes) };
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

// ===========================================================================
// The proposal lifecycle.
//
// Why it is here at all: **the whole of it was reachable only through `crate::git`**, called
// directly from `fm-app` rather than through [`crate::vcs`], so on a phone — which has no `git`
// binary — every proposal failed with "could not run git (is it installed?)". Creating,
// reviewing, accepting and rejecting a proposal is the only way a UI-only user merges anything,
// so the device that most needs it had none of it. That is the exact trap `vcs.rs` warns about,
// a second wave of call sites the first pass missed; a grep in `ci/checks.sh` now closes it.
// ===========================================================================

/// The committer/author for a proposal commit: the agent's *model* identity when one is given
/// (so a proposal is legibly "who proposed this"), else the vault's own.
fn signature<'a>(
    repo: &'a Repository,
    author: Option<(&str, &str)>,
) -> Result<git2::Signature<'a>, StoreError> {
    match author {
        Some((name, email)) => git2::Signature::now(name, email).map_err(map),
        None => repo.signature().map_err(map),
    }
}

/// Mirrors [`crate::git::create_proposal_branch`].
pub fn create_proposal_branch(
    vault: &Path,
    branch: &str,
    rel_path: &str,
    content: &str,
    message: &str,
    author: Option<(&str, &str)>,
) -> Result<(), StoreError> {
    write_proposal_branch(vault, branch, rel_path, content, message, author, false)
}

/// Mirrors [`crate::git::revise_proposal_branch`].
pub fn revise_proposal_branch(
    vault: &Path,
    branch: &str,
    rel_path: &str,
    content: &str,
    message: &str,
    author: Option<(&str, &str)>,
) -> Result<(), StoreError> {
    write_proposal_branch(vault, branch, rel_path, content, message, author, true)
}

/// Mirrors [`crate::git::write_proposal_branch`] — and keeps its defining property: **a pure
/// object-database build that never touches the working tree, the index, or `HEAD`.** libgit2
/// makes that easier than the subprocess does rather than harder: there is no `GIT_INDEX_FILE`
/// to steer at a temp index and therefore no temp index to leak. A total failure leaves the
/// vault exactly as it was.
///
/// **`commit(None)` then `reference(force)`, not `commit(Some("refs/heads/…"))`.** The latter
/// reads as the natural `update-ref` equivalent and is wrong for a revise: libgit2 requires the
/// first parent to be the named ref's current tip, but a revise re-parents on `HEAD` while the
/// branch still points at the previous revision, so it would be refused outright. The two-step
/// is still a single atomic ref move, which is what the guarantee actually rests on — an
/// interrupted revise leaves the prior revision exactly as it was.
/// Mirrors [`crate::git::retain_proposal_tip`] — keep a proposal's outgoing commit reachable before
/// the ref holding it moves (revise) or disappears (reject). See that function for *why*; the
/// behaviour must be identical or the phone silently collects a different corpus from the laptop,
/// which is the class of divergence `git_differential.rs` exists to catch.
///
/// Best-effort throughout: a failure here must never fail the proposal.
fn retain_proposal_tip(vault: &Path, branch: &str) {
    let Some(id) = branch.strip_prefix("proposal/") else { return };
    let Ok(repo) = Repository::open(vault) else { return };
    let Ok(r) = repo.find_reference(&format!("refs/heads/{branch}")) else { return };
    let Some(tip) = r.target() else { return };
    let _ = repo.reference(&format!("refs/fm/review/{id}/{tip}"), tip, true, "retain proposal tip");
}

fn write_proposal_branch(
    vault: &Path,
    branch: &str,
    rel_path: &str,
    content: &str,
    message: &str,
    author: Option<(&str, &str)>,
    force: bool,
) -> Result<(), StoreError> {
    // No `ensure_repo`: proposing must never rewrite the vault's own setup. `ensure_identity`
    // touches only `.git/config`, and without it a fresh phone vault — which has no global git
    // config to fall back on — has no signature to commit with.
    ensure_identity(vault);
    let repo = Repository::open(vault).map_err(map)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok()).ok_or_else(|| {
        StoreError::Io("this vault has no commits yet — nothing to propose a change to".into())
    })?;

    let refname = format!("refs/heads/{branch}");
    if !force && repo.find_reference(&refname).is_ok() {
        return Err(StoreError::Io(format!("proposal branch {branch:?} already exists")));
    }

    let blob = repo.blob(content.as_bytes()).map_err(map)?;
    // `upsert` creates any missing intermediate trees, so a first note under `notes/` works.
    let tree_oid = git2::build::TreeUpdateBuilder::new()
        .upsert(rel_path, blob, git2::FileMode::Blob)
        .create_updated(&repo, &parent.tree().map_err(map)?)
        .map_err(map)?;
    let tree = repo.find_tree(tree_oid).map_err(map)?;

    let sig = signature(&repo, author)?;
    let commit = repo.commit(None, &sig, &sig, message, &tree, &[&parent]).map_err(map)?;
    retain_proposal_tip(vault, branch); // no-op on a create; on a revise this is the whole point
    repo.reference(&refname, commit, force, "proposal").map_err(map)?;
    Ok(())
}

/// Mirrors `crate::git::resolve_proposal_ref`: the local `proposal/<id>` if we have it, else the
/// remote-tracking `origin/proposal/<id>` that any pull's fetch brings down. The second arm is
/// what lets a *reviewer* accept a proposal whose branch was created on someone else's clone.
fn resolve_proposal_ref(repo: &Repository, branch: &str) -> Option<git2::Oid> {
    for cand in
        [format!("refs/heads/{branch}"), format!("refs/remotes/{}/{branch}", crate::git::REMOTE)]
    {
        if let Some(oid) = repo.find_reference(&cand).ok().and_then(|r| r.target()) {
            return Some(oid);
        }
    }
    None
}

/// Mirrors [`crate::git::branch_open`].
pub fn branch_open(vault: &Path, branch: &str) -> bool {
    let Ok(repo) = Repository::open(vault) else { return false };
    resolve_proposal_ref(&repo, branch).is_some()
}

/// Mirrors [`crate::git::file_on_branch`].
pub fn file_on_branch(vault: &Path, branch: &str, rel: &str) -> Option<String> {
    let repo = Repository::open(vault).ok()?;
    let oid = resolve_proposal_ref(&repo, branch)?;
    let tree = repo.find_commit(oid).ok()?.tree().ok()?;
    let entry = tree.get_path(Path::new(rel)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

/// The merge-base of `HEAD` and a proposal, as a tree — the "what did this fork from" side of
/// every diff below, so commits that landed on `main` since are not counted as part of it.
fn proposal_base_tree<'a>(
    repo: &'a Repository,
    head: git2::Oid,
    theirs: git2::Oid,
) -> Result<git2::Tree<'a>, StoreError> {
    let base = repo.merge_base(head, theirs).unwrap_or(head);
    repo.find_commit(base).map_err(map)?.tree().map_err(map)
}

/// Mirrors [`crate::git::branch_diff`].
///
/// **Not byte-identical to the subprocess backend, by declaration** — libgit2 renders its own
/// patch text (`index` line shape, rename-detection defaults). The file list is exact; the patch
/// is the same change described in the same format, not the same bytes. See the "what is NOT
/// symmetric" section of [`crate::vcs`].
pub fn branch_diff(vault: &Path, branch: &str) -> Result<(bool, Vec<String>, String), StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let Some(theirs) = resolve_proposal_ref(&repo, branch) else {
        return Ok((false, Vec::new(), String::new()));
    };
    let head = repo.head().map_err(map)?.target().unwrap_or(theirs);
    let base_tree = proposal_base_tree(&repo, head, theirs)?;
    let their_tree = repo.find_commit(theirs).map_err(map)?.tree().map_err(map)?;

    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&their_tree), None).map_err(map)?;

    let mut files: Vec<String> = Vec::new();
    for d in diff.deltas() {
        if let Some(p) = d.new_file().path().or_else(|| d.old_file().path()) {
            files.push(p.to_string_lossy().into_owned());
        }
    }
    files.sort();
    files.dedup();

    let mut patch = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        // `+`/`-`/` ` are content lines whose origin character is not part of `content()`;
        // headers and hunk markers already carry their own text.
        if matches!(line.origin(), '+' | '-' | ' ') {
            patch.push(line.origin());
        }
        patch.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(map)?;

    Ok((true, files, patch))
}

/// Mirrors [`crate::git::proposal_load`]: how many `proposal/*` branches are open, and the total
/// bytes of the files they change. A branch that cannot be read is skipped, never fatal — a load
/// probe must not become the thing that blocks every new proposal.
///
/// Counts **local** branches only, exactly as the subprocess backend's `for-each-ref
/// refs/heads/proposal/` does: a proposal held only as `origin/proposal/<id>` is somebody else's
/// open slot, not this vault's. Consistent, not an oversight.
pub fn proposal_load(vault: &Path) -> Result<(usize, u64), StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let head = repo.head().ok().and_then(|h| h.target());

    let mut count = 0usize;
    let mut bytes = 0u64;
    for b in repo.branches(Some(git2::BranchType::Local)).map_err(map)? {
        let Ok((br, _)) = b else { continue };
        let Ok(Some(name)) = br.name() else { continue };
        if !name.starts_with("proposal/") {
            continue;
        }
        count += 1;

        let (Some(head), Some(theirs)) = (head, br.get().target()) else { continue };
        let (Ok(base_tree), Ok(their_tree)) = (
            proposal_base_tree(&repo, head, theirs),
            repo.find_commit(theirs).and_then(|c| c.tree()),
        ) else {
            continue;
        };
        let Ok(diff) = repo.diff_tree_to_tree(Some(&base_tree), Some(&their_tree), None) else {
            continue;
        };
        for d in diff.deltas() {
            // A path *deleted* on the branch has no new blob. The subprocess backend's
            // `cat-file -s` silently contributes 0 there, so this skips rather than erroring.
            let id = d.new_file().id();
            if id.is_zero() {
                continue;
            }
            if let Ok(blob) = repo.find_blob(id) {
                bytes = bytes.saturating_add(blob.size() as u64);
            }
        }
    }
    Ok((count, bytes))
}

/// Mirrors [`crate::git::push_branch`]. Through the shared [`credentials`] callbacks, so auth
/// cannot drift from `push`/`pull`.
pub fn push_branch(vault: &Path, branch: &str, force: bool) -> Result<(), StoreError> {
    let repo = Repository::open(vault).map_err(map)?;
    let mut rem = repo.find_remote(crate::git::REMOTE).map_err(map)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(credentials());
    // A revise moves the branch, so its push must overwrite the remote tip.
    let plus = if force { "+" } else { "" };
    rem.push(&[format!("{plus}refs/heads/{branch}:refs/heads/{branch}")], Some(&mut opts))
        .map_err(map)?;
    Ok(())
}

/// Mirrors [`crate::git::delete_branch`] — how a proposal is REJECTED. Best-effort throughout: a
/// branch that is already gone is success, not an error. **Never touches `main`.**
/// Mirrors [`crate::git::retire_proposal`] — release a settled proposal's guardrail slot while
/// keeping its commit. Local only; the remote is deliberately untouched.
pub fn retire_proposal(vault: &Path, branch: &str) -> Result<(), StoreError> {
    if !vault.join(".git").exists() {
        return Ok(());
    }
    retain_proposal_tip(vault, branch);
    let repo = Repository::open(vault).map_err(map)?;
    if let Ok(mut b) = repo.find_branch(branch, git2::BranchType::Local) {
        let _ = b.delete();
    }
    Ok(())
}

pub fn delete_branch(vault: &Path, branch: &str) -> Result<(), StoreError> {
    if !vault.join(".git").exists() {
        return Ok(());
    }
    // Same placement as the subprocess backend, for the same reason: this serves accept AND reject.
    retain_proposal_tip(vault, branch);
    let repo = Repository::open(vault).map_err(map)?;
    if let Ok(mut b) = repo.find_branch(branch, git2::BranchType::Local) {
        let _ = b.delete();
    }
    if remote(vault)?.is_some() {
        if let Ok(mut rem) = repo.find_remote(crate::git::REMOTE) {
            let mut opts = git2::PushOptions::new();
            opts.remote_callbacks(credentials());
            // An empty push-side of the refspec is a delete. libgit2 also drops the matching
            // `refs/remotes/origin/<branch>`, so `branch_open` correctly goes false afterwards.
            let _ = rem.push(&[format!(":refs/heads/{branch}")], Some(&mut opts));
        }
    }
    Ok(())
}

/// Would accepting have to overwrite work the user has not committed? Asked **only of the paths
/// the merge actually writes**, which is what the subprocess backend effectively does: `git
/// merge` refuses when a *merged* or untracked path would be clobbered, and happily merges
/// around an unrelated dirty file. Checking the whole vault instead would make accept refuse
/// forever in any vault that always carries an uncommitted file.
fn accept_would_clobber(repo: &Repository, paths: &[String]) -> bool {
    paths.iter().any(|p| {
        // `Err` = the path matches nothing git knows or watches: nothing to clobber.
        repo.status_file(Path::new(p))
            .map(|s| !s.is_empty() && s != git2::Status::CURRENT)
            .unwrap_or(false)
    })
}

/// Mirrors [`crate::git::merge_proposal_branch`]: accept a proposal by merging it into the
/// current branch, then dropping the branch so it stops showing as open.
///
/// # Why this is shaped differently from the subprocess backend
///
/// Exec runs `git merge --no-ff` and, on failure, `git merge --abort`. Two things make a direct
/// transliteration wrong here, and **both of them cost notes**:
///
/// 1. **libgit2 never runs the `merge=fm` driver.** It resolves no named merge drivers — it
///    contains no process spawn at all — and silently falls back to the built-in text driver.
///    So the conflicted paths are resolved here, by [`merged_text`], the same engine `pull` uses
///    and the same one the desktop driver calls. Without it essentially every accept of a note
///    that also moved on `main` would report a conflict on the manufactured `updated:` collision.
/// 2. **`checkout_head(force)` is not scoped to the merge.** Its baseline and target are both the
///    `HEAD` tree, so every delta is unmodified and only the *working tree* comparison drives
///    action: every tracked file in the vault that differs from `HEAD` gets overwritten and every
///    user-deleted file resurrected, merged or not. A user mid-edit in an unrelated note, with
///    the debounce not yet fired, would silently lose it by tapping Accept. The checkout below is
///    therefore restricted to exactly the paths the merge changes.
///
/// # The ordering, which is the crash-safety argument
///
/// Decide in memory → **write the working tree and index** → move `HEAD` last. There is no
/// crash-free ordering; this is the one whose failure mode is recoverable. Killed between the
/// checkout and the commit (the Android low-memory killer needs no reason), the vault holds the
/// merged content with `HEAD` unmoved: the next debounced `commit_all` records it as an ordinary
/// commit — losing the merge *parent*, losing no bytes — and re-accepting is idempotent.
///
/// Committing first and checking out second, which reads as the more natural order, is the one
/// shape that must not be used: it leaves `HEAD` moved over a stale index and working tree with
/// **no marker of any kind**, after which the next auto-commit writes a tree from that stale
/// index and silently commits a *revert of the whole accepted proposal*, indistinguishable from
/// a user edit. An undetectable half-state is worse than a detectable one.
pub fn merge_proposal_branch(
    vault: &Path,
    branch: &str,
) -> Result<crate::git::Accepted, StoreError> {
    use crate::git::Accepted;

    ensure_identity(vault);
    let repo = Repository::open(vault).map_err(map)?;
    let Some(their_oid) = resolve_proposal_ref(&repo, branch) else {
        return Ok(Accepted::AlreadyGone);
    };

    // (i) Refuse over an unfinished merge. `pull` deliberately leaves `MERGE_HEAD` and a
    //     conflicted index when a note needs a human; merging on top of that would drop the
    //     remote's commits from the graph, erase the markers the user is resolving, and leave a
    //     conflicted index that blocks *every* later `commit_all` in the vault.
    if repo.state() != git2::RepositoryState::Clean || !conflicts(vault)?.is_empty() {
        return Ok(Accepted::Conflicted);
    }

    let our_oid = repo
        .head()
        .map_err(map)?
        .target()
        .ok_or_else(|| StoreError::Io("HEAD has no target".into()))?;
    let our_commit = repo.find_commit(our_oid).map_err(map)?;
    let their_commit = repo.find_commit(their_oid).map_err(map)?;
    let our_tree = our_commit.tree().map_err(map)?;
    let their_tree = their_commit.tree().map_err(map)?;

    // (ii) Already in. Exec prints "Already up to date." and creates no commit; without this a
    //      retried accept (offline, so the branch delete never reached the remote) would add an
    //      empty merge commit every time.
    if our_oid == their_oid || repo.graph_descendant_of(our_oid, their_oid).map_err(map)? {
        let _ = delete_branch(vault, branch);
        return Ok(Accepted::Merged);
    }

    let Ok(base_oid) = repo.merge_base(our_oid, their_oid) else {
        // Unrelated histories — `git merge` refuses these too.
        return Ok(Accepted::Conflicted);
    };
    let base_tree = repo.find_commit(base_oid).map_err(map)?.tree().map_err(map)?;

    // (iii) Decide, entirely in memory. Nothing on disk has changed when this block returns
    //       `Conflicted`, which is the promise the review UI's "resolve it by editing the
    //       proposed body" escape hatch depends on.
    let mut index = repo.merge_trees(&base_tree, &our_tree, &their_tree, None).map_err(map)?;
    if index.has_conflicts() {
        let items: Vec<_> = index.conflicts().map_err(map)?.flatten().collect();
        for c in items {
            let Some(path) = c
                .our
                .as_ref()
                .or(c.their.as_ref())
                .or(c.ancestor.as_ref())
                .map(|e| String::from_utf8_lossy(&e.path).into_owned())
            else {
                continue;
            };
            let text = |e: &Option<git2::IndexEntry>| -> String {
                e.as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| String::from_utf8_lossy(b.content()).into_owned())
                    .unwrap_or_default()
            };
            let (merged, outcome) =
                merged_text(&path, &text(&c.ancestor), &text(&c.our), &text(&c.their))?;
            if outcome != crate::merge::Merged::Clean {
                return Ok(Accepted::Conflicted);
            }
            let oid = repo.blob(merged.as_bytes()).map_err(map)?;
            index.conflict_remove(Path::new(&path)).map_err(map)?;
            // Built fresh rather than by editing the conflict entry: `Index::add` masks only the
            // name length out of `flags`, so reusing a stage-2/3 entry would re-insert a
            // *conflict* and `write_tree_to` would then fail with "not fully merged".
            index
                .add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
                    uid: 0,
                    gid: 0,
                    file_size: merged.len() as u32,
                    id: oid,
                    flags: 0,
                    flags_extended: 0,
                    path: path.into_bytes(),
                })
                .map_err(map)?;
        }
    }
    let merged_tree = repo.find_tree(index.write_tree_to(&repo).map_err(map)?).map_err(map)?;

    // (iv) The paths this merge actually writes — the checkout's scope, and the only paths whose
    //      uncommitted state can block it.
    let changed = repo.diff_tree_to_tree(Some(&our_tree), Some(&merged_tree), None).map_err(map)?;
    let paths: Vec<String> = changed
        .deltas()
        .filter_map(|d| d.new_file().path().or_else(|| d.old_file().path()))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    if accept_would_clobber(&repo, &paths) {
        return Ok(Accepted::Conflicted);
    }

    // (v) Apply: working tree and index first, `HEAD` last. See the ordering note above.
    let mut co = git2::build::CheckoutBuilder::new();
    co.force();
    for p in &paths {
        co.path(p);
    }
    repo.checkout_tree(merged_tree.as_object(), Some(&mut co)).map_err(map)?;

    let sig = repo.signature().map_err(map)?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("accept: merge {branch}"),
        &merged_tree,
        &[&our_commit, &their_commit],
    )
    .map_err(map)?;

    let _ = delete_branch(vault, branch);
    Ok(Accepted::Merged)
}

/// Windows reaches libgit2 through WinHTTP, which uses the machine's own certificate store — so
/// there is no in-process OpenSSL store to add to, and nothing to do. `Ok(0)` rather than an error
/// because "no certificates were added" is the truth and is not a failure.
#[cfg(windows)]
pub fn add_certs_from_pem(pem: &[u8]) -> Result<usize, StoreError> {
    let _ = pem;
    Ok(0)
}
