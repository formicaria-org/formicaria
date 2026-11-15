//! **formicaria's own browser, on a computer** — `decisions.md` 2026-11-15, *formicaria gets its own
//! browser*; the plan is `docs/context/browser-plan-2026-11-15.md` (R1).
//!
//! Firefox ESR, **downloaded from Mozilla and checked against Mozilla's own SHA-512 sums** — formicaria
//! redistributes nothing — into `<config>/formicaria/browser/`, with a profile and a `policies.json`
//! this module writes: formicaria's add-on installed, the add-on's key in Firefox's managed storage,
//! DuckDuckGo as the search engine, telemetry off, strict tracking protection, HTTPS-Only, and other
//! add-ons kept off formicaria's own address. formicaria then opens in it.
//!
//! **Transport-shaped, never a `dispatch` command**, for the reason `update.rs` gives: this is a
//! property of this desktop, not of a vault, and `fm-cli` and the phone would never call it. Every
//! route here is in `REMOTE_DENIED`.
//!
//! **Not the assistant's tools folder.** That folder is Myrme's to manage (`agent.rs`, `tools()`), so
//! the browser has a folder of its own.
//!
//! **Linux only in R1**, the one desktop this was measured on (`web-browsing-research` §14, P0.9).
//! Elsewhere the status says so and nothing is offered.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The add-on's id: what `policies.json` installs and hands the key to.
pub const ADDON_ID: &str = "browser@formicaria-org.github.io";
/// uBlock Origin's id on addons.mozilla.org, installed from there (signed by Mozilla) when the person
/// keeps ad blocking on.
const UBLOCK_ID: &str = "uBlock0@raymondhill.net";
const UBLOCK_URL: &str =
    "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi";
/// The routes the add-on may call, and nothing else (`authorize_extension`).
pub const EXTENSION_ROUTES: &[&str] =
    &["/api/clip_save", "/api/recipe_from_page", "/api/list_vaults"];

/// Where Mozilla answers "which is the newest ESR" — with a redirect to the file itself.
fn latest_url() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("https://download.mozilla.org/?product=firefox-esr-latest-ssl&os=linux64&lang=en-US")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("https://download.mozilla.org/?product=firefox-esr-latest-ssl&os=linux64-aarch64&lang=en-US")
    } else {
        None
    }
}

/// Can this machine have formicaria's browser? R1: Linux only.
pub fn supported() -> bool {
    latest_url().is_some()
}

/// `<config>/formicaria/browser`.
pub fn dir() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|c| c.join("formicaria").join("browser"))
}
fn firefox_bin(d: &Path) -> PathBuf {
    d.join("firefox").join("firefox")
}
fn installed(d: &Path) -> bool {
    firefox_bin(d).is_file()
}

