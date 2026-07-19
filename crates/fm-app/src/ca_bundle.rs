//! A CA trust store for a statically-linked OpenSSL that cannot find one.
//!
//! # The bug this fixes
//!
//! `git2` is built with `vendored-openssl`, so OpenSSL is compiled into the app rather than
//! linked against the system's. A vendored build bakes in a default certificate directory —
//! something like `/usr/local/ssl/certs` — **which does not exist on Android**. With no trust
//! store, every certificate fails to verify, and libgit2 reports exactly one thing:
//!
//! > the SSL certificate is invalid
//!
//! Nothing about that names a missing CA bundle, which is why it reads like a broken server.
//!
//! # Why this concatenates rather than pointing at Android's directory
//!
//! The obvious fix is `SSL_CERT_DIR=/system/etc/security/cacerts`, and it **silently does not
//! work**. That lookup is by hashed filename, and Android's filenames use OpenSSL's
//! *pre-1.0.0* subject hash while a modern vendored OpenSSL computes the current one. Measured
//! on a real device (Android 16), for the same certificate:
//!
//! ```text
//! filename on device        01419da9.0
//! openssl -subject_hash     8d89cda1     ← what OpenSSL 3.x looks for
//! openssl -subject_hash_old 01419da9     ← what Android named it
//! ```
//!
//! So OpenSSL would look for `8d89cda1.0`, find nothing, and fail with the same unhelpful
//! message — a fix that looks correct and changes nothing. `SSL_CERT_FILE` reads a flat
//! concatenated file with no hashed lookup at all, which sidesteps the naming convention
//! entirely.
//!
//! # Why the device's store rather than a bundled one
//!
//! Shipping Mozilla's `cacert.pem` inside the app would work and is what many projects do, but
//! it is a trust store that goes stale the day it ships and can only be refreshed by releasing
//! a new build. Reading the platform's means the app follows the device's own trust decisions
//! and its OS updates, which is the behaviour a user already expects from everything else on
//! their phone.
//!
//! **User-installed CAs are deliberately not included.** Since Android 7 apps do not trust them
//! by default, and silently opting into a corporate or interception CA is not a decision a
//! notes app should make on someone's behalf.
//!
//! # Ordering: this must run before the first `git2` call in the process
//!
//! `libgit2-sys` never defines `GIT_OPENSSL_DYNAMIC`, so libgit2 initialises OpenSSL **eagerly
//! inside `git_libgit2_init()`** — which the `git2` crate triggers on its first use of any API.
//! `SSL_CTX_set_default_verify_paths` (and therefore `SSL_CERT_FILE`) is read exactly once, at
//! that moment. Anything that touches `git2` before this module runs takes the trust store
//! decision away from it permanently.
//!
//! # The directories are a priority list, NOT a union
//!
//! This is a correctness rule, not a preference. Since Android 14 the Conscrypt APEX store is
//! **authoritative** and `/system/etc/security/cacerts` is retained but bypassed — Conscrypt's
//! own `TrustedCertificateStore` picks one directory and ignores the other. Measured on the
//! device here: 145 certificates in the APEX store against 149 in `/system`.
//!
//! **Those four are roots the platform has dropped.** Unioning the two directories would
//! re-trust certificates Android deliberately stopped trusting — turning a trust store into a
//! strictly-more-permissive one, which is the opposite of what a trust store is for. So the
//! first directory that yields any certificate wins, and the rest are not read.

use std::collections::BTreeSet;
use std::path::Path;

const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const END: &str = "-----END CERTIFICATE-----";

/// Pull every PEM certificate block out of `text`, ignoring anything around them.
///
/// Extracting rather than copying whole files matters twice: Android appends a human-readable
/// `Certificate: Data: …` dump after the PEM block in every file, and the two system
/// directories overlap heavily — so the blocks themselves are what can be deduplicated.
fn certificates_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(BEGIN) {
        let after = &rest[start..];
        let Some(end) = after.find(END) else { break };
        out.push(after[..end + END.len()].to_string());
        rest = &after[end + END.len()..];
    }
    out
}

