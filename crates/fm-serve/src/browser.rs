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
/// The routes the add-on may call, and nothing else (`authorize_extension`): keeping a page, reading
/// a recipe from it, knowing the vaults, and — for a download the person chose to keep in formicaria
/// (0.32.0) — uploading the file and making its note (a song from a bought album).
pub const EXTENSION_ROUTES: &[&str] = &[
    "/api/clip_save",
    "/api/recipe_from_page",
    "/api/list_vaults",
    "/api/ingest",
    "/api/create_notes",
    "/api/music_library",
];

/// The origin of formicaria's own add-on in this browser (`moz-extension://<uuid>`), learnt the first
/// time it proves itself with the key. Firefox picks the uuid per profile, so it cannot be written in
/// advance; it is what the app's policy lets frame it (`main.rs`, `app_csp`).
static OUR_ORIGIN: Mutex<Option<String>> = Mutex::new(None);

/// Is `origin` the shape of an add-on's own origin, `moz-extension://<uuid>`? Pure, so it is tested
/// without touching the one remembered origin (tests run in one process, in parallel).
pub fn is_extension_origin(origin: &str) -> bool {
    origin
        .strip_prefix("moz-extension://")
        .is_some_and(|u| !u.is_empty() && u.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
}

/// Remember the add-on's origin. Only ever called after `authorize_extension` said yes.
pub fn remember_origin(origin: &str) {
    if is_extension_origin(origin) {
        if let Ok(mut o) = OUR_ORIGIN.lock() {
            *o = Some(origin.to_string());
        }
    }
}

/// The add-on's origin, if it has proven itself during this run.
pub fn our_origin() -> Option<String> {
    OUR_ORIGIN.lock().ok().and_then(|o| o.clone())
}

/// Can this machine have formicaria's browser? R1: Linux only.
pub fn supported() -> bool {
    crate::browser_kit::platform().is_some()
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
    /// Install uBlock Origin from addons.mozilla.org. **Off by default** (2026-11-15, `browser-plan`
    /// §16 rule 6: built-in before add-on): measured, a fresh profile with it compiled its filter lists
    /// to +66 % memory over the same ESR without it (`ci/browser-check.py`); Firefox's own strict
    /// tracking protection stays on either way. An offer, a switch in Settings.
    #[serde(default)]
    pub block_ads: bool,
    /// Let sites send notifications while their tab is closed (Mozilla's push service). Off
    /// (owner, 2026-11-15: "web push, a switch, off").
    #[serde(default)]
    pub web_push: bool,
}
fn yes() -> bool {
    true
}
impl Default for Prefs {
    fn default() -> Self {
        Prefs { open_in_it: true, block_ads: false, web_push: false }
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

/// The profile's `user.js`. **It no longer sets anything** (`decisions.md` 2026-11-15, evening): Firefox
/// applies `user.js` over the person's own choices at every start, so 0.31–0.32's privacy defaults could
/// not be changed for good. They are now *defaults* set by formicaria's startup script
/// (`browser/kit/formicaria.cfg`), which the person's settings override; this file is written empty so
/// an older one stops forcing them.
pub fn user_js() -> String {
    "// formicaria sets its defaults in formicaria.cfg (beside the program); your settings win.\n"
        .to_string()
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
            // On the toolbar, not inside the Extensions menu (owner, 2026-11-15).
            "default_area": "navbar",
        }),
    );
    // Offered, never forbidden: off means formicaria does not install it, and the person may still
    // add or remove it from Firefox's add-ons like any other.
    if prefs.block_ads {
        ext.insert(
            UBLOCK_ID.into(),
            json!({ "installation_mode": "normal_installed", "install_url": UBLOCK_URL }),
        );
    }
    json!({ "policies": {
        "ExtensionSettings": Value::Object(ext),
        "3rdparty": { "Extensions": { ADDON_ID: { "key": key, "server": server } } },
        "SearchEngines": { "Default": "DuckDuckGo" },
        // **Stripped by policy** (decision 3, 2026-11-15 evening): Mozilla's telemetry, studies,
        // feedback, Pocket, sponsored content, Suggest, its messages and Labs, the AI chatbot, and
        // Firefox's own updater (formicaria updates it). **Kept**: the Mozilla account and Sync
        // (owner: to start smoothly), add-ons, Remote Settings, Safe Browsing, DRM.
        "DisableTelemetry": true,
        "DisableFirefoxStudies": true,
        "DisableFeedbackCommands": true,
        "DisablePocket": true,
        "DisableAppUpdate": true,
        "NoDefaultBookmarks": true,
        "FirefoxHome": {
            "Search": true, "TopSites": true, "SponsoredTopSites": false, "Highlights": false,
            "Pocket": false, "SponsoredPocket": false, "Stories": false, "SponsoredStories": false,
            "Snippets": false, "Locked": false
        },
        "FirefoxSuggest": {
            "WebSuggestions": false, "SponsoredSuggestions": false, "ImproveSuggest": false,
            "Locked": false
        },
        "UserMessaging": {
            "ExtensionRecommendations": false, "FeatureRecommendations": false,
            "UrlbarInterventions": false, "SkipOnboarding": true, "MoreFromMozilla": false,
            "FirefoxLabs": false, "Locked": false
        },
        "GenerativeAI": { "Enabled": false },
        "EncryptedMediaExtensions": { "Enabled": true },
        "Preferences": {
            "dom.push.enabled": { "Value": prefs.web_push, "Status": "default" }
        },
        "DontCheckDefaultBrowser": true,
        "OverrideFirstRunPage": "",
        "OverridePostUpdatePage": "",
        // **The previous session comes back** (0.33.1): restarting formicaria's browser, to finish an
        // update for one, must not lose the person's tabs. The formicaria tab is pinned and restored
        // with them; the startup script opens it if it is missing.
        "Homepage": { "URL": server, "StartPage": "previous-session" },
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

/// The add-on's file name for these bytes: `formicaria-<16 hex>.xpi`.
pub fn xpi_name(xpi: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    xpi.hash(&mut h);
    format!("formicaria-{:016x}.xpi", h.finish())
}

/// Write the add-on, the key, the profile and the policies — at setup, and again before every
/// launch so the address and the choices are current.
fn prepare(d: &Path, xpi: &[u8], server: &str) -> Result<(), String> {
    let key = ensure_key(d)?;
    // **A file name per add-on build** (0.33.1): Firefox installs a policy's add-on again when its
    // address changes, so each build gets its own, and the older files are removed.
    let xpi_path = d.join(if xpi.is_empty() { "formicaria.xpi".into() } else { xpi_name(xpi) });
    if !xpi.is_empty() {
        if !xpi_path.is_file() {
            std::fs::write(&xpi_path, xpi).map_err(|e| e.to_string())?;
        }
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with("formicaria") && n.ends_with(".xpi") && e.path() != xpi_path {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
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

/// Set formicaria's browser up: the pinned ESR, or a newer point release of the same major, from
/// Mozilla, checked, made formicaria's (`browser_kit::acquire`), then put in place, keeping any previous
/// copy. Runs on its own thread.
fn setup(st: &BrowserState, xpi: &[u8], server: &str) -> Result<(), String> {
    let d = dir().ok_or("There is no folder for settings on this computer.")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let pin = crate::browser_kit::pin();
    set_phase(st, "Asking Mozilla for its newest security release…");
    let version = crate::browser_kit::latest_esr()
        .filter(|v| crate::browser_kit::is_newer_point(&pin.version, v, pin.major))
        .unwrap_or_else(|| pin.version.clone());
    let work = d.join("work");
    let progress = |done: u64, total: Option<u64>| {
        if let Ok(mut p) = st.progress.lock() {
            *p = (done, total);
        }
    };
    let ff = crate::browser_kit::acquire(&work, &version, &|m| set_phase(st, m), &progress)?;
    if crate::browser_kit::running(&d) {
        // In use: the ready copy is swapped in at the next start.
        let _ = std::fs::remove_dir_all(d.join("firefox.next"));
        std::fs::rename(&ff, d.join("firefox.next")).map_err(|e| e.to_string())?;
    } else {
        let live = d.join("firefox");
        let prev = d.join("firefox.prev");
        let _ = std::fs::remove_dir_all(&prev);
        if live.exists() {
            std::fs::rename(&live, &prev).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&ff, &live).map_err(|e| e.to_string())?;
        std::fs::write(d.join("version"), &version).map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_dir_all(&work);
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

/// **The window is formicaria's, by name** (owner, 2026-11-15: *"when double clicking on fm, it would
/// be nice to see fm, not firefox … I mean the names"*). The dock, Alt+Tab and the top bar name a window
/// after the desktop entry its label matches: `--name` sets that label (the app id on Wayland, the
/// window class on X11, with `--class`), so the window is shown as **formicaria** with formicaria's
/// icon (`formicaria.desktop`, `StartupWMClass=formicaria`), apart from any other Firefox. Firefox
/// hands a second launch to the running one by the same name.
pub fn launch_args(profile: &Path, url: &str) -> Vec<std::ffi::OsString> {
    vec![
        "--name".into(),
        "formicaria".into(),
        "--class".into(),
        "formicaria".into(),
        "--profile".into(),
        profile.as_os_str().to_owned(),
        url.into(),
    ]
}

/// Open `url` in formicaria's browser. Refreshes the profile and the policies first, so the add-on
/// has this run's address and the person's current choices. A second call while Firefox is running
/// hands the address to the running Firefox (Firefox's own remoting, by profile).
pub fn launch(url: &str, xpi: &[u8], server: &str) -> Result<(), String> {
    let d = dir().ok_or("no settings folder")?;
    if !installed(&d) {
        return Err("formicaria's browser is not set up".into());
    }
    // A copy prepared in the background (a newer kit, a security point release) goes in now, while
    // nothing runs: two renames, no wait (`browser-plan` §16).
    crate::browser_kit::swap_in_prepared(&d);
    prepare(&d, xpi, server)?;
    let started = std::process::Command::new(firefox_bin(&d))
        .args(launch_args(&d.join("profile"), url))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string());
    // And the next one is prepared in the background, after the browser is up.
    crate::browser_kit::prepare_in_background(d);
    started
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
        "pinned": crate::browser_kit::pin().version,
        "update_ready": d.as_deref().is_some_and(|d| d.join("firefox.next").join("firefox").is_file()),
        "web_push": prefs.web_push,
        "running": d.as_deref().is_some_and(crate::browser_kit::running),
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
            if let Some(b) = v.get("web_push").and_then(Value::as_bool) {
                p.web_push = b;
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
        // uBlock Origin is an offer, off by default, and never forbidden.
        assert!(pol["ExtensionSettings"].get(UBLOCK_ID).is_none());
        let off = policies(
            Path::new("/x"),
            "K",
            "http://127.0.0.1:8765",
            &Prefs { open_in_it: true, block_ads: true, web_push: false },
        );
        assert_eq!(
            off["policies"]["ExtensionSettings"][UBLOCK_ID]["installation_mode"],
            "normal_installed"
        );
    }

    #[test]
    fn the_privacy_defaults_are_defaults_and_only_the_add_on_line_is_locked() {
        // user.js forces nothing any more: Firefox would apply it over the person's own choices.
        assert!(!user_js().contains("user_pref"));
        let cfg = include_str!("../../../browser/kit/formicaria.cfg");
        for want in [
            "defaultPref(\"browser.contentblocking.category\", \"strict\")",
            "defaultPref(\"dom.security.https_only_mode\", true)",
            "defaultPref(\"privacy.globalprivacycontrol.enabled\", true)",
            "lockPref(\"xpinstall.signatures.required\", false)",
        ] {
            assert!(cfg.contains(want), "missing {want}");
        }
        assert_eq!(cfg.matches("lockPref(").count(), 1, "only the add-on line is locked");
        // Safe Browsing stays as Firefox ships it: on (owner, 2026-11-15).
        assert!(!cfg.contains("safebrowsing"));
    }

    #[test]
    fn mozillas_services_are_stripped_but_sync_and_add_ons_kept() {
        let p = policies(Path::new("/x"), "K", "http://127.0.0.1:8765", &Prefs::default());
        let pol = &p["policies"];
        for off in
            ["DisableTelemetry", "DisableFirefoxStudies", "DisablePocket", "DisableAppUpdate"]
        {
            assert_eq!(pol[off], true, "{off}");
        }
        assert_eq!(pol["UserMessaging"]["FirefoxLabs"], false);
        assert_eq!(pol["GenerativeAI"]["Enabled"], false);
        assert_eq!(pol["EncryptedMediaExtensions"]["Enabled"], true);
        // Kept, by the owner's choice: the Mozilla account and Sync.
        assert!(pol.get("DisableFirefoxAccounts").is_none());
        // Web push: a switch, off by default, and only a default.
        assert_eq!(pol["Preferences"]["dom.push.enabled"]["Value"], false);
        assert_eq!(pol["Preferences"]["dom.push.enabled"]["Status"], "default");
        let on = policies(
            Path::new("/x"),
            "K",
            "http://127.0.0.1:8765",
            &Prefs { web_push: true, ..Prefs::default() },
        );
        assert_eq!(on["policies"]["Preferences"]["dom.push.enabled"]["Value"], true);
    }

    #[test]
    fn only_an_extension_origin_is_remembered() {
        assert!(!is_extension_origin("https://evil.example"));
        assert!(!is_extension_origin("moz-extension://"));
        assert!(!is_extension_origin("moz-extension://x y"));
        assert!(!is_extension_origin("moz-extension://a.b"));
        assert!(is_extension_origin("moz-extension://3f2a-09bc"));
    }

    #[test]
    fn our_button_is_pinned_to_the_toolbar() {
        let p = policies(Path::new("/x"), "K", "http://127.0.0.1:8765", &Prefs::default());
        assert_eq!(p["policies"]["ExtensionSettings"][ADDON_ID]["default_area"], "navbar");
    }

    #[test]
    fn the_window_is_named_formicaria() {
        let a = launch_args(Path::new("/p"), "http://127.0.0.1:8765");
        let a: Vec<String> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
        assert_eq!(
            a,
            [
                "--name",
                "formicaria",
                "--class",
                "formicaria",
                "--profile",
                "/p",
                "http://127.0.0.1:8765"
            ]
        );
    }

    #[test]
    fn each_add_on_build_has_its_own_file_name() {
        assert_eq!(xpi_name(b"one"), xpi_name(b"one"));
        assert_ne!(xpi_name(b"one"), xpi_name(b"two"));
        assert!(xpi_name(b"one").starts_with("formicaria-") && xpi_name(b"one").ends_with(".xpi"));
    }

    #[test]
    fn keys_compare_whole() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "ab"));
    }
}
