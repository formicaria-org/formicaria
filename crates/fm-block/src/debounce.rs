//! **Skipping a tracker's redirect** (`privacy-plan-2026-11-19.md` 3.5, "debouncing"): a link
//! through `click.tracker.example/?url=https://real.example/` opens `https://real.example/`
//! directly, so the tracker never sees the visit. Rules in Brave's `debounce.json` form (MPL-2.0),
//! with formicaria's own few added ([`OWN`]):
//!
//! `{"include": [match patterns], "exclude": [...], "action": "redirect" | "base64,redirect" |
//! "regex-path" | "regex-path-template", "param": …, "prepend_scheme": "https",
//! "redirect_url_template": "…$1…"}`. A rule with a `pref` (Brave's own settings) is left out.

use base64::Engine as _;
use regex::Regex;
use serde::Deserialize;

/// formicaria's own rules: the wrappers people meet most, which Brave unwraps by other means.
pub const OWN: &str = r#"[
  {"include": ["*://www.google.com/url?*", "*://google.com/url?*"], "exclude": [], "action": "redirect", "param": "q"},
  {"include": ["*://www.google.com/url?*", "*://google.com/url?*"], "exclude": [], "action": "redirect", "param": "url"},
  {"include": ["*://l.facebook.com/l.php?*", "*://lm.facebook.com/l.php?*", "*://l.messenger.com/l.php?*"], "exclude": [], "action": "redirect", "param": "u"},
  {"include": ["*://l.instagram.com/?*"], "exclude": [], "action": "redirect", "param": "u"},
  {"include": ["*://www.youtube.com/redirect?*"], "exclude": [], "action": "redirect", "param": "q"},
  {"include": ["*://out.reddit.com/*"], "exclude": [], "action": "redirect", "param": "url"},
  {"include": ["*://www.linkedin.com/redir/redirect?*"], "exclude": [], "action": "redirect", "param": "url"},
  {"include": ["*://slack-redir.net/link?*"], "exclude": [], "action": "redirect", "param": "url"}
]"#;

#[derive(Deserialize)]
struct RawRule {
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    action: String,
    param: String,
    #[serde(default)]
    prepend_scheme: Option<String>,
    #[serde(default)]
    redirect_url_template: Option<String>,
    #[serde(default)]
    pref: Option<String>,
}

enum Action {
    Param,
    Base64Param,
    Path(Regex),
    PathTemplate(Regex, String),
}

struct Rule {
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
    action: Action,
    param: String,
    scheme: Option<String>,
}

#[derive(Default)]
pub struct Debouncer {
    rules: Vec<Rule>,
}

impl Debouncer {
    /// formicaria's own rules, then each text's (Brave's form); a text that cannot be read adds none.
    pub fn new(texts: &[&str]) -> Debouncer {
        let mut d = Debouncer::default();
        for t in std::iter::once(OWN).chain(texts.iter().copied()) {
            let Ok(raw) = serde_json::from_str::<Vec<RawRule>>(t) else { continue };
            for r in raw.into_iter().filter(|r| r.pref.is_none()) {
                let action = match r.action.as_str() {
                    "redirect" => Action::Param,
                    "base64,redirect" => Action::Base64Param,
                    "regex-path" => match Regex::new(&r.param) {
                        Ok(re) => Action::Path(re),
                        Err(_) => continue,
                    },
                    "regex-path-template" => {
                        match (Regex::new(&r.param), r.redirect_url_template) {
                            (Ok(re), Some(t)) => Action::PathTemplate(re, t),
                            _ => continue,
                        }
                    }
                    _ => continue,
                };
                d.rules.push(Rule {
                    include: r.include.iter().filter_map(|p| Pattern::parse(p)).collect(),
                    exclude: r.exclude.iter().filter_map(|p| Pattern::parse(p)).collect(),
                    action,
                    param: r.param,
                    scheme: r.prepend_scheme,
                });
            }
        }
        d
    }

    /// Where this link really goes, if it goes through a known redirect; `None` otherwise. Followed
    /// at most a few times (a wrapper inside a wrapper); only `http`/`https` results.
    pub fn unwrap(&self, url: &str) -> Option<String> {
        let mut now = url.to_string();
        let mut changed = false;
        for _ in 0..4 {
            match self.once(&now) {
                Some(next) if next != now => {
                    now = next;
                    changed = true;
                }
                _ => break,
            }
        }
        changed.then_some(now)
    }

