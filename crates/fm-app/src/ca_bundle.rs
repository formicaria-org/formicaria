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

/// Collect every certificate under `dirs` into one PEM file at `out`, and point OpenSSL at it.
///
/// Returns how many distinct certificates were written. `Err` only when there is nothing usable
/// to write — a directory that does not exist is skipped, because which ones exist varies by
/// Android version and an absent one is not a failure.
///
/// **Rebuilt on every launch rather than cached.** An OS update changes the trust store, a
/// cached bundle would pin the app to whatever was true at first run, and reading ~150 small
/// files is a few milliseconds against a startup that already opens a SQLite index.
pub fn install(dirs: &[&Path], out: &Path) -> Result<usize, String> {
    let n = build(dirs, out)?;
    // Safety: startup, before any network call and before other threads exist — the same
    // contract `configure_paths` relies on. `SSL_CERT_FILE` is read by OpenSSL when libgit2
    // calls `SSL_CTX_set_default_verify_paths`.
    unsafe { std::env::set_var("SSL_CERT_FILE", out) };
    Ok(n)
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

    /// The two Android directories overlap heavily — 145 and 149 entries on the measured
    /// device — so without dedup the bundle would carry most roots twice.
    #[test]
    fn the_same_certificate_in_two_directories_is_written_once() {
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        std::fs::write(a.path().join("01419da9.0"), ANDROID_SHAPED).unwrap();
        std::fs::write(b.path().join("8d89cda1.0"), ANDROID_SHAPED).unwrap();
        let out = tempdir().unwrap().path().join("ca-bundle.pem");

        let n = build(&[a.path(), b.path()], &out).unwrap();

        assert_eq!(n, 1, "same certificate, different filenames, one entry");
        let written = std::fs::read_to_string(&out).unwrap();
        assert_eq!(written.matches(BEGIN).count(), 1);
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

    /// A directory that does not exist is skipped — which of Android's two stores is present
    /// varies by version, and an absent one is not a failure.
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
