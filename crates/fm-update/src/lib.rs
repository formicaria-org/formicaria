//! The platform-independent half of updating formicaria.
//!
//! **What a release version is, what a signed release manifest says, whether to trust it, and whether
//! to look for one.** Both shells ask these questions — the desktop server before it swaps its own
//! folder, the Android app before it hands a download to the system installer — and the answers must
//! not differ between them. Above all the list of trusted keys must exist exactly once: a trust root
//! kept in two places is one that drifts. So it lives here, and each shell keeps only what is shaped
//! like its platform (`decisions.md#toolchain`, 2026-09-10 and 2026-09-11).
//!
//! What this crate never does is touch a user's files. The allowlist of what an update may write, the
//! swap, the backups and the launcher's counter all belong to the desktop shell, because only a
//! desktop folder has them.

use std::path::Path;

// ---------------------------------------------------------------------------------------------
// Versions and targets
// ---------------------------------------------------------------------------------------------

/// A release version — `vMAJOR.MINOR.PATCH`, the shape every tag in this repo has had since `v0.1.0`.
///
/// **Hand-parsed, and deliberately so.** It is three integers, and the house stance is to add a
/// dependency only when it solves a problem whole.
///
/// **A build that is not a release has no version and must not be compared.** The crates are all
/// `0.0.0`; the real version arrives through `option_env!("FM_VERSION")` and falls back to `dev`, and a
/// hand-fired build on a branch is stamped `dev-<sha>`. Comparing either against `v0.6.0` is
/// meaningless, so [`Version::parse`] answers `None` and every caller treats that as "this copy cannot
/// update itself", never as "you are out of date".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    /// Parse `v1.2.3` (or `1.2.3`). Anything else — `dev`, `dev-3493b4a`, a suffix, a fourth
    /// component, an empty field — is `None`.
    ///
    /// **Strict on purpose.** A lenient parser that read `v0.6` as `0.6.0`, or ignored a `-rc1`
    /// suffix, would let this code *act* on a string it did not really understand — and what it does
    /// when it acts is replace the program.
    pub fn parse(s: &str) -> Option<Version> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let mut it = s.split('.');
        let (a, b, c) = (it.next()?, it.next()?, it.next()?);
        if it.next().is_some() {
            return None;
        }
        // `u32::from_str` accepts a leading `+`, so check the digits ourselves rather than trusting it.
        let num = |t: &str| -> Option<u32> {
            if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            t.parse().ok()
        };
        Some(Version { major: num(a)?, minor: num(b)?, patch: num(c)? })
    }

    /// What this build is, or `None` when it is not a release build.
    pub fn running() -> Option<Version> {
        Version::parse(option_env!("FM_VERSION").unwrap_or("dev"))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The build target this binary was compiled for, in the naming the release artifacts use.
///
/// `#[cfg]`, not a runtime probe, because it names the *build*, not the machine. A target with no
/// release answers `None` and the whole feature reports itself unavailable, rather than downloading
/// somebody else's architecture.
pub const fn target() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Some("linux-x86_64")
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some("macos-arm64")
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Some("windows-x86_64")
    }
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    {
        Some("android-arm64")
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "android", target_arch = "aarch64"),
    )))]
    {
        None
    }
}

// ---------------------------------------------------------------------------------------------
// The manifest, and whether to trust it
// ---------------------------------------------------------------------------------------------

/// One entry of the signed release manifest: everything needed to fetch and trust one artifact.
///
/// **The manifest is signed, not the bare hash, and that is the whole point.** A signature over a
/// loose sha256 is replayable — whoever chooses which signed bytes you see serves last year's genuine
/// hash, and you install a version with a known hole. Binding `version` and `target` into the signed
/// bytes is what makes "this is the artifact I asked for" checkable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub target: String,
    pub file: String,
    pub sha256: String,
    pub size: u64,
}

