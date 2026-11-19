//! **Ad and tracker blocking for the phone's web pages** (`decisions.md` 2026-11-17, *privacy
//! switches, blocking, and media from any site*; plan part B).
//!
//! The community's filter lists (EasyList for ads, EasyPrivacy for trackers; Adblock Plus syntax)
//! read by Brave's `adblock-rust`. **Pure**: the lists come in as text, the answers go out; fetching
//! and keeping the lists is the shell's. The page view asks [`Blocker::check`] for every request and
//! [`Blocker::hide`] for the page's hide rules; the phone-wide tracker block asks
//! [`Blocker::host_blocked`] for an address being looked up.

use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::resources::Resource;
use adblock::Engine;

pub mod debounce;
pub mod dns;
pub mod hosts;
pub mod lists;

pub struct Blocker {
    engine: Engine,
}

/// **What to do with a request** (`plan` 3.2): let it through, refuse it, or answer it with a
/// harmless stand-in (a no-op script, an empty picture) so a page that waits for it does not break.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Block,
    /// The stand-in's media type and bytes.
    StandIn(String, Vec<u8>),
}

impl Blocker {
    /// An engine for these lists (their text, as published).
    pub fn new(lists: &[&str]) -> Blocker {
        let mut set = FilterSet::new(false);
        for l in lists {
            set.add_filter_list(l.to_string(), ParseOptions::default());
        }
        Blocker { engine: Engine::new_with_filter_set(set) }
    }

    /// **The stand-ins and scriptlets** (uBlock Origin's resources, as `adblock-rust` reads them: a
    /// JSON array of `{name, aliases, kind, content}`). Not kept by [`Blocker::save`]: given again
    /// after [`Blocker::load`]. A text that cannot be read leaves the engine without them.
    pub fn use_resources(&mut self, json: &str) -> usize {
        match serde_json::from_str::<Vec<Resource>>(json) {
            Ok(r) => {
                let n = r.len();
                self.engine.use_resources(r);
                n
            }
            Err(_) => 0,
        }
    }

    /// Should this request be refused? `kind` is the request's type as Brave names them
    /// (`script`, `image`, `stylesheet`, `xmlhttprequest`, `sub_frame`, `media`, `other`…); a page
    /// itself (`document`) is never refused here.
    pub fn check(&self, url: &str, page: &str, kind: &str) -> bool {
        self.verdict(url, page, kind) != Verdict::Allow
    }

    /// The request's [`Verdict`]: a refused request whose rule names a stand-in is answered with it.
    pub fn verdict(&self, url: &str, page: &str, kind: &str) -> Verdict {
        if kind == "document" {
            return Verdict::Allow;
        }
        let Ok(r) = Request::new(url, page, kind, "GET") else { return Verdict::Allow };
        let res = self.engine.check_network_request(&r);
        if !res.should_block() {
            return Verdict::Allow;
        }
        res.redirect
            .as_deref()
            .and_then(data_url)
            .map_or(Verdict::Block, |(m, b)| Verdict::StandIn(m, b))
    }

    /// **The address a link should really open** (`plan` 3.5): the lists' `$removeparam` rules
    /// applied (tracking parts taken off). `None` when nothing changes.
    pub fn clean(&self, url: &str) -> Option<String> {
        let r = Request::new(url, url, "document", "GET").ok()?;
        self.engine.check_network_request(&r).rewritten_url.filter(|u| u != url)
    }

    /// The page's own hide rules as one style sheet (`display: none` for each), empty when it has none.
    pub fn hide(&self, url: &str) -> String {
        let r = self.engine.url_cosmetic_resources(url);
        if r.generichide && r.hide_selectors.is_empty() {
            return String::new();
        }
        let mut sel: Vec<&String> = r.hide_selectors.iter().collect();
        sel.sort();
        sel.iter().map(|s| format!("{s}{{display:none!important}}")).collect::<Vec<_>>().join("\n")
    }

    /// **The site's scriptlets** (`plan` 3.3): the script the lists inject into this page at its
    /// start (empty when none, or when no resources were given).
    pub fn scriptlets(&self, url: &str) -> String {
        self.engine.url_cosmetic_resources(url).injected_script
    }

    /// **The site's procedural hide rules** (`plan` 3.4) as one JSON array, each item as the engine
    /// gives it (`{"selector": [...], "action": ...}`); `[]` when none.
    pub fn procedural(&self, url: &str) -> String {
        let mut p: Vec<String> =
            self.engine.url_cosmetic_resources(url).procedural_actions.into_iter().collect();
        p.sort();
        format!("[{}]", p.join(","))
    }

    /// **The generic hide rules for what the page holds** (`##.ad`, `###banner`): the class and id
    /// names the page uses go in, the matching selectors come out as a style sheet. The page asks
    /// again as it changes. Nothing when the page's site is excepted from generic hiding.
    pub fn hide_names(&self, url: &str, classes: &[String], ids: &[String]) -> String {
        let r = self.engine.url_cosmetic_resources(url);
        if r.generichide {
            return String::new();
        }
        let mut sel = self.engine.hidden_class_id_selectors(classes, ids, &r.exceptions);
        sel.sort();
        sel.dedup();
        sel.iter().map(|s| format!("{s}{{display:none!important}}")).collect::<Vec<_>>().join("\n")
    }

    /// Is this address one the lists refuse outright, by address alone?
    pub fn host_blocked(&self, host: &str) -> bool {
        let url = format!("https://{host}/");
        // As a third party's request: what a tracker inside an app is.
        self.check(&url, "https://formicaria.invalid/", "other")
    }