/// Build a trust store from the **first** of `dirs` that has certificates, and point OpenSSL
/// at it.
///
/// `dirs` is a priority list in platform order, not a set to merge — see the module docs: the
/// authoritative store supersedes the legacy one rather than extending it. Returns how many
/// distinct certificates were written. `Err` only when no directory yielded any, because which
/// ones exist varies by Android version and an absent one is not a failure.
///
/// **Rebuilt on every launch rather than cached.** An OS update changes the trust store, a
/// cached bundle would pin the app to whatever was true at first run, and reading ~150 small
/// files is a few milliseconds against a startup that already opens a SQLite index.
pub fn install(dirs: &[&Path], out: &Path) -> Result<usize, String> {
    let result = install_inner(dirs, out);
    // Recorded so `config` can report it. **Startup diagnostics that only reach stderr are
    // invisible on Android** — Rust's stderr is not routed to logcat, so the `eprintln!` here
    // produced exactly nothing while an SSL failure was being debugged. A status the app can
    // show is the only kind that helps.
    let _ = STATUS.set(match &result {
        Ok(n) => format!("{n} certificates"),
        Err(e) => e.clone(),
    });
    result
}

/// The last CA-bundle outcome, for [`crate::dispatch`] to report. `None` on a desktop, where
/// this is never called and the system store is used.
pub fn status() -> Option<String> {
    STATUS.get().cloned()
}

static STATUS: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn install_inner(dirs: &[&Path], out: &Path) -> Result<usize, String> {
    let n = build(dirs, out)?;

    // **The environment first, because it cannot fail.** This used to run *after* the libgit2
    // option with a `?` on it, so an option that errored took the environment variable down
    // with it and left the process with no trust store at all — two mechanisms, and a failure
    // in one discarded both. Order matters here for exactly that reason.
    //
    // Safety: startup, before any network call and before other threads exist — the same
    // contract `configure_paths` relies on.
    unsafe { std::env::set_var("SSL_CERT_FILE", out) };

    // **This result is the whole diagnostic, and discarding it was the mistake that hid the
    // bug.** It had been `let _ =` on the theory that a refusal here was expected and harmless,
    // because libgit2 creates its OpenSSL context lazily. It does not: `libgit2-sys` never
    // defines `GIT_OPENSSL_DYNAMIC`, so `openssl_init()` runs eagerly inside
    // `git_libgit2_init()` and the context always exists by now.
    //
    // So this call genuinely reports whether the bundle loaded, and it is the *only* thing that
    // does. The `SSL_CERT_FILE` route is read once during that eager init, and
    // `X509_STORE_set_default_paths` calls `ERR_clear_error()` and returns success even when it
    // loads nothing — a failure there is invisible by construction.
    //
    // A refusal therefore means the process has **no trusted roots at all**, and every HTTPS
    // remote will fail with "the SSL certificate is invalid" — an error naming the server
    // rather than the cause.
    match fm_core::vcs::set_cert_file(out) {
        Ok(()) => Ok(n),
        Err(e) => Err(format!("{n} certs built, but libgit2 rejected them: {e}")),
    }
}

