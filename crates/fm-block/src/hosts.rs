//! **The phone-wide block's names** (`privacy-plan-2026-11-19.md` 2.4): the trackers apps reach,
//! as a set of names matched by suffix (`pixel.tracker.example` is refused when `tracker.example` is
//! listed). Read from the lists as published, whatever their form:
//!
//! - Adblock syntax (EasyPrivacy, Peter Lowe's): only whole-site rules, `||name^` (with no options or
//!   only `$third-party`/`$all`); `@@||name^` lets a name through. Rules with a path or a type say
//!   nothing about an app's lookup and are left out.
//! - Hosts files (`0.0.0.0 name`) and plain lists of names, one per line (`doh-onlydomains.txt`).
//! - DuckDuckGo's app tracker list (JSON, `trackers`): the names whose `default` is `block`.

use std::collections::HashSet;

#[derive(Default)]
pub struct HostSet {
    refused: HashSet<String>,
    allowed: HashSet<String>,
}

impl HostSet {
    pub fn new() -> HostSet {
        HostSet::default()
    }

    /// Add a list's text, in any of the forms above. Returns how many names it refused.
    pub fn add(&mut self, text: &str) -> usize {
        let t = text.trim_start();
        if t.starts_with('{') {
            return self.add_json(t);
        }
        let before = self.refused.len();
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('!') || l.starts_with('#') || l.starts_with('[') {
                continue;
            }
            if let Some(rest) = l.strip_prefix("@@||") {
                if let Some(n) = whole_site(rest) {
                    self.allowed.insert(n);
                }
                continue;
            }
            if let Some(rest) = l.strip_prefix("||") {
                if let Some(n) = whole_site(rest) {
                    self.refused.insert(n);
                }
                continue;
            }
            // Hosts-file lines: an address, then the name(s).
            let mut parts = l.split_whitespace();
            let first = parts.next().unwrap_or("");
            let name = if matches!(first, "0.0.0.0" | "127.0.0.1" | "::" | "::1") {
                parts.next().unwrap_or("")
            } else {
                first
            };
            if let Some(n) = plain_name(name) {
                if !matches!(n.as_str(), "localhost" | "local" | "broadcasthost") {
                    self.refused.insert(n);
                }
            }
        }
        self.refused.len() - before
    }

    fn add_json(&mut self, t: &str) -> usize {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(t) else { return 0 };
        let Some(tr) = v.get("trackers").and_then(|x| x.as_object()) else { return 0 };
        let before = self.refused.len();
        for (name, entry) in tr {
            if entry.get("default").and_then(|d| d.as_str()) == Some("block") {
                if let Some(n) = plain_name(name) {
                    self.refused.insert(n);
                }
            }
        }
        self.refused.len() - before
    }

    /// Is this name, or a name it is under, refused (and not let through)?
    pub fn contains(&self, name: &str) -> bool {
        let n = name.trim_end_matches('.').to_ascii_lowercase();
        let mut hit = false;
        for suffix in suffixes(&n) {
            if self.allowed.contains(suffix) {
                return false;
            }
            if self.refused.contains(suffix) {
                hit = true;
            }
        }
        hit
    }

    pub fn len(&self) -> usize {
        self.refused.len()
    }

    pub fn is_empty(&self) -> bool {
        self.refused.is_empty()
    }
}

/// `name^`, `name^$third-party`, `name^$all` → the name; anything with a path, a wildcard or other
/// options → nothing.
fn whole_site(rest: &str) -> Option<String> {
    let (body, opts) = rest.split_once('$').unwrap_or((rest, ""));
    if !opts.is_empty()
        && !opts.split(',').all(|o| matches!(o, "third-party" | "3p" | "all" | "important"))
    {
        return None;
    }
    plain_name(body.strip_suffix('^').or_else(|| body.strip_suffix("^|"))?)
}

fn plain_name(s: &str) -> Option<String> {
    let s = s.trim().trim_end_matches('.').to_ascii_lowercase();
    let ok = s.contains('.')
        && s.len() <= 253
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
        && !s.starts_with('.')
        && !s.chars().all(|c| c.is_ascii_digit() || c == '.');
    ok.then_some(s)
}

/// `a.b.c` → `a.b.c`, `b.c`, `c`.
fn suffixes(n: &str) -> impl Iterator<Item = &str> {
    std::iter::once(n).chain(n.match_indices('.').map(move |(i, _)| &n[i + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_site_rules_hosts_lines_and_plain_names_are_read() {
        let mut h = HostSet::new();
        let n = h.add(
            "! An invented list\n||tracker.example^\n||ads.example^$third-party\n||cdn.example/ads/*.js\n\
             ||only-images.example^$image\n@@||ok.tracker.example^\n0.0.0.0 metrics.example\n\
             127.0.0.1 localhost\nplain.example\n# a comment\n10.0.0.1\n",
        );
        assert_eq!(n, 4, "tracker, ads, metrics, plain");
        assert!(h.contains("tracker.example"));
        assert!(h.contains("pixel.tracker.example"), "a name under a listed one");
        assert!(h.contains("PIXEL.Tracker.Example."), "case and the root dot");
        assert!(!h.contains("ok.tracker.example"), "let through");
        assert!(!h.contains("deep.ok.tracker.example"), "and under it");
        assert!(
            h.contains("ads.example")
                && h.contains("metrics.example")
                && h.contains("plain.example")
        );
        assert!(!h.contains("cdn.example"), "a rule with a path says nothing about a lookup");
        assert!(!h.contains("only-images.example"), "nor one for one type");
        assert!(!h.contains("localhost"));
        assert!(!h.contains("example"), "a parent of a listed name is not refused");
        assert!(!h.contains("nottracker.example"), "a suffix is a whole label");
    }

    #[test]
    fn an_app_list_in_json_gives_its_blocked_names() {
        let mut h = HostSet::new();
        let n = h.add(
            r#"{"trackers":{"sdk.tracker.example":{"default":"block"},"cdn.app.example":{"default":"ignore"}}}"#,
        );
        assert_eq!(n, 1);
        assert!(h.contains("eu.sdk.tracker.example"));
        assert!(!h.contains("cdn.app.example"));
        assert_eq!(h.add("{not json"), 0);
    }
}