    /// The engine, kept between starts so they do not parse the lists again.
    pub fn save(&self) -> Vec<u8> {
        self.engine.serialize()
    }

    /// An engine kept by [`Blocker::save`]; `None` if it cannot be read (a newer engine: rebuild).
    pub fn load(bytes: &[u8]) -> Option<Blocker> {
        let mut engine = Engine::default();
        engine.deserialize(bytes).ok()?;
        Some(Blocker { engine })
    }
}

/// `data:<type>;base64,<bytes>` → the type and the bytes.
fn data_url(u: &str) -> Option<(String, Vec<u8>)> {
    use base64::Engine as _;
    let rest = u.strip_prefix("data:")?;
    let (head, body) = rest.split_once(',')?;
    let mime = head.strip_suffix(";base64")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(body).ok()?;
    Some((mime.to_string(), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
! An invented list, in EasyList's syntax.
||ads.example^
||track.example^$third-party
/banner/*$image
@@||ads.example/allowed.js
news.example##.ad-slot
##.sponsored-box
";

    #[test]
    fn requests_are_refused_by_the_lists_and_their_exceptions_kept() {
        let b = Blocker::new(&[LIST]);
        assert!(b.check("https://ads.example/x.js", "https://news.example/", "script"));
        assert!(!b.check("https://ads.example/allowed.js", "https://news.example/", "script"));
        assert!(b.check("https://track.example/p.gif", "https://news.example/", "image"));
        assert!(
            !b.check("https://track.example/", "https://track.example/", "script"),
            "first party"
        );
        assert!(b.check("https://cdn.example/banner/1.png", "https://news.example/", "image"));
        assert!(!b.check("https://cdn.example/story.png", "https://news.example/", "image"));
        assert!(
            !b.check("https://ads.example/", "https://ads.example/", "document"),
            "a page is never refused"
        );
    }

    #[test]
    fn a_page_gets_its_hide_rules() {
        let b = Blocker::new(&[LIST]);
        let css = b.hide("https://news.example/story");
        assert!(css.contains(".ad-slot{display:none!important}"), "{css}");
        assert!(!b.hide("https://other.example/").contains(".ad-slot"));
        // A generic rule reaches a page that uses the class.
        let g =
            b.hide_names("https://other.example/", &["sponsored-box".into(), "story".into()], &[]);
        assert_eq!(g, ".sponsored-box{display:none!important}");
        assert!(b.hide_names("https://other.example/", &["story".into()], &[]).is_empty());
    }

    #[test]
    fn an_address_is_refused_for_the_whole_phone() {
        let b = Blocker::new(&[LIST]);
        assert!(b.host_blocked("ads.example"));
        assert!(b.host_blocked("pixel.track.example"));
        assert!(!b.host_blocked("news.example"));
    }

    /// An invented resource set, in the shape uBlock Origin's are given (`content` in base64).
    const RESOURCES: &str = r#"[
      {"name":"noop.js","aliases":["noopjs"],"kind":{"mime":"application/javascript"},"content":"KGZ1bmN0aW9uKCl7fSkoKTs="},
      {"name":"set-constant.js","aliases":["set.js"],"kind":"template","content":"d2luZG93Wyd7ezF9fSddID0gJ3t7Mn19Jzs="}
    ]"#;

    #[test]
    fn a_refused_script_with_a_stand_in_is_answered_by_it() {
        let mut b = Blocker::new(&[
            "||ads.example/lib.js$script,redirect=noop.js\n||ads.example/pixel.gif\n",
        ]);
        assert_eq!(b.use_resources(RESOURCES), 2);
        assert_eq!(
            b.verdict("https://ads.example/lib.js", "https://news.example/", "script"),
            Verdict::StandIn("application/javascript".into(), b"(function(){})();".to_vec())
        );
        assert_eq!(
            b.verdict("https://ads.example/pixel.gif", "https://news.example/", "image"),
            Verdict::Block
        );
        assert_eq!(
            b.verdict("https://cdn.example/app.js", "https://news.example/", "script"),
            Verdict::Allow
        );
        assert_eq!(b.use_resources("not json"), 0, "an unreadable set is refused, not half-read");
    }

    #[test]
    fn a_site_gets_its_scriptlets_and_its_procedural_rules() {
        let mut b = Blocker::new(&[
            "news.example##+js(set, adsOn, false)\nnews.example##div:has-text(Sponsored)\n",
        ]);
        b.use_resources(RESOURCES);
        let s = b.scriptlets("https://news.example/story");
        assert!(s.contains("adsOn") && s.contains("false"), "{s}");
        assert!(b.scriptlets("https://other.example/").is_empty());
        let p: serde_json::Value =
            serde_json::from_str(&b.procedural("https://news.example/")).unwrap();
        assert_eq!(p.as_array().map(Vec::len), Some(1), "{p}");
        assert!(p.to_string().contains("has-text"), "{p}");
        assert_eq!(b.procedural("https://other.example/"), "[]");
    }

    #[test]
    fn a_link_loses_its_tracking_parts() {
        let b = Blocker::new(&["$removeparam=utm_source\n$removeparam=fbclid\n"]);
        assert_eq!(
            b.clean("https://news.example/a?id=7&utm_source=mail&fbclid=x").as_deref(),
            Some("https://news.example/a?id=7")
        );
        assert_eq!(b.clean("https://news.example/a?id=7"), None);
    }

    #[test]
    fn the_engine_is_kept_between_starts() {
        let b = Blocker::new(&[LIST]);
        let again = Blocker::load(&b.save()).expect("read back");
        assert!(again.check("https://ads.example/x.js", "https://news.example/", "script"));
        assert!(Blocker::load(b"not an engine").is_none());
    }
}