/// A parsed release manifest. Parsing does not trust it — [`fetch_verified_manifest`] does that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub version: Version,
    pub entries: Vec<Entry>,
}

impl Manifest {
    /// Parse the JSON `ci/release-manifest.sh` writes:
    ///
    /// ```json
    /// { "version": "v0.6.0",
    ///   "artifacts": [ {"target":"linux-x86_64","file":"…tar.gz","sha256":"…","size":14669973} ] }
    /// ```
    ///
    /// **A hex digest is checked for being one here**, not deep inside the fetcher: a truncated digest
    /// that reached `fetch` would simply never match, which reports as a download that keeps failing
    /// rather than as a manifest that is malformed.
    pub fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        let v: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|e| format!("the update information could not be read: {e}"))?;
        let version = v["version"]
            .as_str()
            .and_then(Version::parse)
            .ok_or_else(|| "the update information does not name a version".to_string())?;
        let arts = v["artifacts"]
            .as_array()
            .ok_or_else(|| "the update information lists no files".to_string())?;
        let mut entries = Vec::with_capacity(arts.len());
        for a in arts {
            let get = |k: &str| -> Result<String, String> {
                a[k].as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("the update information is missing '{k}'"))
            };
            let sha256 = get("sha256")?;
            if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("the update information carries a checksum that is not one".into());
            }
            let size = a["size"]
                .as_u64()
                .ok_or_else(|| "the update information is missing 'size'".to_string())?;
            entries.push(Entry { target: get("target")?, file: get("file")?, sha256, size });
        }
        Ok(Manifest { version, entries })
    }

    /// The artifact for one target, if this release has one.
    pub fn entry(&self, target: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.target == target)
    }
}

/// The trusted keys, **compiled in** from `release-keys.txt`.
///
/// A `const` in the binary rather than a file read at runtime, because a key the program reads from
/// somewhere is a key an attacker can put there. A text file rather than a Rust array because the
/// release workflow has to check that the key it signs with is one of these, and `grep` on hex is
/// exact where `grep` on rustfmt's line-wrapping of a byte array is not.
///
/// **What these defend, said honestly.** The signing key is held as a CI secret, so a valid signature
/// proves a manifest came from this project's release pipeline and was not altered in transit — by a
/// hostile mirror, a bad CDN or a tampered asset. It does **not** defend against a compromised
/// pipeline, which yields the artifacts and the key together. Say *signed against transport
/// tampering*, never *signed releases*.
const RELEASE_KEYS: &str = include_str!("../release-keys.txt");

/// The keys in `release-keys.txt`, or a description of the first line that is not one.
pub fn trusted_keys() -> Result<Vec<[u8; 32]>, String> {
    parse_keys(RELEASE_KEYS)
}

fn parse_keys(text: &str) -> Result<Vec<[u8; 32]>, String> {
    let mut keys = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let body = line.split('#').next().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        keys.push(hex32(body).ok_or_else(|| {
            format!("release-keys.txt line {}: not a 64-character hex key", n + 1)
        })?);
    }
    Ok(keys)
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in b.chunks(2).enumerate() {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out[i] = (hi * 16 + lo) as u8;
    }
    Some(out)
}

/// Check a detached Ed25519 signature over `bytes` against **any** of `keys`.
///
/// **More than one key, so a lost key is survivable.** A single trusted key is a single point of
/// permanent failure: lose it and nothing signed afterwards is accepted, so no installed copy can ever
/// update again. With a second, offline recovery key listed, the recovery key signs one release that
/// lists a new signing key, and everyone carries on.
///
/// **An all-zero key trusts nothing.** A placeholder must never turn into "no key, so everything
/// passes"; with nothing real to check against this refuses.
pub fn verify(bytes: &[u8], signature: &[u8], keys: &[[u8; 32]]) -> Result<(), String> {
    let real: Vec<&[u8; 32]> = keys.iter().filter(|k| k.iter().any(|b| *b != 0)).collect();
    if real.is_empty() {
        return Err("this build carries no key to check the update against".into());
    }
    let accepted = real.iter().any(|k| {
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, k.as_slice())
            .verify(bytes, signature)
            .is_ok()
    });
    if accepted {
        Ok(())
    } else {
        Err("the update did not come from formicaria, so it was not used".into())
    }
}

