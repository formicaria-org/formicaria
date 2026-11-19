//! **Ad and tracker blocking on the phone** (`decisions.md` 2026-11-17; `privacy-plan-2026-11-19.md`).
//!
//! The engine is `fm-block` (Brave's `adblock-rust`); this keeps its lists and answers the page views
//! through JNI (`Blocker` in `PagePrivacy.kt`), which ask for every request a page makes: an HTTP
//! round trip per request would be too slow. Three parts, each built on its own thread when its
//! switches change:
//!
//! - **The pages' engine**: the lists for *Block trackers*, *Block ads* and *Clean tracking links*
//!   (`fm_block::lists`), with uBlock Origin's stand-ins and scriptlets (`block/ubo-resources.json`,
//!   GPL-3.0 like the APK). Kept between starts, so a start does not parse the lists again.
//! - **The redirect unwrapper** (`fm_block::debounce`) for *Clean tracking links*.
//! - **The app blocker's names** (`fm_block::hosts`) for *Block trackers in other apps*, and the
//!   encrypted-DNS services for *Stop apps going around the block*.
//!
//! Every list is **fetched on the phone from its publisher** and refreshed by its own interval
//! (`List::refresh_days`); a list under a non-commercial licence is never in the APK.

use fm_block::debounce::Debouncer;
use fm_block::hosts::HostSet;
use fm_block::lists::{fetched_for, List, Use};
use fm_block::{dns, Blocker, Verdict};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jint, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

/// uBlock Origin's stand-ins and scriptlets (`pixi run block-resources`).
const RESOURCES: &str = include_str!("../block/ubo-resources.json");

static ENGINE: RwLock<Option<Arc<Blocker>>> = RwLock::new(None);
static DEBOUNCE: RwLock<Option<Arc<Debouncer>>> = RwLock::new(None);
static APPS: RwLock<Option<Arc<HostSet>>> = RwLock::new(None);
/// The switches wanted now, so a slow build for an older choice is not installed.
static WANT: Mutex<(bool, bool, bool)> = Mutex::new((false, false, false));
static WANT_APPS: Mutex<(bool, bool)> = Mutex::new((false, false));
/// **One build at a time** (2026-11-19): the pages' engine and the app blocker share lists
/// (EasyPrivacy, Peter Lowe's), and a start configures twice (the shell's defaults, then the app's
/// switches); two threads fetching the same list into the same place garbled it.
static BUILDING: Mutex<()> = Mutex::new(());

fn dir() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("blocklists"))
}

fn fresh(p: &Path, days: u32) -> bool {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < Duration::from_secs(u64::from(days.max(1)) * 24 * 3600))
}

fn path_of(d: &Path, l: &List) -> PathBuf {
    d.join(format!("{}.txt", l.key))
}

