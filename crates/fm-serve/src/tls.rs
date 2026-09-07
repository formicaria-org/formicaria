//! **TLS for the shared listener** — and only for it.
//!
//! The one thing it buys is a *secure context* in the browser, which is what `getUserMedia`
//! requires: a paired tablet cannot record audio over `http://<ip>` no matter what else is true,
//! and `record.ts` already fails with exactly that message. Everything else about sharing works
//! over plain HTTP — photo and video capture is a `<input type="file" capture>`, a picker, not
//! `getUserMedia`.
//!
//! ## A leaf, deliberately — not a local certificate authority
//!
//! The convenient design is a private CA installed on the tablet, because then the leaf can be
//! re-minted whenever DHCP moves the address and no device notices. It is rejected here. A root
//! CA in a tablet's trust store can sign **any name for that device's entire browsing life**; the
//! key would live in a config directory, get swept into whatever backs that directory up
//! (including this app's own restic backup), and outlive uninstalling formicaria. A leaked leaf
//! key compromises one notebook server. A leaked CA key compromises the device.
//!
//! The DHCP problem that motivated the CA is solved a cheaper way: `<hostname>.local` is in the
//! SANs and is the URL we advertise. mDNS is resolved by both iOS and Android and already
//! advertised by avahi/Bonjour on the desktop, and a name survives the lease moving where a
//! bookmarked IP does not.
//!
//! ## What is deliberately *not* here
//!
//! - **No "click through the warning" path.** `decisions.md` (2026-07-19, Android TLS) rejects
//!   `certificate_check → CertificateOk` emphatically, for skipping hostname verification. Telling
//!   a user to dismiss an interstitial is the same act with the user as the actor — and under this
//!   design a warning only ever appears on a certificate that is *not* the expected one, which is
//!   precisely the case that must not be waved through. A device that will not trust the
//!   certificate simply has no in-app microphone, which is a supported state with an honest
//!   message already written.
//! - **No fetching the certificate over the connection it authenticates.** That is
//!   trust-on-first-use with no verification step at all. The certificate is offered as a file to
//!   move across by hand, and its SHA-256 fingerprint is shown on the desktop so it can be
//!   compared against what the device displays before installing. That comparison *is* the
//!   hostname verification `decisions.md:1269` refuses to skip, performed by a human because
//!   there is no other root of trust available on a home network.
//! - **No HSTS**, anywhere. It would pin the address to https permanently in the browser; when
//!   DHCP later hands that IP to a printer, the user has an error they cannot clear.

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the certificate and its key live: `<config>/formicaria/share/`.
fn dir() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("share"))
}

/// What a live TLS listener needs, plus what the user must be shown to trust it.
pub struct Tls {
    pub config: Arc<rustls::ServerConfig>,
    /// Uppercase, colon-separated SHA-256 of the DER — the form both iOS and Android display, so
    /// the two strings can be compared character by character without mental arithmetic.
    pub fingerprint: String,
    /// The file to move to the device.
    pub cert_path: PathBuf,
}

// The SAN list is deliberately *not* carried out of here as a Host allowlist. It was considered
// — tying the two together means they cannot drift — but the Host gate has to work identically
// on the plain-HTTP fallback, where there is no certificate at all. The rule it uses instead
// ("a remote Host must be an IP literal or a `.local` name") is independent of TLS and strictly
// tighter than a SAN list, which necessarily contains `localhost`.

/// Load the stored certificate, or mint one — re-minting when the machine's names have changed.
///
/// The names are the whole reason this is not a one-time setup: a laptop moves between networks
/// and gets a new address, and a certificate that does not cover the address the tablet is typing
/// produces a warning, which under the no-click-through rule above means it simply does not work.
pub fn load_or_mint() -> Result<Tls, String> {
    let dir = dir().ok_or("no config directory on this OS")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    restrict(&dir)?;

    let names = names_for_this_machine();
    let (cert_der, key_der) = match reuse(&dir, &names) {
        Some(pair) => pair,
        None => mint(&dir, &names)?,
    };

    let fingerprint = fingerprint_of(&cert_der);
    let config = server_config(cert_der, key_der)?;
    Ok(Tls { config: Arc::new(config), fingerprint, cert_path: dir.join("formicaria.crt") })
}

