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
use adblock::Engine;

pub struct Blocker {
    engine: Engine,
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

    /// Should this request be refused? `kind` is the request's type as Brave names them
    /// (`script`, `image`, `stylesheet`, `xmlhttprequest`, `sub_frame`, `media`, `other`…); a page
    /// itself (`document`) is never refused here.
    pub fn check(&self, url: &str, page: &str, kind: &str) -> bool {
        if kind == "document" {
            return false;
        }
        match Request::new(url, page, kind, "GET") {
            Ok(r) => self.engine.check_network_request(&r).should_block(),
            Err(_) => false,
        }
    }

    /// The page's hide rules as one style sheet (`display: none` for each), empty when it has none.
    pub fn hide(&self, url: &str) -> String {
        let r = self.engine.url_cosmetic_resources(url);
        if r.generichide && r.hide_selectors.is_empty() {
            return String::new();
        }
        let mut sel: Vec<&String> = r.hide_selectors.iter().collect();
        sel.sort();
        sel.iter().map(|s| format!("{s}{{display:none!important}}")).collect::<Vec<_>>().join("\n")
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

    /// Is this address one the lists refuse outright (the phone-wide block, by address alone)?
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

    #[test]
    fn the_engine_is_kept_between_starts() {
        let b = Blocker::new(&[LIST]);
        let again = Blocker::load(&b.save()).expect("read back");
        assert!(again.check("https://ads.example/x.js", "https://news.example/", "script"));
        assert!(Blocker::load(b"not an engine").is_none());
    }
}
