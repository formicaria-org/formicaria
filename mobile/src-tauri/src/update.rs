//! Updating formicaria on Android.
//!
//! **The same questions as the desktop, with the same answers.** Whether a version is newer, what a
//! signed release manifest says and whether to trust it all come from `fm_update`, where the list of
//! trusted keys exists exactly once (`decisions.md#toolchain`, 2026-09-11). What is shaped like a
//! phone is only the last step. A desktop replaces its own folder and restarts; a phone cannot,
//! because only Android's package installer may replace an app. So this downloads and verifies, and
//! `MainActivity`'s `UpdateInstaller` bridge hands the result to the installer when the person presses
//! Install (`ci/android-inject-service.sh`).
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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// How far a download has got. **In memory only** — a "downloading" written to disk would outlive the
/// process that meant it.
struct Progress {
    stage: &'static str,
    done: u64,
    total: Option<u64>,
    error: Option<String>,
}

static CHECKING: AtomicBool = AtomicBool::new(false);
static ERROR: Mutex<Option<String>> = Mutex::new(None);
static PROGRESS: Mutex<Option<Progress>> = Mutex::new(None);
/// **Cancel is a generation counter**, as on the desktop and in the assistant's model download:
/// stopping bumps it, and the running download checks on every chunk that it is still the live one.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// `<config>/formicaria/update.json`, the same name as on the desktop. `config_dir()` answers from
/// `FM_CONFIG_DIR`, which `configure_paths` sets from Android's app-data directory.
fn settings_path() -> Option<PathBuf> {
    Some(fm_app::vaults::config_dir()?.join("formicaria").join("update.json"))
}

/// **The one file the installer bridge will install.** `app_cache_dir()` is `activity.cacheDir`, which
/// is exactly what the manifest's `FileProvider` shares (`cache-path "."`). `MainActivity` reads the
/// same literal path — if one of them moves, the other must move with it.
fn apk(cache: &Path) -> PathBuf {
    cache.join("update").join("formicaria.apk")
}

/// Which version the finished download holds. Written only after its checksum matched.
fn apk_version(cache: &Path) -> PathBuf {
    cache.join("update").join("formicaria.apk.version")
}

/// Why this copy cannot look for a newer version, or `None`. A reason, never a bool.
fn cannot_check() -> Option<String> {
    if Version::running().is_none() {
        return Some(
            "This copy was built from source rather than installed from a release, so it has no \
             version to compare."
                .into(),
        );
    }
    if fm_update::target().is_none() {
        return Some("There is no download published for this kind of phone.".into());
    }
    None
}

fn status() -> serde_json::Value {
    let why = cannot_check();
    let s = settings_path().map(|p| fm_update::settings_at(&p)).unwrap_or_default();
    let available = s
        .last_seen
        .as_deref()
        .and_then(Version::parse)
        .filter(|v| fm_update::is_newer(v, s.rejected.as_deref()));
    let progress = PROGRESS.lock().ok().and_then(|p| {
        p.as_ref().map(
            |p| json!({ "stage": p.stage, "done": p.done, "total": p.total, "error": p.error }),
        )
    });
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
        "progress": progress,
        "page": fm_update::RELEASES_PAGE,
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
        .and_then(|p| fm_update::check_at(&p, force));
    set_error(result.err());
    CHECKING.store(false, Ordering::SeqCst);
}

/// Answer one of the update commands, or `None` when `cmd` is not one of them.
///
/// **Matched by exact name, never by prefix.** `update_body` — saving a note — starts with `update_`
/// too, so a prefix match here would swallow every edit made on the phone. `ci/checks.sh` refuses one.
pub fn handle(cmd: &str, args: &serde_json::Value, cache: &Path) -> Option<Result<String, String>> {
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
        "update_start" => answer(start(cache.to_path_buf())),
        "update_cancel" => {
            GENERATION.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut p) = PROGRESS.lock() {
                *p = None;
            }
            Ok(status().to_string())
        }
        "update_apply" => {
            Err("On a phone, Android's installer finishes an update — press Install.".into())
        }
        "update_rollback" => Err(
            "Android can only put an earlier version back by uninstalling formicaria first, which \
             would delete the notes kept inside it — so going back is not offered on a phone."
                .into(),
        ),
        _ => return None,
    })
}