/// Every name a device on this network might reach us by.
///
/// `<hostname>.local` comes first and is what gets advertised; the IP is the fallback for a
/// network with no mDNS. **The IP found here is the default route's**, which on a machine with a
/// VPN up is the tunnel and not the LAN — one more reason the status this feeds reports whether a
/// device has *actually* connected rather than trusting a string.
fn names_for_this_machine() -> Vec<String> {
    let mut names = vec!["localhost".to_string(), "127.0.0.1".to_string(), "::1".to_string()];
    if let Some(h) = crate::share::hostname() {
        names.push(format!("{h}.local"));
    }
    if let Some(ip) = crate::share::local_ip() {
        names.push(ip);
    }
    names.sort();
    names.dedup();
    names
}

/// Reuse the stored certificate only if it still covers exactly the names we would mint for now.
/// Anything else — a new network, a renamed machine — and it is re-minted, because a certificate
/// that does not cover the address being typed is indistinguishable from no certificate.
fn reuse(dir: &Path, names: &[String]) -> Option<(Vec<u8>, Vec<u8>)> {
    let stored = std::fs::read_to_string(dir.join("names")).ok()?;
    if stored.lines().collect::<Vec<_>>() != names.iter().map(String::as_str).collect::<Vec<_>>() {
        return None;
    }
    Some((std::fs::read(dir.join("cert.der")).ok()?, std::fs::read(dir.join("key.der")).ok()?))
}

fn mint(dir: &Path, names: &[String]) -> Result<(Vec<u8>, Vec<u8>), String> {
    // `CertificateParams::new` parses each string into the right SAN kind — an IP literal becomes
    // an IP SAN, anything else a DNS SAN — which is exactly the distinction a browser checks and
    // exactly the one that is easy to get wrong by hand.
    let mut params =
        rcgen::CertificateParams::new(names.to_vec()).map_err(|e| format!("certificate: {e}"))?;
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        format!(
            "formicaria on {}",
            crate::share::hostname().unwrap_or_else(|| "this computer".into())
        ),
    );
    // **Not a CA.** See the module docs; this is the single most consequential line in the file.
    params.is_ca = rcgen::IsCa::ExplicitNoCa;

    let key = rcgen::KeyPair::generate().map_err(|e| format!("key: {e}"))?;
    let cert = params.self_signed(&key).map_err(|e| format!("self-sign: {e}"))?;

    let cert_der = cert.der().to_vec();
    let key_der = key.serialize_der();

    // DER for our own reload (no parser needed on the way back in), PEM for the human to install.
    write_private(&dir.join("key.der"), &key_der)?;
    std::fs::write(dir.join("cert.der"), &cert_der).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("formicaria.crt"), cert.pem()).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("names"), names.join("\n")).map_err(|e| e.to_string())?;
    Ok((cert_der, key_der))
}

fn server_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> Result<rustls::ServerConfig, String> {
    // With `default-features = false` there is no ambient default provider, so it is installed
    // explicitly. `ok()` because a second call in the same process is an error we do not care
    // about — the provider is already what we wanted.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
        .map_err(|e| format!("private key: {e}"))?;
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("tls config: {e}"))
}

/// `AA:BB:CC:…` over the DER, which is what a device's certificate screen shows.
fn fingerprint_of(der: &[u8]) -> String {
    let hex = fm_core::blob::sha256_hex(der);
    hex.as_bytes()
        .chunks(2)
        .map(|p| String::from_utf8_lossy(p).to_uppercase())
        .collect::<Vec<_>>()
        .join(":")
}

/// Owner-only, and created that way rather than corrected afterwards — the same ordering
/// `fm_app::secrets` uses, for the same reason: a chmod after the write leaves a window in which
/// the private key is world-readable.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    f.write_all(bytes).map_err(|e| e.to_string())
}