/// A list's text: the kept copy, fetched again when due (the old copy kept if that fails).
fn list(d: &Path, l: &List) -> Option<String> {
    let p = path_of(d, l);
    #[cfg(feature = "update")]
    if !fresh(&p, l.refresh_days) {
        if let Ok(bytes) = fm_fetch::get(l.url, 20 * 1024 * 1024) {
            let tmp = p.with_extension(format!("new{}", std::process::id()));
            if std::fs::write(&tmp, &bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }
    std::fs::read_to_string(&p).ok()
}

/// The lists for these switches, each once.
fn wanted(trackers: bool, ads: bool, links: bool) -> Vec<&'static List> {
    let mut out: Vec<&'static List> = Vec::new();
    for (on, u) in [(trackers, Use::Trackers), (ads, Use::Ads), (links, Use::Links)] {
        if on {
            for l in fetched_for(u) {
                if l.key != "debounce" && !out.iter().any(|o| o.key == l.key) {
                    out.push(l);
                }
            }
        }
    }
    out
}

/// The switches changed (or the app started): build the pages' engine and the unwrapper for them,
/// on their own thread.
pub fn configure(trackers: bool, ads: bool, links: bool) {
    let want = (trackers, ads, links);
    if let Ok(mut w) = WANT.lock() {
        let built = ENGINE.read().map(|e| e.is_some()).unwrap_or(false);
        if *w == want && built == (trackers || ads || links) {
            return;
        }
        *w = want;
    }
    if !(trackers || ads || links) {
        if let Ok(mut e) = ENGINE.write() {
            *e = None;
        }
    }
    std::thread::spawn(move || {
        let _one = BUILDING.lock().unwrap_or_else(|e| e.into_inner());
        if WANT.lock().map(|w| *w != want).unwrap_or(true) {
            return; // a newer choice came while this one waited: that one builds
        }
        let Some(d) = dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let lists = wanted(trackers, ads, links);
        let key = format!("engine-{}{}{}.bin", u8::from(trackers), u8::from(ads), u8::from(links));
        let cached = d.join(&key);
        let lists_fresh = lists.iter().all(|l| fresh(&path_of(&d, l), l.refresh_days));
        let mut engine = if lists_fresh && fresh(&cached, 7) {
            std::fs::read(&cached).ok().and_then(|b| Blocker::load(&b))
        } else {
            None
        };
        if engine.is_none() && !lists.is_empty() {
            let texts: Vec<String> = lists.iter().filter_map(|l| list(&d, l)).collect();
            if !texts.is_empty() {
                let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
                let b = Blocker::new(&refs);
                let _ = std::fs::write(&cached, b.save());
                engine = Some(b);
            }
        }
        if let Some(e) = engine.as_mut() {
            e.use_resources(RESOURCES);
        }
        let debouncer = links.then(|| {
            let text = fm_block::lists::by_key("debounce").and_then(|l| list(&d, l)).unwrap_or_default();
            Debouncer::new(&[&text])
        });
        if WANT.lock().map(|w| *w == want).unwrap_or(false) {
            if let Ok(mut e) = ENGINE.write() {
                *e = engine.map(Arc::new);
            }
            if let Ok(mut dd) = DEBOUNCE.write() {
                *dd = debouncer.map(Arc::new);
            }
        }
    });
}

/// The app blocker's switches changed: build its names (the lists for apps, and the encrypted-DNS
/// services while *Stop apps going around the block* is on).
pub fn configure_apps(apps: bool, around: bool) {
    let want = (apps, around);
    if let Ok(mut w) = WANT_APPS.lock() {
        let built = APPS.read().map(|e| e.is_some()).unwrap_or(false);
        if *w == want && built == apps {
            return;
        }
        *w = want;
    }
    if !apps {
        if let Ok(mut a) = APPS.write() {
            *a = None;
        }
        return;
    }
    std::thread::spawn(move || {
        let _one = BUILDING.lock().unwrap_or_else(|e| e.into_inner());
        if WANT_APPS.lock().map(|w| *w != want).unwrap_or(true) {
            return;
        }
        let Some(d) = dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let mut set = HostSet::new();
        let uses: &[Use] = if around { &[Use::Apps, Use::DnsServices] } else { &[Use::Apps] };
        for u in uses {
            for l in fetched_for(*u) {
                if let Some(t) = list(&d, l) {
                    set.add(&t);
                }
            }
        }
        if WANT_APPS.lock().map(|w| *w == want).unwrap_or(false) {
            if let Ok(mut a) = APPS.write() {
                *a = Some(Arc::new(set));
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

fn bytes_in(env: &mut JNIEnv, a: &JByteArray) -> Vec<u8> {
    env.convert_byte_array(a).unwrap_or_default()
}

fn bytes_out(env: &mut JNIEnv, b: &[u8]) -> jbyteArray {
    env.byte_array_from_slice(b).map(|a| a.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn on(b: bool) -> jboolean {
    if b {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_configure(
    _env: JNIEnv,
    _class: JClass,
    trackers: jboolean,
    ads: jboolean,
    links: jboolean,
) {
    configure(trackers == JNI_TRUE, ads == JNI_TRUE, links == JNI_TRUE);
}

#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_configureApps(
    _env: JNIEnv,
    _class: JClass,
    apps: jboolean,
    around: jboolean,
) {
    configure_apps(apps == JNI_TRUE, around == JNI_TRUE);
}

/// A request's verdict: `""` let through, `"-"` refused, otherwise a stand-in as
/// `<media type>\n<base64 bytes>`.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_verdict(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    page: JString,
    kind: JString,
) -> jstring {
    use base64::Engine as _;
    let Some(e) = engine() else { return out(&mut env, "") };
    let (u, p, k) = (text(&mut env, &url), text(&mut env, &page), text(&mut env, &kind));
    let v = match e.verdict(&u, &p, &k) {
        Verdict::Allow => String::new(),
        Verdict::Block => "-".into(),
        Verdict::StandIn(m, b) => format!("{m}\n{}", base64::engine::general_purpose::STANDARD.encode(b)),
    };
    out(&mut env, &v)
}

/// The address a link should really open (redirect unwrapped, tracking parts off); `""` if as is.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_cleanLink(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let u = text(&mut env, &url);
    let unwrapped = DEBOUNCE.read().ok().and_then(|d| d.clone()).and_then(|d| d.unwrap(&u));
    let base = unwrapped.clone().unwrap_or_else(|| u.clone());
    let cleaned = engine().and_then(|e| e.clean(&base)).or(unwrapped).unwrap_or_default();
    out(&mut env, &cleaned)
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
    let (u, c, i) =
        (text(&mut env, &url), split(text(&mut env, &classes)), split(text(&mut env, &ids)));
    let css = engine().map(|e| e.hide_names(&u, &c, &i)).unwrap_or_default();
    out(&mut env, &css)
}

/// The site's scriptlets (run at its pages' start), `""` when none.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_scriptlets(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let s = engine().map(|e| e.scriptlets(&text(&mut env, &url))).unwrap_or_default();
    out(&mut env, &s)
}

/// The site's procedural hide rules, a JSON array.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_procedural(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
) -> jstring {
    let s = engine().map(|e| e.procedural(&text(&mut env, &url))).unwrap_or_else(|| "[]".into());
    out(&mut env, &s)
}

/// The app blocker's question: is this name refused?
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_hostBlocked(
    mut env: JNIEnv,
    _class: JClass,
    host: JString,
) -> jboolean {
    let h = text(&mut env, &host);
    on(APPS.read().ok().and_then(|a| a.clone()).is_some_and(|a| a.contains(&h)))
}

/// What the blocking has now, as JSON: whether the pages' engine is built, the switches it was built
/// for, and each kept list's size (the Privacy sheet's "lists" line, and the phone check's evidence).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_state(mut env: JNIEnv, _class: JClass) -> jstring {
    let lists: serde_json::Map<String, serde_json::Value> = dir()
        .map(|d| {
            fm_block::lists::LISTS
                .iter()
                .filter_map(|l| {
                    let n = std::fs::metadata(path_of(&d, l)).ok()?.len();
                    Some((l.key.to_string(), serde_json::json!(n)))
                })
                .collect()
        })
        .unwrap_or_default();
    let want = WANT.lock().map(|w| *w).unwrap_or_default();
    let v = serde_json::json!({
        "engine": engine().is_some(),
        "want": { "trackers": want.0, "ads": want.1, "links": want.2 },
        "apps": APPS.read().ok().and_then(|a| a.as_ref().map(|s| s.len())),
        "lists": lists,
    });
    out(&mut env, &v.to_string())
}

/// The pages' engine is built (until then every request goes through).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_ready(_env: JNIEnv, _class: JClass) -> jboolean {
    on(engine().is_some())
}

/// The app blocker's lists are ready (until then every name is answered).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_appsReady(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    on(APPS.read().ok().is_some_and(|a| a.as_ref().is_some_and(|s| !s.is_empty())))
}

/// The name a DNS question asks for, `""` if it is not one.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_dnsName(
    mut env: JNIEnv,
    _class: JClass,
    q: JByteArray,
) -> jstring {
    let b = bytes_in(&mut env, &q);
    let n = dns::qname(&b).unwrap_or_default();
    out(&mut env, &n)
}

/// "No such name" for this question.
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_dnsRefuse(
    mut env: JNIEnv,
    _class: JClass,
    q: JByteArray,
) -> jbyteArray {
    let b = bytes_in(&mut env, &q);
    bytes_out(&mut env, &dns::refuse(&b))
}

/// The answer leads through a refused name (a tracker behind another name, by CNAME).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_dnsCloaked(
    mut env: JNIEnv,
    _class: JClass,
    answer: JByteArray,
) -> jboolean {
    let b = bytes_in(&mut env, &answer);
    let apps = APPS.read().ok().and_then(|a| a.clone());
    on(apps.is_some_and(|a| dns::cname_targets(&b).iter().any(|n| a.contains(n))))
}

/// How long an answer may be kept, seconds; -1 when it may not (a failure, or no records).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_dnsTtl(
    mut env: JNIEnv,
    _class: JClass,
    answer: JByteArray,
) -> jint {
    let b = bytes_in(&mut env, &answer);
    if dns::failed(&b) {
        return -1;
    }
    dns::min_ttl(&b).map(|t| t.min(3600) as jint).unwrap_or(-1)
}

/// A kept answer for a new question (its id).
#[no_mangle]
pub extern "system" fn Java_dev_formicaria_notes_Blocker_dnsWithId(
    mut env: JNIEnv,
    _class: JClass,
    answer: JByteArray,
    question: JByteArray,
) -> jbyteArray {
    let (a, q) = (bytes_in(&mut env, &answer), bytes_in(&mut env, &question));
    bytes_out(&mut env, &dns::with_id(&a, &q))
}
