//! **Every list formicaria's protections use, in one place** (`privacy-plan-2026-11-19.md` 2.8):
//! where it comes from, under what licence, how often it is fetched, what it serves, and whether the
//! APK carries it or the device fetches it. The phone's shell reads its addresses from here;
//! `THIRD-PARTY.md` names the same lists, and a test keeps the two in step.
//!
//! **Shipped or fetched.** The APK is GPL-3.0 (`decisions.md` 2026-11-17, later). A list under a
//! non-commercial licence (DuckDuckGo's, Peter Lowe's) is never in it: the phone fetches it from its
//! publisher, for its own use, like a browser fetches a page.

/// What a list is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// Ads, in the phone's page views (the *Block ads* switch).
    Ads,
    /// Trackers, in the phone's page views (the *Block trackers* switch).
    Trackers,
    /// Tracking parts of links (`$removeparam`) and tracker redirects (*Clean tracking links*).
    Links,
    /// The phone-wide block of trackers inside other apps.
    Apps,
    /// The names of public encrypted-DNS services (*Stop apps going around the block*).
    DnsServices,
}

#[derive(Debug, Clone, Copy)]
pub struct List {
    /// The file name it is kept under.
    pub key: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub licence: &'static str,
    /// Days between fetches.
    pub refresh_days: u32,
    pub uses: &'static [Use],
    /// Carried in the APK (generated at build), rather than fetched by the device.
    pub shipped: bool,
}

pub const LISTS: &[List] = &[
    List {
        key: "easyprivacy",
        name: "EasyPrivacy",
        url: "https://easylist.to/easylist/easyprivacy.txt",
        licence: "GPL-3.0 / CC BY-SA 3.0",
        refresh_days: 7,
        uses: &[Use::Trackers, Use::Apps],
        shipped: false,
    },
    List {
        key: "easylist",
        name: "EasyList",
        url: "https://easylist.to/easylist/easylist.txt",
        licence: "GPL-3.0 / CC BY-SA 3.0",
        refresh_days: 7,
        uses: &[Use::Ads],
        shipped: false,
    },
    List {
        key: "ubo-filters",
        name: "uBlock filters",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/filters.min.txt",
        licence: "GPL-3.0",
        refresh_days: 5,
        uses: &[Use::Ads],
        shipped: false,
    },
    List {
        key: "ubo-privacy",
        name: "uBlock filters – Privacy",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/privacy.min.txt",
        licence: "GPL-3.0",
        refresh_days: 7,
        uses: &[Use::Trackers],
        shipped: false,
    },
    List {
        key: "ubo-unbreak",
        name: "uBlock filters – Unbreak",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/unbreak.min.txt",
        licence: "GPL-3.0",
        refresh_days: 5,
        uses: &[Use::Ads, Use::Trackers],
        shipped: false,
    },
    List {
        key: "ubo-quick-fixes",
        name: "uBlock filters – Quick fixes",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/quick-fixes.min.txt",
        licence: "GPL-3.0",
        refresh_days: 1,
        uses: &[Use::Ads, Use::Trackers],
        shipped: false,
    },
    List {
        key: "ubo-badware",
        name: "uBlock filters – Badware risks",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/badware.min.txt",
        licence: "GPL-3.0",
        refresh_days: 5,
        uses: &[Use::Trackers],
        shipped: false,
    },
    List {
        key: "pgl",
        name: "Peter Lowe's ad and tracking server list",
        url: "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&mimetype=plaintext",
        licence: "McRae GPL v4 (non-commercial); fetched, never shipped",
        refresh_days: 7,
        uses: &[Use::Ads, Use::Apps],
        shipped: false,
    },
    List {
        key: "url-tracking",
        name: "AdGuard/uBO – URL Tracking Protection",
        url: "https://ublockorigin.github.io/uAssetsCDN/filters/privacy-removeparam.txt",
        licence: "GPL-3.0",
        refresh_days: 7,
        uses: &[Use::Links],
        shipped: false,
    },
    List {
        key: "debounce",
        name: "Brave's debounce rules",
        url: "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/debounce.json",
        licence: "MPL-2.0",
        refresh_days: 7,
        uses: &[Use::Links],
        shipped: false,
    },
    List {
        key: "ddg-apps",
        name: "DuckDuckGo App Tracker Blocklist",
        url: "https://raw.githubusercontent.com/duckduckgo/tracker-blocklists/main/app/android-tds.json",
        licence: "CC BY-NC-SA 4.0; fetched, never shipped",
        refresh_days: 7,
        uses: &[Use::Apps],
        shipped: false,
    },
    List {
        key: "doh-services",
        name: "HaGeZi – encrypted DNS services",
        url: "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/doh-onlydomains.txt",
        licence: "GPL-3.0",
        refresh_days: 7,
        uses: &[Use::DnsServices],
        shipped: false,
    },
    List {
        key: "ubo-resources",
        name: "uBlock Origin's resources (stand-ins and scriptlets)",
        url: "https://github.com/gorhill/uBlock",
        licence: "GPL-3.0",
        refresh_days: 0,
        uses: &[Use::Ads, Use::Trackers],
        shipped: true,
    },
];

