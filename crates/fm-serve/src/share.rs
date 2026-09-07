//! **Sharing a vault with a device on the same network** — the pairing, the tokens, and the
//! honest answer to "is this working?".
//!
//! Everything here is transport-shaped and none of it is a `dispatch` command, for the same
//! reason the agent routes are not: the command surface is shared with `fm-cli` and the phone,
//! neither of which has a notion of *who is calling*. A caller's identity is a property of the
//! connection, so it belongs to the thing that owns connections.
//!
//! What this module is careful about, in order of how badly the wrong answer bites:
//!
//! 1. **A token grants audiences, not a machine.** Every device record carries the vault names it
//!    was paired for, and that list becomes a [`fm_app::Scope`]. There is no "all vaults" pairing.
//! 2. **Capability is not configuration.** `enabled: true` is what the *user asked for*; it says
//!    nothing about whether a listener bound, whether the wifi permits device-to-device traffic,
//!    or whether a firewall is dropping the port. The status this module reports is derived from
//!    the socket that actually bound and from whether a remote device has actually been heard
//!    from — never from the setting. (`restic_ready` is the cautionary tale: it meant "repo set +
//!    password set", so on a machine with no restic the checkbox enabled and the backup failed.)
//! 3. **The plaintext token is never written down.** Only `sha256(token)` is persisted, so the
//!    config file is not a key to the vault.

use crate::{AppState, SHARE_COOKIE};
use fm_app::Scope;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a pairing code is worth typing.
const CODE_TTL: Duration = Duration::from_secs(5 * 60);

/// How many wrong guesses cancel a code.
///
/// The cancellation is **announced on the desktop** rather than silent, because otherwise anyone
/// who can see the pairing screen is up can spam wrong codes and lock the owner out of their own
/// pairing, repeatedly, with no explanation.
const CODE_ATTEMPTS: u8 = 5;

/// Unambiguous alphabet: no `O`/`0`, no `I`/`1`/`L`. A code is read off one screen and typed into
/// another, usually by someone holding both.
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// A code is 8 of those — ~39 bits, which the attempt limit below matters far more than.
const CODE_LEN: usize = 8;

/// One paired device. `token_sha256` is the only trace the token leaves on disk.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// **The audiences this device was paired for.** Never empty in practice, and if it somehow
    /// is, the device gets nothing — see [`Scope::Only`].
    pub vaults: Vec<String>,
    pub token_sha256: String,
    pub created: String,
}

/// The persisted half: `<config>/formicaria/share.json`, beside `agent.json` and `vaults.json`.
#[derive(Clone, Debug, Default)]
pub struct ShareConfig {
    pub enabled: bool,
    pub devices: Vec<Device>,
}

/// A code waiting to be typed in. **Memory only** — a pairing code on disk would outlive the
/// five minutes it is supposed to exist for.
struct Pending {
    code: String,
    vaults: Vec<String>,
    born: Instant,
    wrong: u8,
}

/// Everything sharing needs that outlives one request.
pub struct ShareState {
    config: Mutex<ShareConfig>,
    pending: Mutex<Option<Pending>>,
    /// The address a listener **actually bound**, or the reason it did not. This is the whole
    /// basis of the capability report: it is a fact, where the setting is a wish.
    pub bound: Mutex<Result<Option<String>, String>>,
    /// The certificate's fingerprint and the file to install, once a TLS listener has come up.
    /// Shown on the desktop so it can be compared against what the device displays — the only
    /// verification step available, and the reason no click-through path is documented.
    pub cert: Mutex<Option<(String, String)>>,
    /// When a *remote* request last authenticated. `None` means no device has ever got through,
    /// which is the single most useful thing to tell someone whose tablet cannot connect — it
    /// distinguishes "the server isn't listening" from "your network won't carry the packets".
    pub last_remote: Mutex<Option<Instant>>,
}

