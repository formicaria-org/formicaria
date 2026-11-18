//! **Privacy, as this device keeps it** (`decisions.md` 2026-11-17, *privacy switches, blocking, and
//! media from any site*).
//!
//! One file per device, never in a vault and never synced: `config_dir()/formicaria/privacy.json`.
//! Each protection is a switch; a site can have them all off. The computer's browser maps them onto
//! Firefox's own preferences (`browser/kit/formicaria.cfg`); the phone applies them in its page views
//! (`ci/android-inject-service.sh`). The search engines are this file's list, so every surface offers
//! the same ones.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The search engines offered, in this order: `(key, name, address with %s for the words)`.
/// Private ones only: none of them builds a profile of the person searching.
pub const ENGINES: &[(&str, &str, &str)] = &[
    ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q=%s"),
    ("startpage", "Startpage", "https://www.startpage.com/do/search?q=%s"),
    ("brave", "Brave Search", "https://search.brave.com/search?q=%s"),
    ("mojeek", "Mojeek", "https://www.mojeek.com/search?q=%s"),
    ("kagi", "Kagi", "https://kagi.com/search?q=%s"),
];

/// This device's choices. Every field has its default, so a file from an older version (or none)
/// reads as the defaults for what it does not say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Privacy {
    /// The search engine's key ([`ENGINES`]).
    pub search: String,
    /// Trackers' requests refused (EasyPrivacy on the phone; Firefox's strict protection here).
    pub trackers: bool,
    /// Ads refused (EasyList on the phone; uBlock Origin on the computer). **On by default on the
    /// phone**, where measured it costs about 5 MB and 2 µs a request
    /// (`docs/context/phone-blocking-2026-11-17.md`); **off on a computer**, where it means
    /// installing uBlock Origin (+66 % memory, 2026-11-15).
    pub ads: bool,
    /// Cookie consent pop-ups answered "reject" by themselves.
    pub cookies: bool,
    /// Tracking parameters (`utm_*`, `fbclid`…) taken off addresses.
    pub clean_links: bool,
    /// Global Privacy Control: sites are told not to sell or share.
    pub gpc: bool,
    /// Pages upgraded to `https`; a plain `http` page is asked about first.
    pub https_only: bool,
    /// The whole phone: trackers inside other apps refused (a local VPN).
    pub app_trackers: bool,
    /// The computer: DuckDuckGo Privacy Essentials on top of Firefox's tracking protection. Off by
    /// default: with it the browser measured +19 % memory, over its +10 % budget (2026-11-17).
    pub ddg: bool,
    /// The computer: **more sites, with extensions** (2026-11-18, `lean-plan-2026-11-18.md` L1).
    /// Off, cookie pop-ups and tracking links are Firefox's own work (its cookie banner reduction
    /// and its query stripping); on, Consent-O-Matic and ClearURLs are added for the sites Firefox's
    /// lists miss. Off by default: a warm start measured +23 % memory with both, against Firefox.
    pub more_sites: bool,
    /// Sites with every protection off (host names, lower case), because they broke.
    pub sites_off: Vec<String>,
    /// How many times the fire button was pressed: the browser forgets when this moves.
    pub fired: u64,
    /// Bumped on every change, so a reader applies a change once (`formicaria.cfg`).
    pub v: u64,
}

impl Default for Privacy {
    fn default() -> Self {
        Privacy {
            search: "duckduckgo".into(),
            trackers: true,
            ads: cfg!(target_os = "android"),
            cookies: true,
            clean_links: true,
            gpc: true,
            https_only: true,
            app_trackers: false,
            ddg: false,
            more_sites: false,
            sites_off: Vec::new(),
            fired: 0,
            v: 0,
        }
    }
}

fn path() -> Option<PathBuf> {
    Some(crate::vaults::config_dir()?.join("formicaria").join("privacy.json"))
}