/// Write the bundle, and touch no global state.
///
/// **Split from [`install`] because the environment is shared and tests are threads.** With
/// the `set_var` inline, two tests that each built a bundle raced on `SSL_CERT_FILE` and one
/// saw the other's path — a failure that appeared only in the full suite and passed when run
/// alone, which is the worst shape a test failure can have.
fn build(dirs: &[&Path], out: &Path) -> Result<usize, String> {
    // Sorted and deduplicated: the same root appears in both Android directories, and a stable
    // order makes the file reproducible, which is what makes it diffable when something is off.
    let mut certs: BTreeSet<String> = BTreeSet::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for e in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            certs.extend(certificates_in(&text));
        }
        // **First store with anything in it wins.** Reading the next one too would union a
        // superseded trust store into the authoritative one and re-trust roots the platform
        // dropped. Dedup within a single directory is still wanted — a store may legitimately
        // carry the same certificate under two names.
        if !certs.is_empty() {
            break;
        }
    }
    if certs.is_empty() {
        return Err(format!(
            "found no CA certificates in {} — HTTPS git remotes cannot be verified",
            dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
        ));
    }

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    let body = certs.iter().map(String::as_str).collect::<Vec<_>>().join("\n");
    std::fs::write(out, format!("{body}\n"))
        .map_err(|e| format!("could not write {}: {e}", out.display()))?;
    Ok(certs.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// A real Android cacert file's shape: one PEM block, then a human-readable dump. The
    /// trailing text must not end up in the bundle.
    const ANDROID_SHAPED: &str = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n\
        Certificate:\n    Data:\n        Version: 3 (0x2)\n";

    #[test]
    fn a_pem_block_is_extracted_and_its_trailing_dump_is_not() {
        let got = certificates_in(ANDROID_SHAPED);
        assert_eq!(got.len(), 1);
        assert!(got[0].starts_with(BEGIN) && got[0].ends_with(END));
        assert!(!got[0].contains("Version: 3"), "the metadata dump must not travel");
    }

    #[test]
    fn a_file_with_several_blocks_yields_all_of_them() {
        let two = format!("{ANDROID_SHAPED}\n{ANDROID_SHAPED}");
        assert_eq!(certificates_in(&two).len(), 2);
    }

    #[test]
    fn text_with_no_certificate_yields_nothing() {
        assert!(certificates_in("just a note about certificates\n").is_empty());
        // A truncated block is not a certificate; taking the prefix would write a corrupt bundle.
        assert!(certificates_in("-----BEGIN CERTIFICATE-----\nAAAA\n").is_empty());
    }

    /// **The authoritative store supersedes the legacy one; it does not merge with it.**
    ///
    /// Since Android 14 the Conscrypt APEX store is authoritative and `/system` is bypassed.
    /// On the measured device the legacy directory held four certificates the APEX one did not
    /// — roots the platform has dropped. A union would re-trust them, which makes the trust
    /// store strictly more permissive than the platform's own. This is the test that stops it.
    #[test]
    fn the_legacy_store_is_not_merged_into_the_authoritative_one() {
        let dropped = "-----BEGIN CERTIFICATE-----\nDROPPED\n-----END CERTIFICATE-----\n";
        let apex = tempdir().unwrap();
        let legacy = tempdir().unwrap();
        std::fs::write(apex.path().join("01419da9.0"), ANDROID_SHAPED).unwrap();
        std::fs::write(legacy.path().join("01419da9.0"), ANDROID_SHAPED).unwrap();
        std::fs::write(legacy.path().join("deadbeef.0"), dropped).unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");

        let n = build(&[apex.path(), legacy.path()], &out).unwrap();

        assert_eq!(n, 1, "only the authoritative store is read");
        let written = std::fs::read_to_string(&out).unwrap();
        assert!(
            !written.contains("DROPPED"),
            "a root the platform dropped must not come back via the legacy store:\n{written}"
        );
    }

    /// Dedup still applies *within* a store — the same certificate may appear under two names.
    #[test]
    fn one_certificate_under_two_names_is_written_once() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("01419da9.0"), ANDROID_SHAPED).unwrap();
        std::fs::write(d.path().join("8d89cda1.0"), ANDROID_SHAPED).unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");

        assert_eq!(build(&[d.path()], &out).unwrap(), 1);
        assert_eq!(std::fs::read_to_string(&out).unwrap().matches(BEGIN).count(), 1);
    }

    /// The one test that touches the process environment, so nothing races it.
    #[test]
    fn install_points_openssl_at_the_bundle_it_wrote() {
        let a = tempdir().unwrap();
        std::fs::write(a.path().join("cert.0"), ANDROID_SHAPED).unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");

        install(&[a.path()], &out).unwrap();

        assert_eq!(std::env::var("SSL_CERT_FILE").unwrap(), out.to_string_lossy());
    }

    /// A missing (or empty) directory falls through to the next — which of Android's stores is
    /// present varies by version, and an absent one is not a failure.
    #[test]
    fn a_missing_directory_is_skipped_not_fatal() {
        let a = tempdir().unwrap();
        std::fs::write(a.path().join("cert.0"), ANDROID_SHAPED).unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");

        let n = build(&[Path::new("/no/such/dir"), a.path()], &out).unwrap();
        assert_eq!(n, 1);
    }

    /// Finding nothing must be an error, not a silently empty bundle — an empty trust store
    /// fails every connection with the same unhelpful "certificate is invalid".
    #[test]
    fn finding_no_certificates_at_all_is_an_error() {
        let empty = tempdir().unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");
        assert!(build(&[empty.path()], &out).is_err());
    }
}