fn start(cache: PathBuf) -> Result<(), String> {
    if let Some(why) = cannot_check() {
        return Err(why);
    }
    let path = settings_path().ok_or("this phone has nowhere to keep settings")?;
    let s = fm_update::settings_at(&path);
    let tag = s
        .last_seen
        .filter(|t| {
            Version::parse(t).is_some_and(|v| fm_update::is_newer(&v, s.rejected.as_deref()))
        })
        .ok_or("there is nothing newer to get")?;

    // **Refuse before taking a generation, never after.** Taking it first would cancel the download
    // already running and then report that one is running — which is what the desktop did.
    {
        let mut p = PROGRESS.lock().map_err(|_| "the download state is unavailable".to_string())?;
        if p.as_ref().is_some_and(|p| p.stage != "ready" && p.stage != "failed") {
            return Err("this is already being downloaded".into());
        }
        *p = Some(Progress { stage: "manifest", done: 0, total: None, error: None });
    }
    let gen = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    std::thread::spawn(move || {
        let live = || GENERATION.load(Ordering::SeqCst) == gen;
        let set = |stage: &'static str, done: u64, total: Option<u64>| {
            if live() {
                if let Ok(mut p) = PROGRESS.lock() {
                    *p = Some(Progress { stage, done, total, error: None });
                }
            }
        };
        let result = (|| -> Result<u64, String> {
            // Fetch, verify the signature, refuse anything not newer — the same sequence, against the
            // same keys, as the desktop.
            let manifest = fm_update::fetch_verified_manifest(&tag)?;
            let target =
                fm_update::target().ok_or("there is no download for this kind of phone")?;
            let entry = manifest.entry(target).ok_or_else(|| {
                format!("{} has no download for this kind of phone", manifest.version)
            })?;
            if !live() {
                return Err("stopped".into());
            }
            let dest = apk(&cache);
            if let Some(dir) = dest.parent() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("could not prepare the download: {e}"))?;
            }
            let size = entry.size;
            set("download", 0, Some(size));
            // `fetch` re-verifies a file already at `dest` against this checksum and replaces it if it
            // does not match, so an older download left here cannot satisfy this one by accident.
            fm_fetch::fetch(
                &fm_fetch::Download {
                    url: &fm_update::asset(&tag, &entry.file),
                    dest: &dest,
                    sha256: Some(&entry.sha256),
                },
                &|done, total| set("download", done, total.or(Some(size))),
                &|| !live(),
            )
            .map_err(|e| e.to_string())?;
            let _ = std::fs::write(apk_version(&cache), manifest.version.to_string());
            Ok(size)
        })();
        match result {
            Ok(size) => set("ready", size, Some(size)),
            Err(msg) => {
                if live() {
                    if let Ok(mut p) = PROGRESS.lock() {
                        *p = Some(Progress {
                            stage: "failed",
                            done: 0,
                            total: None,
                            error: Some(msg),
                        });
                    }
                }
            }
        }
    });
    Ok(())
}

/// At launch: remove a download that has already been installed, then look for a newer version in the
/// background.
///
/// **An installed update leaves its ~30 MB APK in the cache** until something removes it, and the only
/// honest test that it was installed is that this build is now that version or newer — which is what
/// the sidecar is for. A download that was *not* installed is left alone, so pressing Install again
/// needs no second download.
///
/// The check is silent on failure: a phone that cannot reach the network at launch is not news, and an
/// error on every offline start is how people learn to dismiss errors.
pub fn on_start(cache: Option<PathBuf>) {
    if let Some(cache) = cache {
        let installed = std::fs::read_to_string(apk_version(&cache))
            .ok()
            .and_then(|t| Version::parse(t.trim()))
            .is_some_and(|v| Version::running().is_some_and(|cur| cur >= v));
        if installed {
            let _ = std::fs::remove_file(apk(&cache));
            let _ = std::fs::remove_file(apk_version(&cache));
        }
    }
    if cannot_check().is_some()
        || !settings_path().is_some_and(|p| fm_update::settings_at(&p).check)
    {
        return;
    }
    std::thread::spawn(|| {
        // Let the app finish starting first; nothing here is urgent enough to compete with it.
        std::thread::sleep(std::time::Duration::from_secs(5));
        check(false);
    });
}
