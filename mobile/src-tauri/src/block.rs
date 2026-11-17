//! **Ad and tracker blocking in the phone's page views** (`decisions.md` 2026-11-17; plan part B).
//!
//! The engine is `fm-block` (Brave's `adblock-rust`); this keeps its lists and answers the page views
//! through JNI (`Blocker` in `PagePrivacy.kt`), which ask for every request a page makes: an HTTP
//! round trip per request would be too slow. The lists are the community's (EasyPrivacy for
//! trackers, EasyList for ads), **fetched on the phone from their publishers** and refreshed weekly,
//! never shipped in the app. The engine built from them is kept, so a start does not parse them again.

use fm_block::Blocker;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

const TRACKERS: (&str, &str) = ("easyprivacy", "https://easylist.to/easylist/easyprivacy.txt");
const ADS: (&str, &str) = ("easylist", "https://easylist.to/easylist/easylist.txt");
const WEEK: Duration = Duration::from_secs(7 * 24 * 3600);

static ENGINE: RwLock<Option<Arc<Blocker>>> = RwLock::new(None);
/// The lists wanted now (`trackers`, `ads`), so a slow build for an older choice is not installed.
static WANT: Mutex<(bool, bool)> = Mutex::new((false, false));

fn dir() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("blocklists"))
}

fn fresh(p: &std::path::Path) -> bool {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < WEEK)
}

/// A list's text: the kept copy, fetched again when a week old (the old copy kept if that fails).
fn list(d: &std::path::Path, (name, url): (&str, &str)) -> Option<String> {
    let p = d.join(format!("{name}.txt"));
    #[cfg(feature = "update")]
    if !fresh(&p) {
        if let Ok(bytes) = fm_fetch::get(url, 20 * 1024 * 1024) {
            let tmp = p.with_extension("new");
            if std::fs::write(&tmp, &bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }
    #[cfg(not(feature = "update"))]
    let _ = (url, fresh(&p));
    std::fs::read_to_string(&p).ok()
}

/// The switches changed (or the app started): build the engine for them, on its own thread.
pub fn configure(trackers: bool, ads: bool) {
    if let Ok(mut w) = WANT.lock() {
        if *w == (trackers, ads) && ENGINE.read().map(|e| e.is_some() == (trackers || ads)).unwrap_or(false) {
            return;
        }
        *w = (trackers, ads);
    }
    if !(trackers || ads) {
        if let Ok(mut e) = ENGINE.write() {
            *e = None;
        }
        return;
    }
    std::thread::spawn(move || {
        let Some(d) = dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let key = format!("engine-{}{}.bin", u8::from(trackers), u8::from(ads));
        let cached = d.join(&key);
        let lists_fresh = [(trackers, TRACKERS), (ads, ADS)]
            .iter()
            .filter(|(on, _)| *on)
            .all(|(_, (n, _))| fresh(&d.join(format!("{n}.txt"))));
        let built = if lists_fresh && fresh(&cached) {
            std::fs::read(&cached).ok().and_then(|b| Blocker::load(&b))
        } else {
            None
        };
        let engine = built.or_else(|| {
            let texts: Vec<String> = [(trackers, TRACKERS), (ads, ADS)]
                .iter()
                .filter(|(on, _)| *on)
                .filter_map(|(_, l)| list(&d, *l))
                .collect();
            if texts.is_empty() {
                return None;
            }
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            let b = Blocker::new(&refs);
            let _ = std::fs::write(&cached, b.save());
            Some(b)
        });
        if WANT.lock().map(|w| *w == (trackers, ads)).unwrap_or(false) {
            if let Ok(mut e) = ENGINE.write() {
                *e = engine.map(Arc::new);
            }
        }
    });
}

fn engine() -> Option<Arc<Blocker>> {
    ENGINE.read().ok().and_then(|e| e.clone())
}

fn text(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s).map(|s| s.into()).unwrap_or_default()
}

fn out(env: &mut JNIEnv, s: &str) -> jstring {
    env.new_string(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_configure(
    _env: JNIEnv,
    _class: JClass,
    trackers: jboolean,
    ads: jboolean,
) {
    configure(trackers == JNI_TRUE, ads == JNI_TRUE);
}

/// Should this request be refused?
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_check(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    page: JString,
    kind: JString,
) -> jboolean {
    let Some(e) = engine() else { return JNI_FALSE };
    let (u, p, k) = (text(&mut env, &url), text(&mut env, &page), text(&mut env, &kind));
    if e.check(&u, &p, &k) { JNI_TRUE } else { JNI_FALSE }
}

/// The page's own hide rules, as a style sheet.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_hide(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let css = engine().map(|e| e.hide(&text(&mut env, &url))).unwrap_or_default();
    out(&mut env, &css)
}

/// The generic hide rules for the class and id names a page uses (space-separated).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_hideNames(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    classes: JString,
    ids: JString,
) -> jstring {
    let split = |s: String| s.split_whitespace().take(5000).map(String::from).collect::<Vec<_>>();
    let (u, c, i) = (text(&mut env, &url), split(text(&mut env, &classes)), split(text(&mut env, &ids)));
    let css = engine().map(|e| e.hide_names(&u, &c, &i)).unwrap_or_default();
    out(&mut env, &css)
}

/// The phone-wide tracker block's question: is this address refused?
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_hostBlocked(
    mut env: JNIEnv,
    _class: JClass,
    host: JString,
) -> jboolean {
    let Some(e) = engine() else { return JNI_FALSE };
    if e.host_blocked(&text(&mut env, &host)) { JNI_TRUE } else { JNI_FALSE }
}