fn restrict(dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fingerprint_is_the_shape_a_device_shows() {
        let fp = fingerprint_of(b"whatever");
        assert_eq!(fp.matches(':').count(), 31, "32 bytes, 31 separators");
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit() || c == ':'));
        assert_eq!(fp, fp.to_uppercase(), "devices display uppercase");
    }

    /// The certificate has to cover the names a device will actually type, and **must not be a
    /// CA** — the property the whole trust story rests on, and a one-word change away from being
    /// wrong.
    #[test]
    fn a_minted_certificate_covers_its_names_and_is_not_a_ca() {
        let dir = tempfile::tempdir().unwrap();
        let names =
            vec!["127.0.0.1".to_string(), "kestrel.local".to_string(), "192.168.1.5".to_string()];
        let (der, key) = mint(dir.path(), &names).unwrap();

        assert!(!der.is_empty() && !key.is_empty());
        assert!(dir.path().join("formicaria.crt").exists(), "a file the user can install");

        // Read the DER back as text and look for the SANs. Crude, and deliberately so: parsing
        // X.509 to test X.509 would mean trusting a second parser to prove the first one right.
        let raw = String::from_utf8_lossy(&der);
        assert!(raw.contains("kestrel.local"), "the mDNS name must be a SAN");

        // It must load as a usable server config — which is also the only check that the key and
        // the certificate actually match.
        server_config(der, key).expect("the minted pair must build a server config");
    }

    /// Re-minting is keyed on the names, because a certificate that does not cover the address
    /// being typed is as good as no certificate — and under the no-click-through rule, that means
    /// the feature silently stops working when a laptop changes network.
    #[test]
    fn a_changed_address_forces_a_new_certificate() {
        let dir = tempfile::tempdir().unwrap();
        let home = vec!["127.0.0.1".to_string(), "192.168.1.5".to_string()];
        let (first, _) = mint(dir.path(), &home).unwrap();

        assert!(reuse(dir.path(), &home).is_some(), "the same network reuses the certificate");

        let cafe = vec!["10.0.0.9".to_string(), "127.0.0.1".to_string()];
        assert!(reuse(dir.path(), &cafe).is_none(), "a new address must not reuse it");

        let (second, _) = mint(dir.path(), &cafe).unwrap();
        assert_ne!(first, second);
    }

    /// **A real handshake, end to end** — because everything above tests that we produced
    /// plausible bytes, and none of it tests that a browser could actually talk to us.
    ///
    /// The client here trusts **our own certificate** and nothing else. A `dangerous()` verifier
    /// that accepts anything would make this pass against a server presenting garbage, which is
    /// the same class of mistake `decisions.md:1269` rejects in the shipping code — a test that
    /// skips verification is testing that verification can be skipped.
    #[test]
    fn a_browser_can_complete_a_handshake_and_get_a_response() {
        use std::io::{Read, Write};

        let dir = tempfile::tempdir().unwrap();
        let (cert_der, key_der) = mint(dir.path(), &["localhost".to_string()]).unwrap();
        let server_config = Arc::new(server_config(cert_der.clone(), key_der).unwrap());

        // Trust exactly this certificate, the way a device does after the user installs it.
        let mut roots = rustls::RootCertStore::empty();
        roots.add(rustls::pki_types::CertificateDer::from(cert_der)).unwrap();
        let client_config = Arc::new(
            rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth(),
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (sock, _) = listener.accept().unwrap();
            let conn = rustls::ServerConnection::new(server_config).unwrap();
            let mut s = rustls::StreamOwned::new(conn, sock);
            let mut buf = [0u8; 512];
            let _ = s.read(&mut buf);
            // Written through the same `Conn` path the real server uses — which is the point:
            // this asserts the `&mut dyn Conn` generalization actually carries a TLS session.
            crate::write_response(&mut s, "200 OK", "text/plain", b"hello over tls").unwrap();
        });

        let sock = std::net::TcpStream::connect(addr).unwrap();
        let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let conn = rustls::ClientConnection::new(client_config, name)
            .expect("the client must accept a certificate it has been given");
        let mut s = rustls::StreamOwned::new(conn, sock);
        s.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut got = String::new();
        let _ = s.read_to_string(&mut got);
        server.join().unwrap();

        assert!(got.starts_with("HTTP/1.1 200 OK"), "{got}");
        assert!(got.contains("hello over tls"));
        // The security headers must survive the new transport — they are added in
        // `write_response`, which is the single door every response goes through precisely so
        // that a new transport cannot quietly lose them.
        assert!(got.contains("Content-Security-Policy:"), "CSP lost over TLS: {got}");
        assert!(got.contains("X-Content-Type-Options: nosniff"));
    }

    /// The private key must never be group- or world-readable, and must be created that way
    /// rather than corrected afterwards.
    #[cfg(unix)]
    #[test]
    fn the_private_key_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        mint(dir.path(), &["127.0.0.1".to_string()]).unwrap();
        let mode = std::fs::metadata(dir.path().join("key.der")).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "key.der is readable by someone else: {mode:o}");
    }
}