/// The person's choices for the browser, kept beside it.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Prefs {
    /// Open formicaria in its own browser at launch (on once it is set up).
    #[serde(default = "yes")]
    pub open_in_it: bool,
    /// Block ads with uBlock Origin, from addons.mozilla.org.
    #[serde(default = "yes")]
    pub block_ads: bool,
}
fn yes() -> bool {
    true
}
impl Default for Prefs {
    fn default() -> Self {
        Prefs { open_in_it: true, block_ads: true }
    }
}
fn read_prefs(d: &Path) -> Prefs {
    std::fs::read(d.join("browser.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn write_prefs(d: &Path, p: &Prefs) -> std::io::Result<()> {
    std::fs::create_dir_all(d)?;
    std::fs::write(d.join("browser.json"), serde_json::to_vec_pretty(p).unwrap_or_default())
}

/// What a setup in progress is doing, for Settings to show.
#[derive(Default)]
pub struct BrowserState {
    pub busy: AtomicBool,
    pub progress: Mutex<(u64, Option<u64>)>,
    pub phase: Mutex<String>,
    pub error: Mutex<Option<String>>,
}

// ---------------------------------------------------------------------------------------------
// The key: what lets formicaria's add-on, and only it, call the capture routes.
// ---------------------------------------------------------------------------------------------

fn key_path(d: &Path) -> PathBuf {
    d.join("key")
}

/// This machine's browser key, if the browser was ever set up.
pub fn key() -> Option<String> {
    let d = dir()?;
    let k = std::fs::read_to_string(key_path(&d)).ok()?;
    let k = k.trim().to_string();
    (k.len() >= 32).then_some(k)
}

/// Make the key if there is none. `0600`: it is a credential.
fn ensure_key(d: &Path) -> Result<String, String> {
    if let Some(k) = key() {
        return Ok(k);
    }
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).map_err(|e| format!("no randomness for the browser key: {e}"))?;
    let k: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    let p = key_path(d);
    write_private(&p, k.as_bytes()).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(k)
}

fn write_private(p: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(p)?;
    f.write_all(bytes)
}

/// Constant-time equality, so the key cannot be guessed a byte at a time from response times.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// **The one new way in** (decision 4): a request from a `moz-extension://` page is let through only
/// on loopback, only with this machine's key, and only to [`EXTENSION_ROUTES`]. Anything else from
/// such an origin stays refused by the CSRF guard, exactly as before.
pub fn authorize_extension(
    origin: &str,
    loopback: bool,
    path: &str,
    given: Option<&str>,
    want: Option<&str>,
) -> bool {
    let route = path.split('?').next().unwrap_or(path);
    loopback
        && origin.starts_with("moz-extension://")
        && EXTENSION_ROUTES.contains(&route)
        && matches!((given, want), (Some(g), Some(w)) if same(g, w))
}

// ---------------------------------------------------------------------------------------------
// The profile and the policy file.
// ---------------------------------------------------------------------------------------------

/// The profile's own preferences (`user.js`), the privacy defaults the owner chose (plan §6):
/// strict tracking protection, HTTPS-Only, Global Privacy Control, cookie banners refused, Safe
/// Browsing on (Firefox's default, left as it is), nothing sent to Mozilla, history on this device.
/// And the one that makes the add-on possible: ESR accepts an add-on Mozilla has not signed
/// (measured, research §14).
pub fn user_js() -> String {
    [
        ("xpinstall.signatures.required", "false"),
        ("browser.contentblocking.category", "\"strict\""),
        ("dom.security.https_only_mode", "true"),
        ("privacy.globalprivacycontrol.enabled", "true"),
        ("cookiebanners.service.mode", "1"),
        ("cookiebanners.service.mode.privateBrowsing", "1"),
        ("browser.shell.checkDefaultBrowser", "false"),
        ("browser.aboutwelcome.enabled", "false"),
        ("datareporting.healthreport.uploadEnabled", "false"),
        ("datareporting.policy.dataSubmissionEnabled", "false"),
        ("toolkit.telemetry.enabled", "false"),
        ("app.shield.optoutstudies.enabled", "false"),
        ("browser.newtabpage.activity-stream.showSponsored", "false"),
        ("browser.newtabpage.activity-stream.showSponsoredTopSites", "false"),
    ]
    .iter()
    .map(|(k, v)| format!("user_pref(\"{k}\", {v});\n"))
    .collect()
}

/// The policy file: our add-on with its key, uBlock Origin if wanted, DuckDuckGo, and formicaria's
/// address kept away from every other add-on's pages.
pub fn policies(xpi: &Path, key: &str, server: &str, prefs: &Prefs) -> Value {
    let xpi_url = file_url(xpi);
    let mut ext = serde_json::Map::new();
    // **Every other add-on is kept off formicaria's pages.** It covers what an add-on injects into
    // a page; an add-on's own requests are refused by the CSRF guard instead (research §14).
    ext.insert("*".into(), json!({ "runtime_blocked_hosts": ["*://127.0.0.1", "*://localhost"] }));
    ext.insert(
        ADDON_ID.into(),
        json!({
            "installation_mode": "force_installed",
            "install_url": xpi_url,
            "runtime_allowed_hosts": ["*://127.0.0.1"],
        }),
    );
    ext.insert(
        UBLOCK_ID.into(),
        if prefs.block_ads {
            json!({ "installation_mode": "normal_installed", "install_url": UBLOCK_URL })
        } else {
            json!({ "installation_mode": "blocked" })
        },
    );
    json!({ "policies": {
        "ExtensionSettings": Value::Object(ext),
        "3rdparty": { "Extensions": { ADDON_ID: { "key": key, "server": server } } },
        "SearchEngines": { "Default": "DuckDuckGo" },
        "DisableTelemetry": true,
        "DisableFirefoxStudies": true,
        "DisablePocket": true,
        "DontCheckDefaultBrowser": true,
        "OverrideFirstRunPage": "",
        "OverridePostUpdatePage": "",
        "Homepage": { "URL": server, "StartPage": "homepage" },
        "EnableTrackingProtection": {
            "Value": true, "Cryptomining": true, "Fingerprinting": true, "EmailTracking": true
        },
    }})
}

fn file_url(p: &Path) -> String {
    let s = p.to_string_lossy();
    let enc: String = s
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("file://{enc}")
}

/// Write the add-on, the key, the profile and the policies — at setup, and again before every
/// launch so the address and the choices are current.
fn prepare(d: &Path, xpi: &[u8], server: &str) -> Result<(), String> {
    let key = ensure_key(d)?;
    let xpi_path = d.join("formicaria.xpi");
    if !xpi.is_empty() {
        std::fs::write(&xpi_path, xpi).map_err(|e| e.to_string())?;
    }
    let profile = d.join("profile");
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
    std::fs::write(profile.join("user.js"), user_js()).map_err(|e| e.to_string())?;
    let dist = d.join("firefox").join("distribution");
    std::fs::create_dir_all(&dist).map_err(|e| e.to_string())?;
    let pol = policies(&xpi_path, &key, server, &read_prefs(d));
    // `0600`: it carries the key.
    write_private(&dist.join("policies.json"), pol.to_string().as_bytes())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// Setting it up.
// ---------------------------------------------------------------------------------------------

/// From Mozilla's redirect target, the version and the file's path inside the release folder:
/// `…/pub/firefox/releases/153.7.0esr/linux-x86_64/en-US/firefox-153.7.0esr.tar.xz` →
/// (`153.7.0esr`, `linux-x86_64/en-US/firefox-153.7.0esr.tar.xz`). Only Mozilla's own hosts.
pub fn parse_release(url: &str) -> Option<(String, String)> {
    let rest = url
        .strip_prefix("https://download-installer.cdn.mozilla.net/pub/firefox/releases/")
        .or_else(|| url.strip_prefix("https://archive.mozilla.org/pub/firefox/releases/"))?;
    let (version, file) = rest.split_once('/')?;
    let ok_version = !version.is_empty()
        && version.ends_with("esr")
        && version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
    let ok_file = file.ends_with(".tar.xz") && !file.contains("..") && !file.contains('?');
    (ok_version && ok_file).then(|| (version.to_string(), file.to_string()))
}

/// The SHA-512 Mozilla publishes for `file`, from the text of `SHA512SUMS`.
pub fn sum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let (h, f) = (it.next()?, it.next()?);
        (f == file && h.len() == 128 && h.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| h.to_ascii_lowercase())
    })
}

