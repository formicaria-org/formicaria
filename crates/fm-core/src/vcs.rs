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

/// True when this call should go to libgit2: only when there is no `git` binary to prefer.
#[cfg(feature = "native-git")]
fn native() -> bool {
    !crate::git::available()
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
route!(unpushed(vault: &Path) -> Result<Option<u32>, StoreError>);
route!(pull(vault: &Path) -> Result<crate::git::Pulled, StoreError>);
route!(push_squashed(vault: &Path, message: &str) -> Result<u32, StoreError>);
route!(remote_moved(vault: &Path) -> Result<Option<bool>, StoreError>);
route!(activity(vault: &Path, since: &str) -> Result<Vec<crate::git::Touch>, StoreError>);
route!(probe(url: &str) -> crate::git::Probe);

/// Load CA certificates into the in-process TLS store, from memory.
///
/// **The only route that works on Android**, where vendored OpenSSL is built `no-stdio` and no
/// file can be opened by it at all — see [`crate::git_native::add_certs_from_pem`]. A no-op
/// where a `git` binary does the talking: that uses the system's own trust store.
pub fn add_certs_from_pem(pem: &[u8]) -> Result<usize, StoreError> {
    #[cfg(feature = "native-git")]
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
/// model identity, so its own messages are authored by it, not the vault default. Always the git-CLI
/// path: authored commits are an occasional agent op, and `native-git` is a differential-test feature.
pub fn commit_all_as(
    vault: &Path,
    message: &str,
    paths: &[std::path::PathBuf],
    name: &str,
    email: &str,
) -> Result<bool, StoreError> {
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
