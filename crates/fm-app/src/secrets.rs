//! A git token, on a platform that has nowhere else to put one.
//!
//! # This module should not exist on a desktop, and it does not run there
//!
//! Where there is a `git` binary there is a credential helper, and
//! [`fm_core::git::credential_approve`] hands the token to it — the platform keychain, the
//! GNOME keyring, the Windows Credential Manager. Nothing is stored by us, there is no second
//! copy to go stale when the user rotates the token, and every other git tool on the machine
//! gets the same credential. That is what keeps *"the app holds no secret of its own"*
//! literally true rather than approximately true.
//!
//! **Android has none of that.** No git binary, therefore no helper, no terminal to
//! authenticate in, and no ambient credential store git knows how to reach. libgit2 takes a
//! callback instead, and something has to answer it. So this exists for exactly that case, and
//! [`should_store`] is the guard that keeps it from ever being the desktop's path.
//!
//! # What this protects against, stated honestly
//!
//! The token lives in the app's private directory, which the kernel isolates by UID: **no other
//! app on the device can read it**, and that is a real, enforced boundary — stronger than the
//! plaintext `~/.git-credentials` that `credential.helper store` leaves on a Linux desktop,
//! where every process running as that user can read it.
//!
//! It does **not** protect against:
//!
//! - **A rooted device.** Root reads every app's storage.
//! - **Someone holding the unlocked phone.** They can open formicaria and sync; the token is
//!   usable by the app by design. Only a device-credential gate stops that, and nothing in git
//!   has that concept — it is an OS feature.
//! - **Full-device backup extraction**, unless the manifest disables backup.
//!
//! The hardening step is the Android Keystore: wrap this file's contents with a hardware-backed
//! key so the bytes at rest are useless without the device's secure element. That needs a Kotlin
//! or JNI layer this repo does not have yet, and it is tracked as such rather than implied.
//!
//! # The mitigation that matters more than any of the above
//!
//! **A fine-grained token, scoped to the one repository, with an expiry.** A GitHub PAT is a
//! *bearer* token — it is not bound to a device, and anyone holding the string can use it from
//! anywhere — so the only thing that limits a leak is what the token is allowed to do and for
//! how long. The UI says so at the point the token is pasted, which is the only moment the
//! advice is actionable.

use std::path::PathBuf;

/// The environment variable `fm_core::git_native`'s credential callback reads.
const ENV: &str = "FM_GIT_TOKEN";

/// Whether this platform has to keep the token itself.
///
/// **`false` wherever git exists**, which is the whole point: a desktop delegates to the
/// credential helper and this module never writes anything. A phone has no helper, so it must.
pub fn should_store() -> bool {
    !fm_core::git::available()
}

/// Where the token file goes — beside the vault list, in the app's own config directory.
///
/// `None` when this device has no config directory at all, in which case nothing can be
/// persisted and the caller must say so rather than failing at push time.
pub fn token_path() -> Option<PathBuf> {
    crate::vaults::config_dir().map(|d| d.join("formicaria").join("git-token"))
}

/// Persist the token and make it usable immediately.
///
/// Written `0600` on Unix **before** the secret goes in — created empty with the mode set, then
/// filled. Creating it world-readable and chmod'ing afterwards leaves a window in which the
/// token is on disk and readable, which on a multi-user machine is the whole vulnerability.
pub fn save_token(token: &str) -> Result<(), String> {
    let path = token_path()
        .ok_or("this device has no config directory, so there is nowhere to keep a token")?;
    let parent = path.parent().ok_or("bad token path")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("could not create {}: {e}", parent.display()))?;

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&path).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    {
        use std::io::Write;
        f.write_all(token.trim().as_bytes())
            .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    }
    install(token.trim());
    Ok(())
}

/// Forget the token — the "sign out" a shared or handed-on device needs.
pub fn clear_token() -> Result<(), String> {
    if let Some(path) = token_path() {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("could not remove {}: {e}", path.display())),
        }
    }
    // Safety: the same single-threaded-at-startup contract the mobile shell already relies on
    // for `FM_CONFIG_DIR`. Clearing is a deliberate user action, not a background task.
    unsafe { std::env::remove_var(ENV) };
    Ok(())
}

/// Is a token available to this process right now — from the environment or from disk?
///
/// **Never returns the token**, only whether there is one. No caller needs the value, so no
/// signature here can leak it.
pub fn has_token() -> bool {
    std::env::var(ENV).is_ok_and(|v| !v.is_empty())
        || token_path().and_then(|p| std::fs::read_to_string(p).ok()).is_some_and(|t| !t.trim().is_empty())
}

/// Load a stored token into the environment, where the credential callback reads it.
///
/// Called once at startup by the shell. A missing file is the ordinary state and not an error.
pub fn install_into_env() {
    if std::env::var(ENV).is_ok_and(|v| !v.is_empty()) {
        return; // an explicit environment token wins; this is how CI and tests override it
    }
    let Some(path) = token_path() else { return };
    let Ok(token) = std::fs::read_to_string(&path) else { return };
    if !token.trim().is_empty() {
        install(token.trim());
    }
}

fn install(token: &str) {
    // Safety: startup, or a deliberate user action from the single command surface. The same
    // contract `configure_paths` already relies on.
    unsafe { std::env::set_var(ENV, token) };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file must never be group- or world-readable. Asserted on the mode rather than on the
    /// code that sets it, because the failure mode is silent and permanent.
    #[cfg(unix)]
    #[test]
    fn a_stored_token_is_only_readable_by_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("FM_CONFIG_DIR", dir.path());
        std::env::set_var("XDG_CONFIG_HOME", dir.path());

        save_token("ghp_notarealtoken").unwrap();
        let path = token_path().unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        assert_eq!(mode, 0o600, "a token file must not be readable by anyone else");
        assert!(has_token());

        clear_token().unwrap();
        assert!(!path.exists(), "clearing removes the file, not just the variable");
    }

    /// A desktop delegates to git's credential helper and must never take this path.
    #[test]
    fn a_machine_with_git_does_not_store_a_token_itself() {
        if !fm_core::git::available() {
            eprintln!("skipping: no git on PATH");
            return;
        }
        assert!(!should_store(), "where git exists, the helper owns the credential");
    }
}