impl ShareState {
    pub fn load() -> Self {
        ShareState {
            config: Mutex::new(read_config().unwrap_or_default()),
            pending: Mutex::new(None),
            bound: Mutex::new(Ok(None)),
            cert: Mutex::new(None),
            last_remote: Mutex::new(None),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config.lock().map(|c| c.enabled).unwrap_or(false)
    }

    /// Turn sharing on **in memory only**, for the transport tests.
    ///
    /// Deliberately not `set_enabled`, which persists: a test that wrote `share.json` would
    /// reach into the developer's real config directory and turn sharing on for their actual
    /// machine. The gate reads this flag and nothing else, so the coverage is identical.
    #[cfg(test)]
    pub fn force_enabled_for_test(&self) {
        if let Ok(mut c) = self.config.lock() {
            c.enabled = true;
        }
    }

    /// Register a device with a known token, in memory. Same reasoning as above.
    #[cfg(test)]
    pub fn add_device_for_test(&self, token: &str, vaults: &[&str]) {
        if let Ok(mut c) = self.config.lock() {
            c.devices.push(Device {
                id: "test".into(),
                name: "test device".into(),
                vaults: vaults.iter().map(|s| (*s).to_string()).collect(),
                token_sha256: hash(token),
                created: String::new(),
            });
        }
    }
}

/// `<config>/formicaria/share.json`.
fn config_path() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("share.json"))
}

/// Read the config, tolerating absence (never shared before) but not silently tolerating a
/// malformed file — a device list we failed to parse must not be *overwritten*, so a parse
/// failure is reported and the caller keeps the default without ever writing back.
fn read_config() -> Option<ShareConfig> {
    let text = std::fs::read_to_string(config_path()?).ok()?;
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("share: {} is not valid JSON ({e}) — sharing is off, and the file will not be overwritten", config_path()?.display());
            return None;
        }
    };
    Some(ShareConfig {
        enabled: v.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false),
        devices: v
            .get("devices")
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|d| {
                        Some(Device {
                            id: d.get("id")?.as_str()?.to_string(),
                            name: d.get("name").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                            vaults: d
                                .get("vaults")?
                                .as_array()?
                                .iter()
                                .filter_map(|s| s.as_str().map(str::to_string))
                                .collect(),
                            token_sha256: d.get("token_sha256")?.as_str()?.to_string(),
                            created: d
                                .get("created")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Write **every key together**, always.
///
/// The `agent.json` lesson, and it costs more here: writing `enabled` alone would drop `devices`
/// and silently unpair every tablet the user owns, with no error and no way to tell until one of
/// them next tried to connect.
fn write_config(cfg: &ShareConfig) -> Result<(), String> {
    let path = config_path().ok_or("no config directory on this OS")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let devices: Vec<serde_json::Value> = cfg
        .devices
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "name": d.name,
                "vaults": d.vaults,
                "token_sha256": d.token_sha256,
                "created": d.created,
            })
        })
        .collect();
    let doc = serde_json::json!({ "enabled": cfg.enabled, "devices": devices });
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    write_private(&path, text.as_bytes())
}

/// Create the file with owner-only permissions **before** the bytes go in, then write.
///
/// The ordering is the whole point and it is the same one `fm_app::secrets` uses: creating it
/// world-readable and chmod-ing afterwards leaves a window where the token hashes are readable by
/// every account on the machine.
fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
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

/// Random bytes from the OS. **Not `ulid`** — a ULID is timestamp-prefixed and largely
/// predictable, which is right for an identifier and disqualifying for a secret.
fn random_bytes(n: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(buf)
}

/// Compare two equal-length strings without leaking *where* they differ through timing.
///
/// Hand-rolled rather than pulling `subtle` in: it is six lines, and this crate's whole stance is
/// that a dependency has to buy more than that (`decisions.md`, "the core ships as one file").
/// Randomness is the opposite call — see [`random_bytes`] — because getting entropy wrong is
/// silent and unfixable, where getting *this* wrong is visible in the code.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

fn hash(token: &str) -> String {
    fm_core::blob::sha256_hex(token.as_bytes())
}

/// Mint a pairing code for a set of vaults. Replaces any code already waiting: two live codes
/// would mean the screen shows one and the other still works.
pub fn new_code(state: &AppState, vaults: Vec<String>) -> Result<String, String> {
    let raw = random_bytes(CODE_LEN)?;
    let code: String = raw.iter().map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char).collect();
    let mut pending = state.share.pending.lock().map_err(|e| e.to_string())?;
    *pending = Some(Pending { code: code.clone(), vaults, born: Instant::now(), wrong: 0 });
    Ok(code)
}