/// This device's choices, or the defaults.
pub fn load() -> Privacy {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(p: &Privacy) -> Result<(), String> {
    let path = path().ok_or("this device has no settings folder")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// A host name as kept in `sites_off`: lower case, no scheme, path, port or `www.`.
pub fn site_of(s: &str) -> String {
    let s = s.trim().to_lowercase();
    let s = s.split("://").nth(1).unwrap_or(&s).to_string();
    let host = s.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// Apply a change: only the fields given, each checked. `site_off`/`site_on` add or take one site.
pub fn apply(mut p: Privacy, change: &Value) -> Result<Privacy, String> {
    if let Some(s) = change.get("search").and_then(Value::as_str) {
        if !ENGINES.iter().any(|(k, ..)| *k == s) {
            return Err(format!("no search engine called {s}"));
        }
        p.search = s.to_string();
    }
    for (k, slot) in [
        ("trackers", &mut p.trackers),
        ("ads", &mut p.ads),
        ("cookies", &mut p.cookies),
        ("clean_links", &mut p.clean_links),
        ("gpc", &mut p.gpc),
        ("https_only", &mut p.https_only),
        ("app_trackers", &mut p.app_trackers),
        ("ddg", &mut p.ddg),
        ("more_sites", &mut p.more_sites),
    ] {
        if let Some(b) = change.get(k).and_then(Value::as_bool) {
            *slot = b;
        }
    }
    if let Some(s) = change.get("site_off").and_then(Value::as_str) {
        let site = site_of(s);
        if site.is_empty() {
            return Err("which site?".into());
        }
        if !p.sites_off.contains(&site) {
            p.sites_off.push(site);
            p.sites_off.sort();
        }
    }
    if let Some(s) = change.get("site_on").and_then(Value::as_str) {
        let site = site_of(s);
        p.sites_off.retain(|x| *x != site);
    }
    if change.get("fire").and_then(Value::as_bool) == Some(true) {
        p.fired += 1;
    }
    p.v += 1;
    Ok(p)
}

/// Change this device's choices and keep them.
pub fn set(change: &Value) -> Result<Privacy, String> {
    let p = apply(load(), change)?;
    save(&p)?;
    Ok(p)
}

/// What the `privacy` command answers: the choices, and the engines to choose from.
pub fn answer(p: &Privacy) -> Value {
    let mut v = serde_json::to_value(p).unwrap_or(Value::Null);
    v["engines"] = ENGINES
        .iter()
        .map(|(key, name, url)| json!({ "key": key, "name": name, "url": url }))
        .collect();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_protect_and_leave_ads_to_be_measured() {
        let p = Privacy::default();
        assert_eq!(p.search, "duckduckgo");
        assert!(p.trackers && p.cookies && p.clean_links && p.gpc && p.https_only);
        assert_eq!(p.ads, cfg!(target_os = "android"));
        assert!(!p.app_trackers);
        // An older or partial file reads as the defaults for what it does not say.
        let old: Privacy = serde_json::from_str(r#"{"gpc": false}"#).unwrap();
        assert!(!old.gpc && old.trackers && old.search == "duckduckgo");
    }

    #[test]
    fn a_change_touches_only_what_it_names_and_is_checked() {
        let p =
            apply(Privacy::default(), &json!({ "cookies": false, "search": "mojeek" })).unwrap();
        assert!(!p.cookies && p.trackers);
        assert_eq!(p.search, "mojeek");
        assert_eq!(p.v, 1);
        assert!(apply(p.clone(), &json!({ "search": "evil" })).is_err());
        assert!(apply(p.clone(), &json!({ "site_off": "  " })).is_err());
    }

    #[test]
    fn a_site_is_its_host_and_can_be_turned_back_on() {
        let p = apply(
            Privacy::default(),
            &json!({ "site_off": "https://www.Example.org:8443/news?x=1" }),
        )
        .unwrap();
        assert_eq!(p.sites_off, vec!["example.org".to_string()]);
        let p = apply(p, &json!({ "site_off": "example.org" })).unwrap();
        assert_eq!(p.sites_off.len(), 1, "not twice");
        let p = apply(p, &json!({ "site_on": "http://example.org/" })).unwrap();
        assert!(p.sites_off.is_empty());
    }

    #[test]
    fn the_fire_button_counts() {
        let p = apply(Privacy::default(), &json!({ "fire": true })).unwrap();
        let p = apply(p, &json!({ "fire": true })).unwrap();
        assert_eq!(p.fired, 2);
    }

    #[test]
    fn every_engine_takes_the_words_in_one_place() {
        for (k, _, url) in ENGINES {
            assert_eq!(url.matches("%s").count(), 1, "{k}");
            assert!(url.starts_with("https://"), "{k}");
        }
        assert_eq!(answer(&Privacy::default())["engines"][0]["key"], "duckduckgo");
    }
}
