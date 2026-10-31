//! Updating formicaria on Android.
//!
//! **The same questions as the desktop, with the same answers.** Whether a version is newer, what a
//! signed release manifest says and whether to trust it all come from `fm_update`, where the list of
//! trusted keys exists exactly once (`decisions.md#toolchain`, 2026-09-11). What is shaped like a
//! phone is only the last step. A desktop replaces its own folder and restarts; a phone cannot,
//! because only Android's package installer may replace an app. **So this only looks**: pressing
//! Update has `MainActivity`'s `UpdateDownload` bridge open the newest APK in the browser, and
//! Android's installer takes it from there (`decisions.md#track-m`, 2026-10-31, *a phone update
//! opens in the browser* — the app no longer declares `REQUEST_INSTALL_PACKAGES`).
//!
//! **No going back on a phone.** Android installs an older version only after uninstalling the newer
//! one, and uninstalling deletes the app's private storage — which is where the notes live. A button
//! that offered it would be offering to delete somebody's notebook, so `update_rollback` refuses and
//! the panel never shows the row here.
//!
//! Compiled only under `cfg(update_shell)`: the `update` feature, on Android. iOS cannot install an app
//! either and has no bridge, so it answers a status that hides the rows until notify-only is built.

use fm_update::Version;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static CHECKING: AtomicBool = AtomicBool::new(false);
/// The app store that installed this copy, if one did — read once from what `MainActivity` wrote.
static STORE: OnceLock<Option<&'static str>> = OnceLock::new();
static ERROR: Mutex<Option<String>> = Mutex::new(None);

/// `<config>/formicaria/update.json`, the same name as on the desktop. `config_dir()` answers from
/// `FM_CONFIG_DIR`, which `configure_paths` sets from Android's app-data directory.
fn settings_path() -> Option<PathBuf> {
    Some(fm_app::vaults::config_dir()?.join("formicaria").join("update.json"))
}

/// Where the app kept its own downloaded update before 2026-10-31. Removed at launch: an APK left
/// there by an older version is ~48 MB nothing will ever install.
fn old_download(cache: &Path) -> PathBuf {
    cache.join("update")
}

/// **Which app installed this one**, as Android reports it (the installer of record). `MainActivity`
/// writes it at every start, before the Rust side runs; it reads the same literal path — if one of
/// them moves, the other must move with it. Empty when Android names nobody (a cable install).
fn installer_file(cache: &Path) -> PathBuf {
    cache.join("installer")
}

/// Read who installed this copy, once per run. A store keeps the app up to date itself, so the
/// GitHub check starts off for a store install and the row says who updates it (`decisions.md`
/// 2026-10-30, *the Android update check follows who installed the app*). Anything else — the
/// website, this app's own updater, a cable — keeps today's default: on.
fn note_installer(cache: &Path) {
    STORE.get_or_init(|| {
        std::fs::read_to_string(installer_file(cache)).ok().and_then(|t| fm_update::store_name(&t))
    });
}

/// The store that installed this copy, if one did.
fn store() -> Option<&'static str> {
    STORE.get().copied().flatten()
}

/// What the check switch is when the person never set it: off for a store install, on otherwise.
fn check_default() -> bool {
    store().is_none()
}

/// Why this copy cannot look for a newer version, or `None`. A reason, never a bool.
/// Why this copy cannot check, if it cannot. **A build with no version can** (2026-10-09): it is
/// offered the latest release, so a test build put on by cable can get back in line from Settings.
fn cannot_check() -> Option<String> {
    if fm_update::target().is_none() {
        return Some("There is no download published for this kind of phone.".into());
    }
    None
}

fn status() -> serde_json::Value {
    let why = cannot_check();
    let s = settings_path()
        .map(|p| fm_update::settings_or(&p, check_default()))
        .unwrap_or(fm_update::Settings { check: check_default(), ..Default::default() });
    let available = s
        .last_seen
        .as_deref()
        .and_then(Version::parse)
        .filter(|v| fm_update::is_newer(v, s.rejected.as_deref()));
    // **The same keys as fm-serve's `/api/update_status`**, because the same Settings panel renders
    // both. `ci/checks.sh` holds the two lists together.
    json!({
        "can_check": why.is_none(),
        "can_install": why.is_none(),
        "why": why.unwrap_or_default(),
        "current": Version::running().map(|v| v.to_string()),
        "available": available.map(|v| v.to_string()),
        "previous": serde_json::Value::Null,
        "can_go_back": false,
        "checking": CHECKING.load(Ordering::SeqCst),
        "check": s.check,
        "last_check": s.last_check,
        "error": ERROR.lock().ok().and_then(|e| e.clone()),
        "progress": serde_json::Value::Null,
        "page": fm_update::RELEASES_PAGE,
        // The store that keeps this copy up to date, by name, or null (`decisions.md` 2026-10-30).
        "updated_by": store(),
    })
}

fn set_error(msg: Option<String>) {
    if let Ok(mut e) = ERROR.lock() {
        *e = msg;
    }
}

fn check(force: bool) {
    // `swap`, not load-then-store: two presses at once must make one request.
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let result = settings_path()
        .ok_or_else(|| "this phone has nowhere to keep settings".to_string())
        .and_then(|p| fm_update::check_at_or(&p, force, check_default()));
    set_error(result.err());
    CHECKING.store(false, Ordering::SeqCst);
}

/// Answer one of the update commands, or `None` when `cmd` is not one of them.
///
/// **Matched by exact name, never by prefix.** `update_body` — saving a note — starts with `update_`
/// too, so a prefix match here would swallow every edit made on the phone. `ci/checks.sh` refuses one.
pub fn handle(cmd: &str, args: &serde_json::Value, cache: &Path) -> Option<Result<String, String>> {
    note_installer(cache);
    let answer = |r: Result<(), String>| r.map(|()| status().to_string());
    Some(match cmd {
        "update_status" => Ok(status().to_string()),
        "update_check" => {
            if let Some(why) = cannot_check() {
                return Some(Err(why));
            }
            check(true);
            Ok(status().to_string())
        }
        "set_update_check" => {
            let on = args.get("check").and_then(|v| v.as_bool()).unwrap_or(false);
            answer(
                settings_path()
                    .ok_or_else(|| "this phone has nowhere to keep settings".to_string())
                    .and_then(|p| {
                        fm_update::save_at(&p, |v| v["check"] = serde_json::Value::Bool(on))
                    }),
            )
        }
        "update_start" | "update_apply" => {
            Err("On a phone, Update opens the download in your browser, and Android's installer \
             finishes it."
                .into())
        }
        "update_cancel" => Ok(status().to_string()),
        "update_rollback" => Err(
            "Android can only put an earlier version back by uninstalling formicaria first, which \
             would delete the notes kept inside it — so going back is not offered on a phone."
                .into(),
        ),
        _ => return None,
    })
}

/// At launch: remove what the old in-app download left, then look for a newer version in the
/// background.
///
/// The check is silent on failure: a phone that cannot reach the network at launch is not news, and an
/// error on every offline start is how people learn to dismiss errors.
pub fn on_start(cache: Option<PathBuf>) {
    if let Some(cache) = cache {
        note_installer(&cache);
        let _ = std::fs::remove_dir_all(old_download(&cache));
    }
    if cannot_check().is_some()
        || !settings_path().is_some_and(|p| fm_update::settings_or(&p, check_default()).check)
    {
        return;
    }
    std::thread::spawn(|| {
        // Let the app finish starting first; nothing here is urgent enough to compete with it.
        std::thread::sleep(std::time::Duration::from_secs(5));
        check(false);
    });
}