/// What `POST /api/pair` returns, or the reason it did not.
pub struct Paired {
    pub token: String,
    pub vaults: Vec<String>,
}

/// Redeem a code for a token. **The only unauthenticated write in the server.**
pub fn pair(state: &AppState, code: &str, name: &str) -> Result<Paired, String> {
    let mut pending = state.share.pending.lock().map_err(|e| e.to_string())?;
    let p = pending.as_mut().ok_or("no pairing code is active — start one on the computer")?;

    if p.born.elapsed() > CODE_TTL {
        *pending = None;
        return Err("that code has expired — start a new one on the computer".into());
    }
    if !constant_time_eq(&p.code, &code.trim().to_uppercase()) {
        p.wrong += 1;
        if p.wrong >= CODE_ATTEMPTS {
            *pending = None;
            // Loud, and specifically *not* the same message as a plain wrong code: someone on
            // the network guessing is the case the owner most needs to be able to tell apart
            // from their own typo.
            return Err(
                "too many wrong codes — that code was cancelled. Start a new one on the computer."
                    .into(),
            );
        }
        return Err("that code is not right".into());
    }

    let vaults = p.vaults.clone();
    // Single use: a code that still worked after pairing is a code that can pair a second,
    // unnoticed device.
    *pending = None;
    drop(pending);

    let token = hex(&random_bytes(32)?);
    let device = Device {
        id: hex(&random_bytes(8)?),
        name: if name.trim().is_empty() { "a device".into() } else { name.trim().to_string() },
        vaults: vaults.clone(),
        token_sha256: hash(&token),
        created: now_stamp(),
    };

    let mut cfg = state.share.config.lock().map_err(|e| e.to_string())?;
    cfg.devices.push(device);
    write_config(&cfg)?;
    Ok(Paired { token, vaults })
}

/// Resolve a presented token to what it may reach. `None` means "not one of ours".
pub fn scope_for(state: &AppState, token: &str) -> Option<Scope> {
    let want = hash(token);
    let cfg = state.share.config.lock().ok()?;
    cfg.devices
        .iter()
        .find(|d| constant_time_eq(&d.token_sha256, &want))
        .map(|d| Scope::Only(d.vaults.clone()))
}

/// Forget every paired device. One button, because a per-device list is v2 and "revoke all" is
/// the action someone actually reaches for when a tablet goes missing.
pub fn revoke_all(state: &AppState) -> Result<(), String> {
    let mut cfg = state.share.config.lock().map_err(|e| e.to_string())?;
    cfg.devices.clear();
    write_config(&cfg)
}