/// [`verify`] against the keys this build trusts.
pub fn verify_release(bytes: &[u8], signature: &[u8]) -> Result<(), String> {
    verify(bytes, signature, &trusted_keys()?)
}

/// Where a release's files live.
pub fn asset(tag: &str, file: &str) -> String {
    format!("https://github.com/formicaria-org/formicaria/releases/download/{tag}/{file}")
}

/// Fetch a release's manifest and its signature, and return the manifest only if it can be trusted.
///
/// **The order is the security property.** The signature is checked before the manifest is read as
/// anything but bytes — *"unverified is a binary already running"* — and a manifest that does not name
/// a **newer** version than this one is refused, because a genuine signature over an old manifest is
/// exactly what a replay looks like. Only then does a caller learn which file to download, and the
/// hash that file must have.
pub fn fetch_verified_manifest(tag: &str) -> Result<Manifest, String> {
    // 64 KiB each: a manifest is a few hundred bytes per artifact and a signature is 64.
    let manifest_bytes =
        fm_fetch::get(&asset(tag, &format!("formicaria-{tag}.manifest.json")), 64 * 1024)
            .map_err(|e| format!("could not read what {tag} contains: {e}"))?;
    let signature =
        fm_fetch::get(&asset(tag, &format!("formicaria-{tag}.manifest.json.sig")), 64 * 1024)
            .map_err(|e| format!("could not read the signature for {tag}: {e}"))?;
    verify_release(&manifest_bytes, &signature)?;
    let manifest = Manifest::parse(&manifest_bytes)?;
    let current = Version::running().ok_or("this copy has no version to compare")?;
    if manifest.version <= current {
        // Not "you are up to date": we asked for `tag` and were handed something that is not newer,
        // which is a different fact and worth saying plainly.
        return Err(format!(
            "the download said it was {} rather than something newer, so it was not used",
            manifest.version
        ));
    }
    Ok(manifest)
}

// ---------------------------------------------------------------------------------------------
// The per-device setting, and the check
// ---------------------------------------------------------------------------------------------

/// The stored setting, and what the last check found. Each shell decides where the file lives.
///
/// **Read as a `Value` with defaults, never a strict struct.** A user who updates and then goes back
/// must find the older version able to read a file the newer one wrote; `deny_unknown_fields` here
/// would break every downgrade, silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Whether to look for a newer version at all. **On by default** — a fix nobody hears about is a
    /// fix nobody has.
    pub check: bool,
    /// Unix seconds of the last completed check, so a restart does not mean another request.
    pub last_check: u64,
    /// The newest version the last check saw, so the UI can say something without the network.
    pub last_seen: Option<String>,
    /// A version the user installed and then went back from. Not offered again unless they ask.
    pub rejected: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { check: true, last_check: 0, last_seen: None, rejected: None }
    }
}

/// Read the settings at `path`, defaulting anything missing or unreadable.
pub fn settings_at(path: &Path) -> Settings {
    let d = Settings::default();
    let Some(v) = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    else {
        return d;
    };
    Settings {
        check: v["check"].as_bool().unwrap_or(d.check),
        last_check: v["last_check"].as_u64().unwrap_or(0),
        last_seen: v["last_seen"].as_str().map(str::to_owned),
        rejected: v["rejected"].as_str().map(str::to_owned),
    }
}