    fn once(&self, url: &str) -> Option<String> {
        let parts = Parts::of(url)?;
        for r in &self.rules {
            if !r.include.iter().any(|p| p.matches(&parts))
                || r.exclude.iter().any(|p| p.matches(&parts))
            {
                continue;
            }
            let found = match &r.action {
                Action::Param => query_param(parts.query, &r.param),
                Action::Base64Param => query_param(parts.query, &r.param).and_then(|v| {
                    let v = v.trim_end_matches('=');
                    base64::engine::general_purpose::STANDARD_NO_PAD
                        .decode(v)
                        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(v))
                        .ok()
                        .and_then(|b| String::from_utf8(b).ok())
                }),
                Action::Path(re) => re.captures(parts.path).map(|c| {
                    let joined: String = c.iter().skip(1).flatten().map(|m| m.as_str()).collect();
                    decode(&joined)
                }),
                Action::PathTemplate(re, t) => re.captures(parts.path).map(|c| {
                    let mut out = t.clone();
                    for i in (1..c.len()).rev() {
                        out = out.replace(&format!("${i}"), c.get(i).map_or("", |m| m.as_str()));
                    }
                    out
                }),
            };
            let Some(mut next) = found else { continue };
            if let Some(s) = &r.scheme {
                if !next.contains("://") {
                    next = format!("{s}://{next}");
                }
            }
            if next.starts_with("https://") || next.starts_with("http://") {
                return Some(next);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

struct Parts<'a> {
    scheme: &'a str,
    host: String,
    /// The path and the query, as match patterns see them.
    path_query: &'a str,
    path: &'a str,
    query: &'a str,
}

impl<'a> Parts<'a> {
    fn of(url: &'a str) -> Option<Parts<'a>> {
        let (scheme, rest) = url.split_once("://")?;
        let cut = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..cut];
        let host = authority.rsplit('@').next()?.split(':').next()?.to_ascii_lowercase();
        let mut path_query = &rest[cut..];
        if let Some(h) = path_query.find('#') {
            path_query = &path_query[..h];
        }
        let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
        Some(Parts { scheme, host, path_query, path, query })
    }
}

/// A match pattern: `<scheme>://<host><path>`, `*` the only wildcard; `*.example.com` is the name
/// and every name under it.
struct Pattern {
    scheme: String,
    host: String,
    path: Regex,
}

impl Pattern {
    fn parse(p: &str) -> Option<Pattern> {
        let (scheme, rest) = p.split_once("://")?;
        let cut = rest.find('/').unwrap_or(rest.len());
        let host = rest[..cut].to_ascii_lowercase();
        let path = if cut < rest.len() { &rest[cut..] } else { "/*" };
        let re = format!("^{}$", path.split('*').map(regex::escape).collect::<Vec<_>>().join(".*"));
        Some(Pattern { scheme: scheme.to_string(), host, path: Regex::new(&re).ok()? })
    }

    fn matches(&self, u: &Parts) -> bool {
        let scheme_ok = match self.scheme.as_str() {
            "*" => u.scheme == "http" || u.scheme == "https",
            s => s == u.scheme,
        };
        let host_ok = if self.host == "*" {
            true
        } else if let Some(base) = self.host.strip_prefix("*.") {
            u.host == base || u.host.ends_with(&format!(".{base}"))
        } else {
            u.host == self.host
        };
        let pq = if u.path_query.is_empty() { "/" } else { u.path_query };
        scheme_ok && host_ok && self.path.is_match(pq)
    }
}

fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (decode(k) == name).then(|| decode(v))
    })
}

fn decode(s: &str) -> String {
    percent_encoding::percent_decode_str(&s.replace('+', " ")).decode_utf8_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRAVE_LIKE: &str = r#"[
      {"include": ["*://*.click.example/r?*"], "exclude": ["*://safe.click.example/*"], "action": "redirect", "param": "to"},
      {"include": ["*://b64.example/*"], "exclude": [], "action": "base64,redirect", "param": "u"},
      {"include": ["*://mail.example/CL0/*"], "exclude": [], "action": "regex-path", "param": "^/CL0/([^/]+)/.*$"},
      {"include": ["*://amp.example/c/s/*"], "exclude": [], "prepend_scheme": "https", "action": "regex-path", "param": "^/c/s/(.*)$"},
      {"include": ["*://short.example/*"], "exclude": [], "action": "regex-path-template", "param": "^/([^/]+)$", "redirect_url_template": "https://video.example/watch?v=$1"},
      {"include": ["*://pref.example/*"], "pref": "brave.only", "exclude": [], "action": "redirect", "param": "u"}
    ]"#;

    #[test]
    fn a_wrapped_link_opens_where_it_goes() {
        let d = Debouncer::new(&[BRAVE_LIKE]);
        assert_eq!(
            d.unwrap("https://go.click.example/r?id=1&to=https%3A%2F%2Freal.example%2Fa%3Fb%3D1")
                .as_deref(),
            Some("https://real.example/a?b=1")
        );
        assert_eq!(
            d.unwrap("https://safe.click.example/r?to=https://real.example/"),
            None,
            "excluded"
        );
        assert_eq!(
            d.unwrap("https://b64.example/x?u=aHR0cHM6Ly9yZWFsLmV4YW1wbGUv").as_deref(),
            Some("https://real.example/")
        );
        assert_eq!(
            d.unwrap("https://mail.example/CL0/https:%2F%2Freal.example%2Fpage/1/abc").as_deref(),
            Some("https://real.example/page")
        );
        assert_eq!(
            d.unwrap("https://amp.example/c/s/real.example/story").as_deref(),
            Some("https://real.example/story")
        );
        assert_eq!(
            d.unwrap("https://short.example/abc123").as_deref(),
            Some("https://video.example/watch?v=abc123")
        );
        assert_eq!(
            d.unwrap("https://pref.example/?u=https://real.example/"),
            None,
            "Brave's own settings left out"
        );
        assert_eq!(
            d.unwrap("https://real.example/?to=https://elsewhere.example/"),
            None,
            "an ordinary link"
        );
    }

    #[test]
    fn formicaria_s_own_rules_unwrap_the_common_wrappers() {
        let d = Debouncer::new(&[]);
        assert_eq!(
            d.unwrap("https://www.google.com/url?sa=t&q=https://real.example/&usg=x").as_deref(),
            Some("https://real.example/")
        );
        assert_eq!(
            d.unwrap("https://l.facebook.com/l.php?u=https%3A%2F%2Freal.example%2F&h=AT0")
                .as_deref(),
            Some("https://real.example/")
        );
        assert_eq!(d.unwrap("https://www.google.com/search?q=ants"), None);
    }

    #[test]
    fn only_web_addresses_come_out_and_a_bad_rule_set_adds_nothing() {
        let d = Debouncer::new(&["not json"]);
        assert_eq!(d.unwrap("https://www.google.com/url?q=javascript:alert(1)"), None);
        assert_eq!(d.len(), Debouncer::new(&[]).len());
    }
}
