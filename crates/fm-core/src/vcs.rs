//! Which git backend this build talks to — **the seam that was missing**.
//!
//! # Why this module exists
//!
//! [`crate::git`] shells out and [`crate::git_native`] links libgit2, they are held
//! byte-identical by `tests/git_differential.rs` and `fm-cli/tests/git_native_merge.rs`… and
//! until this module, **nothing outside those tests ever called the native one**. The app layer
//! named `git::` directly at all thirteen call sites, so a phone — which has no `git` binary —
//! got `available() == false`, hid every history feature, and reported "git not installed" while
//! carrying a fully working, fully tested libgit2 inside it.
//!
//! That is the trap `overview.md` already records about `boardOrder.ts`: *a unit test cannot
//! catch a caller that stops calling.* Here it was worse — no caller ever started.
//!
//! # The selection rule
//!
//! **A real `git` binary wins; libgit2 is the fallback.** Deliberately a *runtime* choice rather
//! than `#[cfg]` on the feature, for two reasons:
//!
//! 1. **The desktop must not change behaviour because a feature flag got switched on.** Someone
//!    building with `native-git` for any reason still gets the subprocess path, unchanged, which
//!    is what keeps `decisions.md`'s "the desktop keeps shelling out" true by construction
//!    instead of by convention.
//! 2. **The `.md` merge driver only exists for real git.** It is an external program git spawns,
//!    and libgit2 contains no process spawn at all — so where a `git` binary exists we want the
//!    backend that uses the driver, because that is also the one a collaborator's terminal
//!    `git pull` will use.
//!
//! On Android there is no binary, so every call lands on libgit2. On a desktop nothing here
//! changes what happens at all.
//!
//! # What is NOT symmetric, and why that is stated rather than hidden
//!
//! `activity` on the native backend ignores its `since` argument and returns a *superset* — git's
//! `--since` takes a human string only git's approxidate parser understands. Since each note
//! keeps only its newest touch, walking further back can only fill in notes the desktop would
//! have left blank. Documented at [`crate::git_native::activity`].
//!
//! `branch_diff`'s **patch text** is not byte-identical across backends: libgit2 renders its own
//! (`index` line shape, rename-detection defaults). The *file list* is exact and the change
//! described is the same; only the rendering differs, and nothing reads the patch except a human.
//!
//! `merge_proposal_branch` uses a **single merge base** where git's `ort` strategy recurses over
//! all of them. In a criss-cross history the native accept can therefore refuse where the desktop
//! would merge — fail-closed, never corrupting, and the user's escape hatch (edit the proposed
//! body, save, which revises the proposal onto current `main`) still works.

use crate::StoreError;
use std::path::Path;

/// Whether this build can do history at all: a `git` binary, or libgit2 compiled in.
///
/// **The meaning shifts between backends and the callers are right either way.** On the desktop
/// this answers "is there a git binary", and its absence means no history is possible. With
/// libgit2 linked it is always true, because the library is *there* — so on a phone the honest
/// follow-up questions are whether an identity and a remote are configured, which the backup
/// panel already asks separately.
pub fn available() -> bool {
    crate::git::available() || cfg!(feature = "native-git")
}

/// Force every routed call to libgit2 even where a `git` binary exists — a **testing and
/// diagnostic seam**, not a product switch.
///
/// **Why it has to exist.** [`native`] answers *"is there no git binary"*, so on any developer
/// machine, and in CI, every `vcs::` call resolves to the subprocess backend. That makes the
/// phone's real code path unreachable from any test that drives the **app commands** — and it is
/// precisely the blind spot that let the whole proposal lifecycle ship desktop-only while a
/// two-user test claiming to cover "native" passed, because that test swapped only `pull` and
/// every other call quietly went back to the subprocess. A backend you cannot select is a
/// backend you cannot test end to end.
///
/// **Inert in production.** It exists only when `native-git` is compiled in — off for the desktop
/// — and on Android [`native`] is already true, so nothing there consults it.
///
/// Process-global on purpose: it stands in for "this device has no git", which is a property of a
/// device and not of a call. A test that flips it must serialise, since it is not per-thread.
#[cfg(feature = "native-git")]
static FORCE_NATIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the [`FORCE_NATIVE`] override. See its docs before reaching for this.
#[cfg(feature = "native-git")]
pub fn force_native(on: bool) {
    FORCE_NATIVE.store(on, std::sync::atomic::Ordering::SeqCst);
}