fn set_phase(st: &BrowserState, s: &str) {
    if let Ok(mut p) = st.phase.lock() {
        *p = s.to_string();
    }
}

/// Download, check and unpack Firefox ESR, then write the profile. Runs on its own thread.
fn setup(st: &BrowserState, xpi: &[u8], server: &str) -> Result<(), String> {
    let d = dir().ok_or("There is no folder for settings on this computer.")?;
    let latest =
        latest_url().ok_or("formicaria's browser is not available on this computer yet.")?;
    set_phase(st, "Asking Mozilla for the newest Firefox ESR…");
    let target = fm_fetch::redirect_target(latest)
        .map_err(|e| format!("Mozilla could not be reached: {e}"))?
        .ok_or("Mozilla did not say where Firefox is.")?;
    let (version, file) =
        parse_release(&target).ok_or_else(|| format!("Unexpected download address: {target}"))?;
    let sums_url = format!("https://archive.mozilla.org/pub/firefox/releases/{version}/SHA512SUMS");
    let sums = fm_fetch::get(&sums_url, 8 * 1024 * 1024)
        .map_err(|e| format!("Mozilla's checksums could not be read: {e}"))?;
    let want = sum_for(&String::from_utf8_lossy(&sums), &file)
        .ok_or("Mozilla's checksums do not list this file, so it was not downloaded.")?;

    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let archive = d.join(format!("firefox-{version}.tar.xz"));
    set_phase(st, &format!("Downloading Firefox {version} from Mozilla…"));
    let progress = |done: u64, total: Option<u64>| {
        if let Ok(mut p) = st.progress.lock() {
            *p = (done, total);
        }
    };
    fm_fetch::fetch(
        &fm_fetch::Download { url: &target, dest: &archive, sha256: None },
        &progress,
        &|| false,
    )
    .map_err(|e| format!("The download stopped: {e}"))?;
    set_phase(st, "Checking it against Mozilla's published sum…");
    if !fm_fetch::verify_sha512(&archive, &want)? {
        let _ = std::fs::remove_file(&archive);
        return Err(
            "The download did not match Mozilla's published sum, so it was discarded.".into()
        );
    }

    set_phase(st, "Unpacking…");
    let staging = d.join("firefox.new");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("tar")
        .arg("-xJf")
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .output()
        .map_err(|e| format!("`tar` could not be run to unpack Firefox: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!(
            "Firefox could not be unpacked: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let unpacked = staging.join("firefox");
    if !firefox_bin(&staging).is_file() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err("The download held no Firefox program.".into());
    }
    let live = d.join("firefox");
    let _ = std::fs::remove_dir_all(&live);
    std::fs::rename(&unpacked, &live).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_file(&archive);
    std::fs::write(d.join("version"), &version).map_err(|e| e.to_string())?;

    set_phase(st, "Setting up the profile…");
    if !d.join("browser.json").exists() {
        write_prefs(&d, &Prefs::default()).map_err(|e| e.to_string())?;
    }
    prepare(&d, xpi, server)?;
    set_phase(st, "");
    Ok(())
}

/// Start the setup on its own thread, unless one is already running.
pub fn start_setup(st: Arc<BrowserState>, xpi: Vec<u8>, server: String) {
    if st.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(mut e) = st.error.lock() {
        *e = None;
    }
    std::thread::spawn(move || {
        let r = setup(&st, &xpi, &server);
        if let Err(msg) = r {
            if let Ok(mut e) = st.error.lock() {
                *e = Some(msg);
            }
            set_phase(&st, "");
        }
        st.busy.store(false, Ordering::SeqCst);
    });
}

// ---------------------------------------------------------------------------------------------
// Opening formicaria in it.
// ---------------------------------------------------------------------------------------------

/// Should formicaria open in its own browser? Set up, and the person has not turned it off.
pub fn opens_in_it() -> bool {
    dir().is_some_and(|d| installed(&d) && read_prefs(&d).open_in_it)
}

/// Open `url` in formicaria's browser. Refreshes the profile and the policies first, so the add-on
/// has this run's address and the person's current choices. A second call while Firefox is running
/// hands the address to the running Firefox (Firefox's own remoting, by profile).
pub fn launch(url: &str, xpi: &[u8], server: &str) -> Result<(), String> {
    let d = dir().ok_or("no settings folder")?;
    if !installed(&d) {
        return Err("formicaria's browser is not set up".into());
    }
    prepare(&d, xpi, server)?;
    std::process::Command::new(firefox_bin(&d))
        .arg("--profile")
        .arg(d.join("profile"))
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// The routes Settings calls.
// ---------------------------------------------------------------------------------------------

/// What Settings shows.
pub fn status(st: &BrowserState) -> Value {
    let d = dir();
    let installed = d.as_deref().is_some_and(installed);
    let prefs = d.as_deref().map(read_prefs).unwrap_or_default();
    let version = d
        .as_deref()
        .and_then(|d| std::fs::read_to_string(d.join("version")).ok())
        .map(|v| v.trim().to_string());
    let (done, total) = st.progress.lock().map(|p| *p).unwrap_or((0, None));
    json!({
        "supported": supported(),
        "installed": installed,
        "version": version,
        "open_in_it": prefs.open_in_it,
        "block_ads": prefs.block_ads,
        "busy": st.busy.load(Ordering::SeqCst),
        "phase": st.phase.lock().map(|p| p.clone()).unwrap_or_default(),
        "done": done,
        "total": total,
        "error": st.error.lock().ok().and_then(|e| e.clone()),
        "folder": d.map(|p| p.display().to_string()),
    })
}

/// `/api/browser_status`, `/api/browser_setup`, `/api/browser_set`, `/api/browser_open`.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    st: &Arc<BrowserState>,
    xpi: &dyn Fn() -> Vec<u8>,
    server: &str,
) -> Option<std::io::Result<()>> {
    let ok = |stream: &mut dyn crate::Conn, v: Value| {
        crate::write_response(stream, "200 OK", "application/json", v.to_string().as_bytes())
    };
    match path {
        "/api/browser_status" => Some(ok(stream, status(st))),
        "/api/browser_setup" => {
            if !supported() {
                return Some(crate::write_response(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"formicaria's browser is not available on this computer yet.",
                ));
            }
            start_setup(Arc::clone(st), xpi(), server.to_string());
            Some(ok(stream, status(st)))
        }
        "/api/browser_set" => {
            let Some(d) = dir() else { return Some(ok(stream, status(st))) };
            let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
            let mut p = read_prefs(&d);
            if let Some(b) = v.get("open_in_it").and_then(Value::as_bool) {
                p.open_in_it = b;
            }
            if let Some(b) = v.get("block_ads").and_then(Value::as_bool) {
                p.block_ads = b;
            }
            let _ = write_prefs(&d, &p);
            if installed(&d) {
                let _ = prepare(&d, &xpi(), server);
            }
            Some(ok(stream, status(st)))
        }
        // Open formicaria in its browser now, from Settings, right after setting it up.
        "/api/browser_open" => {
            let r = launch(server, &xpi(), server);
            Some(match r {
                Ok(()) => ok(stream, status(st)),
                Err(e) => {
                    crate::write_response(stream, "400 Bad Request", "text/plain", e.as_bytes())
                }
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_add_on_with_the_key_reaches_only_the_capture_routes() {
        let k = Some("k".repeat(64));
        let k = k.as_deref();
        let ext = "moz-extension://1234";
        assert!(authorize_extension(ext, true, "/api/clip_save", k, k));
        assert!(authorize_extension(ext, true, "/api/recipe_from_page?url=x", k, k));
        // The wrong key, no key, or no key on this machine.
        assert!(!authorize_extension(ext, true, "/api/clip_save", Some("nope"), k));
        assert!(!authorize_extension(ext, true, "/api/clip_save", None, k));
        assert!(!authorize_extension(ext, true, "/api/clip_save", k, None));
        // Any other route, even with the key.
        assert!(!authorize_extension(ext, true, "/api/delete", k, k));
        assert!(!authorize_extension(ext, true, "/api/browser_setup", k, k));
        // Not loopback, or not an extension page.
        assert!(!authorize_extension(ext, false, "/api/clip_save", k, k));
        assert!(!authorize_extension("https://evil.example", true, "/api/clip_save", k, k));
    }

    #[test]
    fn only_mozillas_own_esr_addresses_are_followed() {
        let u = "https://download-installer.cdn.mozilla.net/pub/firefox/releases/153.7.0esr/linux-x86_64/en-US/firefox-153.7.0esr.tar.xz";
        assert_eq!(
            parse_release(u),
            Some(("153.7.0esr".into(), "linux-x86_64/en-US/firefox-153.7.0esr.tar.xz".into()))
        );
        assert_eq!(parse_release("https://evil.example/pub/firefox/releases/1esr/x.tar.xz"), None);
        let not_esr = "https://download-installer.cdn.mozilla.net/pub/firefox/releases/159.0/linux-x86_64/en-US/firefox-159.0.tar.xz";
        assert_eq!(parse_release(not_esr), None);
        let sneaky =
            "https://download-installer.cdn.mozilla.net/pub/firefox/releases/153esr/../../x.tar.xz";
        assert_eq!(parse_release(sneaky), None);
    }

    #[test]
    fn the_sum_is_read_for_exactly_this_file() {
        let h = "a".repeat(128);
        let sums = format!(
            "{h}  linux-x86_64/en-US/firefox-153.7.0esr.tar.xz.asc\n{h}  linux-x86_64/en-US/firefox-153.7.0esr.tar.xz\n"
        );
        assert_eq!(sum_for(&sums, "linux-x86_64/en-US/firefox-153.7.0esr.tar.xz"), Some(h));
        assert_eq!(sum_for(&sums, "linux-x86_64/de/firefox-153.7.0esr.tar.xz"), None);
        assert_eq!(sum_for("short  f.tar.xz", "f.tar.xz"), None);
    }

    #[test]
    fn policies_hand_the_key_to_our_add_on_alone_and_keep_others_off_formicaria() {
        let p = policies(
            Path::new("/x y/formicaria.xpi"),
            "KEY",
            "http://127.0.0.1:8765",
            &Prefs::default(),
        );
        let pol = &p["policies"];
        assert_eq!(pol["3rdparty"]["Extensions"][ADDON_ID]["key"], "KEY");
        assert_eq!(pol["3rdparty"]["Extensions"].as_object().unwrap().len(), 1);
        assert_eq!(
            pol["ExtensionSettings"][ADDON_ID]["install_url"],
            "file:///x%20y/formicaria.xpi"
        );
        assert_eq!(
            pol["ExtensionSettings"]["*"]["runtime_blocked_hosts"],
            json!(["*://127.0.0.1", "*://localhost"])
        );
        assert_eq!(pol["SearchEngines"]["Default"], "DuckDuckGo");
        assert_eq!(pol["DisableTelemetry"], true);
        assert_eq!(pol["ExtensionSettings"][UBLOCK_ID]["installation_mode"], "normal_installed");
        let off = policies(
            Path::new("/x"),
            "K",
            "http://127.0.0.1:8765",
            &Prefs { open_in_it: true, block_ads: false },
        );
        assert_eq!(off["policies"]["ExtensionSettings"][UBLOCK_ID]["installation_mode"], "blocked");
    }

    #[test]
    fn the_profile_keeps_the_owners_privacy_defaults() {
        let u = user_js();
        for want in [
            "\"xpinstall.signatures.required\", false",
            "\"browser.contentblocking.category\", \"strict\"",
            "\"dom.security.https_only_mode\", true",
            "\"privacy.globalprivacycontrol.enabled\", true",
            "\"datareporting.healthreport.uploadEnabled\", false",
        ] {
            assert!(u.contains(want), "missing {want}");
        }
        // Safe Browsing stays as Firefox ships it: on (owner, 2026-11-15).
        assert!(!u.contains("safebrowsing"));
    }

    #[test]
    fn keys_compare_whole() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "ab"));
    }
}