/// Change the settings at `path` by read-modify-write, preserving keys this version does not know.
pub fn save_at(path: &Path, f: impl FnOnce(&mut serde_json::Value)) -> Result<(), String> {
    let bytes = merged(std::fs::read_to_string(path).ok().as_deref(), f)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// The read-modify-write itself, kept pure so the property that matters can be tested.
///
/// **Unknown keys survive**, so an older version can read what a newer one wrote. Anything that is not
/// an object is replaced rather than merged: it carries nothing worth keeping, and refusing to save
/// because of it would leave the switch permanently stuck.
pub fn merged(
    existing: Option<&str>,
    f: impl FnOnce(&mut serde_json::Value),
) -> Result<Vec<u8>, String> {
    let mut v = existing
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    f(&mut v);
    serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())
}

/// Seconds since the epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The public release listing. **The only address a check ever contacts.**
pub const LATEST: &str = "https://api.github.com/repos/formicaria-org/formicaria/releases/latest";

/// The release page, for a person to read or download from by hand.
pub const RELEASES_PAGE: &str = "https://github.com/formicaria-org/formicaria/releases/latest";

/// A day between checks.
pub const EVERY: u64 = 24 * 60 * 60;

/// Ask which version is newest. Returns the tag, whatever it is — the comparison is the caller's.
///
/// **What this discloses, in full:** one HTTPS GET to [`LATEST`], with a fixed `User-Agent:
/// formicaria`, no query string, no identifier, and nothing about the machine, its vaults or its user.
/// GitHub learns an IP and a time, which is what any download already tells it.
pub fn ask() -> Result<String, String> {
    // 256 KiB: the real answer is a few kilobytes, and the cap is what stops an endless body.
    let body = fm_fetch::get(LATEST, 256 * 1024).map_err(|e| e.to_string())?;
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("the reply could not be read: {e}"))?;
    v["tag_name"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "the reply did not name a version".to_string())
}

/// Is `v` newer than what is running, and not the version this person already went back from?
///
/// `false` when this build has no version at all — the `dev` refusal, applied at the one place that
/// decides whether anything is offered.
pub fn is_newer(v: &Version, rejected: Option<&str>) -> bool {
    if !Version::running().is_some_and(|cur| *v > cur) {
        return false;
    }
    rejected.and_then(Version::parse) != Some(*v)
}