/// True when this call should go to libgit2: when there is no `git` binary to prefer, or when a
/// test has asked to stand in for a device that has none.
///
/// `pub(crate)` because [`crate::merge`] asks the same question for the body engine, and the
/// answer must be **one** definition: a device that routes its `pull` to libgit2 and its body
/// merge to a subprocess is a device whose pull dies half-way through, which is exactly the bug
/// this became.
#[cfg(feature = "native-git")]
pub(crate) fn native() -> bool {
    FORCE_NATIVE.load(std::sync::atomic::Ordering::SeqCst) || !crate::git::available()
}

/// One arm per operation. The `#[cfg]` is inside each function rather than around a `pub use`
/// so that a build *without* the feature still compiles every signature identically — the two
/// backends cannot drift in shape without the compiler saying so.
macro_rules! route {
    ($name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty) => {
        pub fn $name($($arg: $ty),*) -> $ret {
            #[cfg(feature = "native-git")]
            if native() {
                return crate::git_native::$name($($arg),*);
            }
            crate::git::$name($($arg),*)
        }
    };
}

route!(ensure_repo(vault: &Path) -> Result<bool, StoreError>);
route!(identity(vault: &Path) -> Option<crate::git::Identity>);
route!(set_identity(vault: &Path, name: &str, email: &str) -> Result<(), StoreError>);
route!(remote(vault: &Path) -> Result<Option<String>, StoreError>);
route!(set_remote(vault: &Path, url: &str) -> Result<(), StoreError>);
route!(clone(url: &str, dest: &Path) -> Result<(), StoreError>);
route!(conflicts(vault: &Path) -> Result<Vec<String>, StoreError>);
// The conflict *kinds*, and resolving one by keeping a side. Added 2026-07-31 because the marker
// scan the UI listed conflicts from covers only `UU`/`AA`: a delete/modify conflict has no markers,
// so it appeared in no surface while it froze every commit in its vault — for a week, on the
// owner's laptop, with 95 notes unrecorded. `unrecorded` is the other half of that failure.
route!(conflicted(vault: &Path) -> Result<Vec<crate::git::Conflict>, StoreError>);
route!(resolve_conflict(vault: &Path, rel: &str, keep: crate::git::Keep) -> Result<(), StoreError>);
route!(unrecorded(vault: &Path, notes_rel: &str) -> Result<Vec<crate::git::UnrecordedNote>, StoreError>);
route!(unpushed(vault: &Path) -> Result<Option<u32>, StoreError>);
// **How long this vault has been quiet.** Routed from the start rather than reached for directly:
// the device that most needs the answer is the phone, and the phone is the one that never runs the
// subprocess backend.
route!(last_commit(vault: &Path) -> Result<Option<i64>, StoreError>);
route!(last_sent(vault: &Path) -> Result<Option<i64>, StoreError>);
// **Which notes a merge brought back, and the mark that says they have been seen.** Routed like
// everything else in the sync path: the device a resurrection is most likely to surprise is the
// phone, and the phone is the one that never runs the subprocess backend.
route!(kept_notes(vault: &Path) -> Result<Vec<String>, StoreError>);
route!(mark_kept_seen(vault: &Path) -> Result<(), StoreError>);
route!(pull(vault: &Path) -> Result<crate::git::Pulled, StoreError>);
route!(push_squashed(vault: &Path, message: &str) -> Result<u32, StoreError>);
route!(remote_moved(vault: &Path) -> Result<Option<bool>, StoreError>);
route!(activity(vault: &Path, since: &str) -> Result<Vec<crate::git::Touch>, StoreError>);
route!(probe(url: &str) -> crate::git::Probe);