/// Turn sharing on or off. Takes effect **at the next launch** — the `agent.rs` precedent:
/// starting and stopping a listener under live connections is a concurrency problem that buys
/// the user nothing they cannot get by reopening the app.
pub fn set_enabled(state: &AppState, enabled: bool) -> Result<(), String> {
    let mut cfg = state.share.config.lock().map_err(|e| e.to_string())?;
    cfg.enabled = enabled;
    write_config(&cfg)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_stamp() -> String {
    // Seconds since the epoch. Not pretty, but this crate has no date formatter and the field is
    // only ever shown as "paired on…" — a dependency for that would be absurd.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

/// The URL to type into the tablet.
///
/// **Prefer `<hostname>.local` over the IP.** Both iOS and Android resolve mDNS, and desktop
/// Linux/macOS already advertise it (avahi/Bonjour), so this costs nothing to try — and it is the
/// only form that survives the DHCP lease moving, which a bookmarked IP does not. The IP is the
/// fallback for a network with no mDNS.
///
/// The IP itself is found by asking the routing table where a packet *would* go: a `connect` on
/// a UDP socket sends nothing, it just picks the interface. **This is the default route**, which
/// on a machine with a VPN up is the tunnel rather than the LAN — so it can be confidently wrong,
/// which is exactly why the status this feeds also reports whether any device has ever actually
/// arrived rather than claiming success on the strength of a string.
/// Just the host part, so a caller that knows its own scheme and port can build the rest.
pub fn reachable_host() -> String {
    if let Some(name) = hostname() {
        return format!("{name}.local");
    }
    local_ip().unwrap_or_else(|| "<this computer>".to_string())
}

pub(crate) fn hostname() -> Option<String> {
    // No dependency for three lines. `hostname` is not in std, but every platform we ship to
    // answers it from the environment or a file.
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return Some(h);
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(crate) fn local_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // Sends no packet — `connect` on UDP only fixes the peer, which makes the kernel choose a
    // source interface. The address is a well-known one that need not be reachable.
    sock.connect("8.8.8.8:53").ok()?;
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

/// The capability report — **derived from what happened, never from the setting**.
pub fn status_json(state: &AppState) -> serde_json::Value {
    let enabled = state.share.enabled();
    let bound = state.share.bound.lock().ok().map(|b| b.clone());
    let devices = state.share.config.lock().map(|c| c.devices.len()).unwrap_or(0);
    let seen = state.share.last_remote.lock().ok().and_then(|t| *t).map(|t| t.elapsed().as_secs());

    match bound {
        // Bound, and we know whether anything ever arrived. `seen: null` is the useful half:
        // "listening, but no device has ever reached me" is what distinguishes a wifi that
        // blocks device-to-device traffic from a server that is not running.
        Some(Ok(Some(url))) => {
            let cert = state.share.cert.lock().ok().and_then(|c| c.clone());
            serde_json::json!({
                "state": "listening", "url": url, "devices": devices, "seen": seen,
                // Present only when the listener is TLS. The UI shows the fingerprint beside the
                // install step so the two can be compared **before** tapping Install — that
                // comparison is the only verification a home network offers, and it is why no
                // click-through-the-warning path is documented anywhere.
                "fingerprint": cert.as_ref().map(|c| c.0.clone()),
                "cert_path": cert.as_ref().map(|c| c.1.clone()),
            })
        }
        Some(Err(why)) => serde_json::json!({ "state": "failed", "why": why }),
        _ if enabled => serde_json::json!({
            "state": "failed",
            "why": "sharing is on, but no listener is running — restart the app to apply it",
        }),
        _ => serde_json::json!({ "state": "off", "devices": devices }),
    }
}

/// The sharing routes. `None` means "not one of mine" — fall through, exactly as `agent::route`
/// does.
///
/// **Every route except `/api/pair` is loopback-only**, and that is not merely the denylist
/// repeating itself: minting a pairing code, reading the device list and turning sharing off are
/// all things a *guest* must not be able to do to its host, and the check reads better as a
/// precondition here than as an entry in a table somewhere else.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    peer: crate::Peer,
    state: &AppState,
) -> Option<std::io::Result<()>> {
    let json = |v: serde_json::Value| serde_json::to_vec(&v).unwrap_or_default();
    let arg = |k: &str| -> String {
        serde_json::from_slice::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get(k).and_then(|s| s.as_str()).map(str::to_string))
            .unwrap_or_default()
    };

    match path {
        // The only route a not-yet-paired device may reach, which is why it is the only one that
        // does not check `peer.loopback`.
        "/api/pair" => {
            if peer.loopback {
                // Nothing breaks, but nothing sensible happens either: the desktop already has
                // every vault. Saying so beats minting a token nobody needs.
                return Some(crate::write_response(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"this computer does not need to pair with itself",
                ));
            }
            match pair(state, &arg("code"), &arg("name")) {
                Ok(p) => {
                    let body = json(serde_json::json!({ "vaults": p.vaults }));
                    // `SameSite=Lax`, **not `Strict`**. A Strict cookie is not sent on a
                    // cross-site *top-level navigation*, which is exactly how the URL reaches a
                    // tablet: a link tapped in Messages, Mail, or a QR scanner. The device would
                    // pair, land back on the app logged out, and burn the single-use code doing
                    // it. The CSRF property Strict would buy is already covered — a remote POST
                    // must carry a matching `Origin`, with no missing-Origin carve-out.
                    //
                    // **`Secure` and persistent, or neither.**
                    //
                    // Over TLS the token gets `Secure` (a browser then refuses to send it over
                    // plain http at all, which is what makes it safe to keep) and a week's
                    // `Max-Age`, so a tablet picked up the next morning is still paired.
                    //
                    // Over plain HTTP — the `--no-default-features` build, or a machine where no
                    // certificate could be minted — it becomes a **session** cookie instead: no
                    // `Max-Age`, gone when the browser closes. `decisions.md` (Android TLS)
                    // rejects sending `userpass_plaintext` over an unauthenticated connection,
                    // and a long-lived bearer token in clear on a LAN is that. Short-lived is the
                    // most that path can honestly have.
                    //
                    // `Max-Age` is capped in days for a reason the HSTS decision already names:
                    // DHCP eventually hands this IP to something else, and a token scoped to an
                    // address outlives the address.
                    let lifetime = if peer.tls { "; Secure; Max-Age=604800" } else { "" };
                    let cookie = format!(
                        "Set-Cookie: {SHARE_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax{lifetime}\r\n",
                        p.token
                    );
                    Some(crate::write_response_with(
                        stream,
                        "200 OK",
                        "application/json",
                        &cookie,
                        &body,
                    ))
                }
                Err(e) => {
                    Some(crate::write_response(stream, "403 Forbidden", "text/plain", e.as_bytes()))
                }
            }
        }
        "/api/share_status" => Some(guarded(stream, peer, || Ok(status_json(state)))),
        "/api/share_code" => Some(guarded(stream, peer, || {
            let vaults: Vec<String> = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|v| {
                    v.get("vaults")?
                        .as_array()
                        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                })
                .unwrap_or_default();
            // **No vaults means no code.** A pairing that granted nothing would look like a
            // broken app; a pairing that silently granted *everything* would be the disclosure
            // this whole feature is arranged to prevent. Refusing is the only honest option.
            if vaults.is_empty() {
                return Err("choose at least one vault to share".to_string());
            }
            let code = new_code(state, vaults)?;
            Ok(serde_json::json!({ "code": code, "expires_in": CODE_TTL.as_secs() }))
        })),
        "/api/set_share" => Some(guarded(stream, peer, || {
            let on = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("enabled").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            set_enabled(state, on)?;
            Ok(serde_json::json!({ "enabled": on, "restart_required": true }))
        })),
        "/api/revoke_devices" => Some(guarded(stream, peer, || {
            revoke_all(state)?;
            Ok(serde_json::json!({ "devices": 0 }))
        })),
        _ => None,
    }
}