/// The lists a use needs, fetched ones only (what the device downloads).
pub fn fetched_for(u: Use) -> impl Iterator<Item = &'static List> {
    LISTS.iter().filter(move |l| !l.shipped && l.uses.contains(&u))
}

pub fn by_key(key: &str) -> Option<&'static List> {
    LISTS.iter().find(|l| l.key == key)
}

/// **Lists that could not be fetched, tried again** (2026-11-19: a phone's first start on a poor
/// network built its engine without EasyPrivacy, and nothing tried again until the app was started
/// anew). While any of `lists` is not `kept`, after each of `waits` (passed to `sleep`) and holding
/// `lock` for the turn, each missing list is fetched again, and `rebuild` runs when one has arrived.
/// Stops once all are kept, the waits run out, or `still` says the switches have changed since.
#[allow(clippy::too_many_arguments)]
pub fn retry_missing<L: Copy, G>(
    lists: &[L],
    kept: impl Fn(L) -> bool,
    waits: &[u64],
    sleep: impl Fn(u64),
    lock: impl Fn() -> G,
    still: impl Fn() -> bool,
    fetch: impl Fn(L),
    rebuild: impl Fn(),
) {
    let missing = || lists.iter().copied().filter(|l| !kept(*l)).count();
    for wait in waits {
        if missing() == 0 {
            return;
        }
        sleep(*wait);
        let _turn = lock();
        if !still() {
            return;
        }
        let before = missing();
        for l in lists.iter().copied().filter(|l| !kept(*l)) {
            fetch(l);
        }
        if missing() < before {
            rebuild();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_is_named_once_fetched_securely_and_licensed() {
        let mut keys: Vec<_> = LISTS.iter().map(|l| l.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), LISTS.len(), "keys are unique");
        for l in LISTS {
            assert!(l.url.starts_with("https://"), "{}", l.key);
            assert!(!l.licence.is_empty() && !l.uses.is_empty(), "{}", l.key);
            if l.licence.contains("non-commercial") || l.licence.contains("NC") {
                assert!(!l.shipped, "{}: a non-commercial list is never in the GPL APK", l.key);
            }
        }
    }

    /// `THIRD-PARTY.md` names every list (the APK's notice and the owner's record).
    #[test]
    fn third_party_notice_names_every_list() {
        let notice = include_str!("../../../THIRD-PARTY.md");
        for l in LISTS {
            assert!(notice.contains(l.name), "THIRD-PARTY.md does not name {}", l.name);
        }
    }

    #[test]
    fn each_use_has_its_lists() {
        for u in [Use::Ads, Use::Trackers, Use::Links, Use::Apps, Use::DnsServices] {
            assert!(fetched_for(u).next().is_some(), "{u:?}");
        }
        assert_eq!(by_key("easylist").map(|l| l.name), Some("EasyList"));
    }

    #[test]
    fn a_list_that_failed_is_fetched_again_and_the_engine_rebuilt_when_it_arrives() {
        use std::cell::{Cell, RefCell};
        let kept = RefCell::new(vec![false, true]); // the second arrived at the start
        let (tries, rebuilt, slept) = (Cell::new(0), Cell::new(0), RefCell::new(Vec::new()));
        super::retry_missing(
            &[0usize, 1],
            |l| kept.borrow()[l],
            &[30, 60, 120, 300, 600, 1200],
            |s| slept.borrow_mut().push(s),
            || (),
            || true,
            |l| {
                tries.set(tries.get() + 1);
                if tries.get() == 3 {
                    kept.borrow_mut()[l] = true; // the network fails twice, then answers
                }
            },
            || rebuilt.set(rebuilt.get() + 1),
        );
        assert_eq!(tries.get(), 3, "only the missing list, until it arrived");
        assert_eq!(rebuilt.get(), 1, "rebuilt once, when it arrived");
        assert_eq!(*slept.borrow(), vec![30, 60, 120], "waiting longer each time, then done");
    }

    #[test]
    fn the_tries_stop_when_the_switches_change_or_the_waits_run_out() {
        use std::cell::Cell;
        let tries = Cell::new(0);
        let count = |_| tries.set(tries.get() + 1);
        super::retry_missing(
            &[0usize, 1],
            |_| false,
            &[0; 6],
            |_| (),
            || (),
            || false,
            count,
            || {},
        );
        assert_eq!(tries.get(), 0, "switched since: nothing fetched");
        super::retry_missing(
            &[0usize, 1],
            |_| false,
            &[0; 3],
            |_| (),
            || (),
            || true,
            count,
            || {},
        );
        assert_eq!(tries.get(), 6, "two lists, three turns, never arriving");
        super::retry_missing(&[0usize, 1], |_| true, &[0; 3], |_| (), || (), || true, count, || {});
        assert_eq!(tries.get(), 6, "all kept: nothing to do");
    }
}