// The proposal lifecycle — create, review, accept, reject. Routed late (2026-07-24): it was the
// *second* wave of call sites naming `git::` directly, and the one that mattered most, since a
// proposal is the only way a UI-only user merges anything and the phone could not make one.
route!(create_proposal_branch(vault: &Path, branch: &str, rel_path: &str, content: &str, message: &str, author: Option<(&str, &str)>) -> Result<(), StoreError>);
route!(revise_proposal_branch(vault: &Path, branch: &str, rel_path: &str, content: &str, message: &str, author: Option<(&str, &str)>) -> Result<(), StoreError>);
route!(proposal_load(vault: &Path) -> Result<(usize, u64), StoreError>);
route!(push_branch(vault: &Path, branch: &str, force: bool) -> Result<(), StoreError>);
route!(delete_branch(vault: &Path, branch: &str) -> Result<(), StoreError>);
route!(retire_proposal(vault: &Path, branch: &str) -> Result<(), StoreError>);
route!(branch_open(vault: &Path, branch: &str) -> bool);
route!(file_on_branch(vault: &Path, branch: &str, rel: &str) -> Option<String>);
route!(branch_diff(vault: &Path, branch: &str) -> Result<(bool, Vec<String>, String), StoreError>);
route!(merge_proposal_branch(vault: &Path, branch: &str) -> Result<crate::git::Accepted, StoreError>);

/// Load CA certificates into the in-process TLS store, from memory.
///
/// **The only route that works on Android**, where vendored OpenSSL is built `no-stdio` and no
/// file can be opened by it at all — see [`crate::git_native::add_certs_from_pem`]. A no-op
/// where a `git` binary does the talking: that uses the system's own trust store.
pub fn add_certs_from_pem(pem: &[u8]) -> Result<usize, StoreError> {
    // Not on Windows or iOS: libgit2 speaks WinHTTP and SecureTransport there, both of which use
    // the system trust store, and neither links the OpenSSL this reaches into. See the target
    // sections in `Cargo.toml`.
    #[cfg(all(feature = "native-git", not(any(windows, target_os = "ios"))))]
    if native() {
        return crate::git_native::add_certs_from_pem(pem);
    }
    let _ = pem;
    Ok(0)
}

/// Point the in-process TLS at a CA bundle *file*. Retained for platforms whose OpenSSL has
/// stdio; **on Android this can never succeed** and [`add_certs_from_pem`] is the path.
pub fn set_cert_file(path: &Path) -> Result<(), StoreError> {
    #[cfg(feature = "native-git")]
    if native() {
        return crate::git_native::set_cert_file(path);
    }
    let _ = path;
    Ok(())
}

/// `commit_all` takes a slice, which the macro's by-value arm cannot express.
pub fn commit_all(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
) -> Result<bool, StoreError> {
    #[cfg(feature = "native-git")]
    if native() {
        return crate::git_native::commit_all(vault, message, paths);
    }
    crate::git::commit_all(vault, message, paths)
}

/// Like [`commit_all`] but attributed to a specific collaborator `(name, email)` — the study agent's
/// model identity, so its own messages are authored by it, not the vault default.
///
/// Hand-written for the same reason [`commit_all`] is: the `route!` macro's by-value arm cannot
/// express a `&[PathBuf]`.
///
/// **This was routed to the git CLI unconditionally until 2026-09-02**, on the stated premise that
/// *"authored commits are an occasional agent op, and `native-git` is a differential-test feature."*
/// The second half of that stopped being true when the phone shipped on libgit2 — so on the one
/// device with no `git` binary, the one call that carries the agent's identity was the one call
/// that could not run. It failed silently (`dispatch.rs` discards the result), and the message
/// committed under the vault's default identity instead.
pub fn commit_all_as(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
    name: &str,
    email: &str,
) -> Result<bool, StoreError> {
    #[cfg(feature = "native-git")]
    if native() {
        return crate::git_native::commit_all_as(vault, message, paths, name, email);
    }
    crate::git::commit_all_as(vault, message, paths, name, email)
}

#[cfg(test)]
mod tests {
    /// On a machine with git, the seam must be the subprocess backend and nothing else — the
    /// desktop's behaviour is not allowed to change because a feature flag exists.
    #[test]
    fn a_machine_with_git_uses_the_subprocess_backend() {
        if !crate::git::available() {
            eprintln!("skipping: no git on PATH");
            return;
        }
        assert!(super::available(), "git is here, so history is possible");
        #[cfg(feature = "native-git")]
        assert!(!super::native(), "a real git binary must win over libgit2");
    }
}