/// Run a loopback-only handler and frame its answer.
fn guarded(
    stream: &mut dyn crate::Conn,
    peer: crate::Peer,
    f: impl FnOnce() -> Result<serde_json::Value, String>,
) -> std::io::Result<()> {
    if !peer.loopback {
        return crate::write_response(
            stream,
            "403 Forbidden",
            "text/plain",
            b"only the computer sharing the vault can change this",
        );
    }
    match f() {
        Ok(v) => crate::write_response(
            stream,
            "200 OK",
            "application/json",
            &serde_json::to_vec(&v).unwrap_or_default(),
        ),
        Err(e) => {
            crate::write_response(stream, "500 Internal Server Error", "text/plain", e.as_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_still_compares() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("", "x"));
        assert!(constant_time_eq("", ""));
    }

    /// A code has to be typeable off one screen into another, so the ambiguous glyphs must not
    /// be in it at all — not merely "unlikely".
    #[test]
    fn the_code_alphabet_has_no_confusable_glyphs() {
        for bad in *b"O0I1L" {
            assert!(!ALPHABET.contains(&bad), "{} is confusable", bad as char);
        }
    }

    #[test]
    fn random_bytes_are_not_a_constant() {
        let a = random_bytes(32).unwrap();
        let b = random_bytes(32).unwrap();
        assert_ne!(a, b, "two tokens must not be the same token");
        assert_eq!(a.len(), 32);
        assert!(a.iter().any(|&x| x != 0), "all-zero is not randomness");
    }

    /// The token must not be derivable from what is stored.
    #[test]
    fn only_the_hash_is_stored() {
        let token = hex(&random_bytes(32).unwrap());
        let stored = hash(&token);
        assert_ne!(stored, token);
        assert!(!stored.contains(&token));
        assert_eq!(stored.len(), 64, "sha256, hex");
    }
}