/// Run a check if one is due, remember what it found, and answer with a version worth offering.
///
/// `force` is *Check now*: it ignores both the switch and the interval, because somebody pressed it,
/// and an explicit ask is its own consent. It also clears an earlier rejection — an ask outranks a no.
pub fn check_at(path: &Path, force: bool) -> Result<Option<Version>, String> {
    let s = settings_at(path);
    if !force && (!s.check || now().saturating_sub(s.last_check) < EVERY) {
        return Ok(s
            .last_seen
            .as_deref()
            .and_then(Version::parse)
            .filter(|v| is_newer(v, s.rejected.as_deref())));
    }
    let tag = ask()?;
    let _ = save_at(path, |v| {
        v["last_check"] = serde_json::json!(now());
        v["last_seen"] = serde_json::json!(tag);
        if force {
            v["rejected"] = serde_json::Value::Null;
        }
    });
    let rejected = if force { None } else { s.rejected };
    Ok(Version::parse(&tag).filter(|v| is_newer(v, rejected.as_deref())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_tag_parses_and_orders() {
        assert_eq!(Version::parse("v0.5.1"), Some(Version { major: 0, minor: 5, patch: 1 }));
        assert_eq!(Version::parse("0.5.1"), Version::parse("v0.5.1"));
        assert!(Version::parse("v0.6.0") > Version::parse("v0.5.1"));
        assert!(Version::parse("v0.10.0") > Version::parse("v0.9.9"), "numeric, not lexical");
        assert!(Version::parse("v1.0.0") > Version::parse("v0.99.99"));
    }

    /// A build that is not a release must never be told it is out of date, and must never update
    /// itself: there is nothing to compare.
    #[test]
    fn a_build_that_is_not_a_release_has_no_version() {
        for s in ["dev", "dev-3493b4a", "", "v", "v1", "v1.2", "v1.2.3.4", "v1.2.x", "v1.2.3-rc1"] {
            assert_eq!(Version::parse(s), None, "{s} must not parse");
        }
    }

    #[test]
    fn a_version_field_is_digits_only() {
        for s in ["v+1.2.3", "v1.+2.3", "v-1.2.3", "v1..3", "v 1.2.3", "v1.2. 3"] {
            assert_eq!(Version::parse(s), None, "{s} must not parse");
        }
    }

    fn manifest_json(sha: &str) -> String {
        format!(
            r#"{{"version":"v0.6.0","artifacts":[
                 {{"target":"linux-x86_64","file":"formicaria-v0.6.0-linux-x86_64.tar.gz",
                   "sha256":"{sha}","size":14669973}}]}}"#
        )
    }

    #[test]
    fn a_manifest_parses_and_finds_its_target() {
        let sha = "a".repeat(64);
        let m = Manifest::parse(manifest_json(&sha).as_bytes()).expect("parses");
        assert_eq!(m.version, Version::parse("v0.6.0").unwrap());
        let e = m.entry("linux-x86_64").expect("has linux");
        assert_eq!(e.sha256, sha);
        assert_eq!(e.size, 14669973);
        assert!(
            m.entry("solaris-vax").is_none(),
            "a target we do not publish is absent, not wrong"
        );
    }

    #[test]
    fn a_checksum_that_is_not_one_is_refused() {
        for bad in ["", "abc", &"a".repeat(63), &"a".repeat(65), &"z".repeat(64)] {
            let e = Manifest::parse(manifest_json(bad).as_bytes()).unwrap_err();
            assert!(e.contains("checksum"), "{bad} → {e}");
        }
    }

    #[test]
    fn a_manifest_without_a_version_is_refused() {
        let j = r#"{"version":"dev","artifacts":[]}"#;
        assert!(Manifest::parse(j.as_bytes()).unwrap_err().contains("version"));
    }

    /// A key list with nothing real in it must trust nothing. The failure has to be a refusal, not an
    /// accidental "no key, so everything passes".
    #[test]
    fn no_real_key_verifies_nothing() {
        assert!(verify(b"anything", &[0u8; 64], &[]).unwrap_err().contains("no key"));
        assert!(verify(b"anything", &[0u8; 64], &[[0u8; 32]]).unwrap_err().contains("no key"));
    }

    /// The whole trust root, end to end, with a real Ed25519 key: a signature verifies; a changed byte
    /// does not; a key nobody listed does not.
    #[test]
    fn a_real_signature_verifies_and_a_changed_byte_does_not() {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let rng = ring::rand::SystemRandom::new();
        let pair = |r| {
            Ed25519KeyPair::from_pkcs8(Ed25519KeyPair::generate_pkcs8(r).unwrap().as_ref()).unwrap()
        };
        let signer = pair(&rng);
        let stranger = pair(&rng);
        let mut public = [0u8; 32];
        public.copy_from_slice(signer.public_key().as_ref());

        let manifest = manifest_json(&"a".repeat(64)).into_bytes();
        let sig = signer.sign(&manifest);

        assert!(verify(&manifest, sig.as_ref(), &[public]).is_ok());
        let mut changed = manifest.clone();
        changed[10] ^= 1;
        assert!(
            verify(&changed, sig.as_ref(), &[public]).is_err(),
            "a changed manifest is refused"
        );
        let theirs = stranger.sign(&manifest);
        assert!(
            verify(&manifest, theirs.as_ref(), &[public]).is_err(),
            "an unlisted key is refused"
        );
    }

    /// **More than one trusted key is what makes a lost key survivable** — and a placeholder beside a
    /// real key must be ignored rather than fatal.
    #[test]
    fn any_listed_key_is_enough() {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let rng = ring::rand::SystemRandom::new();
        let recovery =
            Ed25519KeyPair::from_pkcs8(Ed25519KeyPair::generate_pkcs8(&rng).unwrap().as_ref())
                .unwrap();
        let mut public = [0u8; 32];
        public.copy_from_slice(recovery.public_key().as_ref());
        let sig = recovery.sign(b"a release signed with the recovery key");
        assert!(verify(
            b"a release signed with the recovery key",
            sig.as_ref(),
            &[[0u8; 32], [7u8; 32], public]
        )
        .is_ok());
    }

    /// **`openssl` and `ring` agree.** CI signs with `openssl pkeyutl -rawin`; the app verifies with
    /// `ring`. Both claim to do pure Ed25519, and this pins that the claims meet: a manifest written by
    /// `ci/release-manifest.sh` and signed by `ci/release-sign.sh` with a throwaway key verifies here,
    /// and stops verifying when a byte changes. It also pins that the script's output parses.
    #[test]
    fn a_manifest_signed_by_openssl_verifies_with_ring() {
        let manifest = include_bytes!("../tests/fixtures/manifest.json");
        let sig = include_bytes!("../tests/fixtures/manifest.json.sig");
        let key =
            hex32(include_str!("../tests/fixtures/manifest.pub.hex").trim()).expect("fixture key");
        assert_eq!(sig.len(), 64, "a raw Ed25519 signature is 64 bytes");
        verify(manifest, sig, &[key]).expect("ring accepts what openssl signed");

        let mut changed = manifest.to_vec();
        let last = changed.len() - 2;
        changed[last] ^= 1;
        assert!(verify(&changed, sig, &[key]).is_err());

        let parsed = Manifest::parse(manifest).expect("the script's output parses");
        assert_eq!(parsed.version, Version::parse("v9.9.9").unwrap());
        assert!(parsed.entry("android-arm64").is_some() && parsed.entry("linux-x86_64").is_some());
    }

    /// A malformed `release-keys.txt` must fail here, in CI, never on a user's machine as a build that
    /// trusts nothing without saying why.
    #[test]
    fn the_trusted_key_list_is_well_formed() {
        trusted_keys()
            .expect("every non-comment line of release-keys.txt is a 64-character hex key");
        assert!(parse_keys("abcd\n").is_err());
        assert_eq!(parse_keys("# only a comment\n\n").unwrap().len(), 0);
        assert_eq!(
            parse_keys(&format!("{}  # trailing comment\n", "0f".repeat(32))).unwrap().len(),
            1
        );
    }

    /// A version already turned down is not offered again; any other newer version still is.
    #[test]
    fn a_rejected_version_is_not_offered_again() {
        // Under `cargo test` there is no FM_VERSION, so nothing is newer than this build — the `dev`
        // refusal — and a rejection can only ever narrow that further.
        let v = Version::parse("v99.0.0").unwrap();
        assert!(!is_newer(&v, Some("v99.0.0")));
        assert_eq!(is_newer(&v, None), Version::running().is_some());
    }

    /// **The downgrade promise, as a test.** `update.json` must stay readable by an older version,
    /// which only holds if a save preserves keys it does not know.
    #[test]
    fn saving_keeps_keys_this_version_has_never_heard_of() {
        let before = r#"{"check":true,"a_later_idea":{"deep":[1,2]},"channel":"beta"}"#;
        let out = merged(Some(before), |v| v["check"] = serde_json::Value::Bool(false)).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(after["check"], serde_json::json!(false));
        assert_eq!(after["a_later_idea"], serde_json::json!({"deep":[1,2]}));
        assert_eq!(after["channel"], serde_json::json!("beta"));
    }

    #[test]
    fn an_unreadable_settings_file_is_replaced_rather_than_fatal() {
        for junk in [None, Some("not json"), Some("[1,2,3]"), Some("")] {
            let out = merged(junk, |v| v["check"] = serde_json::Value::Bool(true)).unwrap();
            let after: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert_eq!(after["check"], serde_json::json!(true), "{junk:?}");
        }
    }
}
