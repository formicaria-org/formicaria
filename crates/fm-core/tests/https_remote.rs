//! **Can libgit2 reach an HTTPS remote?** — and on Apple targets, does SecureTransport work at all?
//!
//! This is rung 4 of the iOS ladder (`docs/context/ios-plan-2026-09-02.md`), written as an ordinary
//! test so the assertion is the same one on every platform and the *platform* is the only variable.
//!
//! **Why it exists.** `libgit2-sys` picks its TLS backend by target string: WinHTTP on Windows,
//! **SecureTransport on every `apple` target**, OpenSSL everywhere else. So on iOS the vendored
//! OpenSSL this project builds is never called for git, and `git_native::add_certs_from_pem` — the
//! whole Android CA workaround — is `cfg`'d off (`git_native.rs:386`). That leaves the iOS HTTPS
//! path resting entirely on a claim nothing has ever executed: *"SecureTransport uses the system
//! trust store, so it just works."* If that is wrong, formicaria on an iPhone cannot sync, and a
//! notes app whose notes cannot leave the device is not worth distributing.
//!
//! **The TCP pre-check is the load-bearing part, not politeness.** Without it, "there is no
//! network" and "TLS is broken" both arrive as `Probe::Unreachable` and the test proves nothing on
//! the one platform it was written for. A plain `TcpStream` to :443 separates them: reaching the
//! port and *then* failing the git probe is a TLS or trust-store failure and nothing else.
//!
//! Runs against a public repository, so it needs no credentials and asserts nothing about auth —
//! `NeedsAuth` would itself be a pass for the TLS question, and is reported as such.
#![cfg(feature = "native-git")]

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A tiny, long-lived public repository. Chosen for being unlikely to move, and small enough that
/// a ref advertisement is all this costs — `probe` connects and lists refs, downloading no objects.
const URL: &str = "https://github.com/octocat/Hello-World.git";
const HOST: &str = "github.com";
const PORT: u16 = 443;

/// Is there a route to the host at all, TLS aside? Deliberately answered with a raw socket rather
/// than with the function under test.
fn tcp_reachable() -> bool {
    let Ok(addrs) = (HOST, PORT).to_socket_addrs() else {
        return false;
    };
    addrs.into_iter().any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(10)).is_ok())
}

#[test]
fn libgit2_reaches_a_public_https_remote() {
    if !tcp_reachable() {
        eprintln!("skipping: no TCP route to {HOST}:{PORT} — this machine is offline");
        return;
    }

    // **Say what happened, always.** A silent pass is indistinguishable from a skipped test in a
    // CI log, and this one exists to be read on a platform nobody can attach a debugger to.
    match fm_core::git_native::probe(URL) {
        // The question is whether the transport works. Either of these answers means the TLS
        // handshake completed and the certificate chain validated.
        fm_core::git::Probe::Reachable => {
            eprintln!("libgit2 reached {URL} over HTTPS: TLS handshake and certificate chain OK");
        }
        fm_core::git::Probe::NeedsAuth => {
            eprintln!("note: {URL} asked for credentials — TLS still worked, which is the question");
        }
        fm_core::git::Probe::Unreachable(why) => panic!(
            "TCP to {HOST}:{PORT} succeeded but libgit2 could not reach {URL}: {why}\n\
             \n\
             The network is up, so this is the TLS/trust-store path, not connectivity. On an Apple\n\
             target that means SecureTransport or its system trust store — see\n\
             docs/context/ios-plan-2026-09-02.md, rung 4, and git_native.rs's add_certs_from_pem,\n\
             which is cfg'd off there precisely because SecureTransport was assumed to need no\n\
             help. If this is failing on iOS, that assumption is what broke."
        ),
    }
}
