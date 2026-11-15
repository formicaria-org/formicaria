//! **One contract for every source** (`decisions.md` 2026-11-15, late night; the research:
//! `docs/context/data-sources-research-2026-11-15.md` §5).
//!
//! formicaria brings things in from outside: web pages, mail, calendars, files, and next a watch. Each
//! source has its own reader, but what it brings is described the same way, so the vault's search,
//! the views, the "this page is in your notes" lookup and the assistant can read any of it alike:
//!
//! - [`SOURCE`]: which source (`web`, `mail`, `calendar`, `files`, `watch`, …);
//! - [`SOURCE_ID`]: the source's own name for the item (a page's address, a DOI, a thread id), so a
//!   second arrival can be recognised;
//! - [`CAPTURED`]: when it came in;
//! - [`RECORDED`]: how it was made: `sensed` (a device measured it), `typed` (a person wrote it),
//!   `copied` (taken as it was from a document) or `derived` (computed from other items).
//!
//! The original is kept in the vault where there is one (a page's copy, a FIT file). Mail and
//! calendars keep their own identity keys from before the contract (`gmail_thread`, `ics_uid`): the
//! [`catalog`] names them rather than renaming notes people already have.

use fm_core::{Store, StoreError};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Which source an item came from.
pub const SOURCE: &str = "source";
/// The source's own identity for the item.
pub const SOURCE_ID: &str = "source_id";
/// When it came in (RFC 3339, UTC).
pub const CAPTURED: &str = "captured";
/// How it was made: `sensed`, `typed`, `copied` or `derived`.
pub const RECORDED: &str = "recorded";

/// Add the contract's properties to `props` where the caller did not set them.
pub fn stamp(props: &mut BTreeMap<String, Value>, source: &str, id: &str, recorded: &str) {
    let now = OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default();
    for (k, v) in
        [(SOURCE, source), (SOURCE_ID, id), (CAPTURED, now.as_str()), (RECORDED, recorded)]
    {
        if !v.is_empty() {
            props.entry(k.to_string()).or_insert_with(|| Value::String(v.to_string()));
        }
    }
}

/// A page's address reduced to what names the page: no scheme, no `www.`/`m.` host prefix, no
/// fragment, no tracking parameters, no trailing slash, host in lower case. Two addresses of the
/// same page give the same key; `None` when it is not a web address.
pub fn page_key(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let rest = rest.split('#').next().unwrap_or("");
    let (hostpath, query) = match rest.split_once('?') {
        Some((a, b)) => (a, Some(b)),
        None => (rest, None),
    };
    let (host, path) = match hostpath.split_once('/') {
        Some((h, p)) => (h, p),
        None => (hostpath, ""),
    };
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host);
    if host.is_empty() || !host.contains('.') && !host.starts_with("localhost") {
        return None;
    }
    let path = path.trim_end_matches('/');
    let mut kept: Vec<&str> = query
        .unwrap_or("")
        .split('&')
        .filter(|p| !p.is_empty())
        .filter(|p| {
            let k = p.split('=').next().unwrap_or("").to_ascii_lowercase();
            !(k.starts_with("utm_")
                || matches!(
                    k.as_str(),
                    "fbclid" | "gclid" | "mc_cid" | "mc_eid" | "ref" | "ref_src"
                ))
        })
        .collect();
    kept.sort_unstable();
    let mut key = host.to_string();
    if !path.is_empty() {
        key.push('/');
        key.push_str(path);
    }
    if !kept.is_empty() {
        key.push('?');
        key.push_str(&kept.join("&"));
    }
    Some(key)
}

/// Every web address written in `text`, in order (Markdown links, angle brackets and bare ones).
pub fn addresses(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = text[i..].find("http") {
        let start = i + at;
        let tail = &text[start..];
        if tail.starts_with("http://") || tail.starts_with("https://") {
            let end = tail
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, ')' | '>' | '<' | '"' | '\'' | ']' | '`')
                })
                .unwrap_or(tail.len());
            let found = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
            out.push(found);
            i = start + end.max(1);
        } else {
            i = start + 4;
        }
    }
    out
}

/// A note that names a page, with the line it is named on.
#[derive(Debug, Serialize, PartialEq)]
pub struct PageNote {
    pub id: String,
    pub title: String,
    /// The line of the note the page is named on (or the property that names it), shortened.
    pub context: String,
}

/// **"This page is in your notes"**: the notes that name `url`, by its [`page_key`], in their text or
/// in a property (`source_url`, `source_id`, `url`). At most `limit`, most recently edited first.
pub fn page_notes(store: &dyn Store, url: &str, limit: usize) -> Result<Vec<PageNote>, StoreError> {
    let Some(key) = page_key(url) else { return Ok(vec![]) };
    let q = fm_query::Query {
        filter: crate::thread::notes_base(),
        sort: vec![fm_query::SortKey::desc("updated")],
        ..Default::default()
    };
    let mut out = Vec::new();
    for o in store.query(&q)?.rows.iter() {
        let mut context = None;
        for k in ["source_url", SOURCE_ID, "url"] {
            if let Some(v) = o.extra.get(k) {
                if page_key(&v.display()).as_deref() == Some(key.as_str()) {
                    context = Some(format!("{k}: {}", v.display()));
                    break;
                }
            }
        }
        if context.is_none() {
            for line in o.body.lines() {
                if addresses(line).iter().any(|a| page_key(a).as_deref() == Some(key.as_str())) {
                    context = Some(shorten(line.trim(), 200));
                    break;
                }
            }
        }
        if let Some(context) = context {
            out.push(PageNote {
                id: o.id.to_string(),
                title: o.title.clone().unwrap_or_default(),
                context,
            });
            if out.len() >= limit {
                break;
            }
        }
    }
    Ok(out)
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{cut}…")
}

/// One source, as the catalog describes it.
#[derive(Debug, Serialize)]
pub struct SourceEntry {
    pub source: &'static str,
    pub name: &'static str,
    /// What it brings in.
    pub kinds: &'static [&'static str],
    /// The property that identifies its items.
    pub identity: &'static str,
    /// How its items are made (`RECORDED`).
    pub recorded: &'static str,
    /// Whether the original is kept in the vault.
    pub raw_kept: bool,
    /// `shipped` or `planned`.
    pub status: &'static str,
}

/// **The catalog**: every source formicaria knows, what it brings and how its items are named. What
/// Settings shows and what an assistant reads first (research §5, element 5).
pub fn catalog() -> Vec<SourceEntry> {
    vec![
        SourceEntry {
            source: "web",
            name: "Web pages (formicaria's browser)",
            kinds: &["article", "recipe", "paper", "video", "song", "bookmark"],
            identity: SOURCE_ID,
            recorded: "copied",
            raw_kept: true,
            status: "shipped",
        },
        SourceEntry {
            source: "mail",
            name: "Gmail, the labelled conversations",
            kinds: &["conversation"],
            identity: fm_core::mail::GMAIL_THREAD,
            recorded: "copied",
            raw_kept: false,
            status: "shipped",
        },
        SourceEntry {
            source: "calendar",
            name: "Calendars",
            kinds: &["meeting"],
            identity: fm_core::calendar::ICS_UID,
            recorded: "copied",
            raw_kept: false,
            status: "shipped",
        },
        SourceEntry {
            source: "files",
            name: "Files added to a note",
            kinds: &["asset"],
            identity: "sha256",
            recorded: "copied",
            raw_kept: true,
            status: "shipped",
        },
        SourceEntry {
            source: "watch",
            name: "A smartwatch, through Gadgetbridge (FIT files)",
            kinds: &["workout", "sleep", "heart rate"],
            identity: SOURCE_ID,
            recorded: "sensed",
            raw_kept: true,
            status: "planned",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_addresses_of_one_page_share_a_key() {
        let k = page_key("https://en.wikipedia.org/wiki/Ant").unwrap();
        for same in [
            "http://en.wikipedia.org/wiki/Ant/",
            "https://en.wikipedia.org/wiki/Ant#Taxonomy",
            "https://en.wikipedia.org/wiki/Ant?utm_source=x&fbclid=y",
            "https://EN.wikipedia.org/wiki/Ant",
        ] {
            assert_eq!(page_key(same).as_deref(), Some(k.as_str()), "{same}");
        }
        assert_eq!(page_key("https://www.example.org/a").unwrap(), "example.org/a");
        assert_eq!(page_key("https://m.example.org/a").unwrap(), "example.org/a");
        assert_ne!(page_key("https://en.wikipedia.org/wiki/Ants"), Some(k.clone()));
        assert_eq!(page_key("https://a.org/p?b=2&a=1"), page_key("https://a.org/p?a=1&b=2"));
        assert_eq!(page_key("mailto:x@y.org"), None);
        assert_eq!(page_key("https://"), None);
    }

    #[test]
    fn addresses_are_found_in_markdown_and_prose() {
        let t = "See [the ant](https://a.org/ant). Also <https://b.org/x>, and https://c.org/y.";
        assert_eq!(addresses(t), vec!["https://a.org/ant", "https://b.org/x", "https://c.org/y"]);
        assert!(addresses("no links, just http talk").is_empty());
    }

    #[test]
    fn a_stamp_never_overrides_what_the_caller_set() {
        let mut p = BTreeMap::new();
        p.insert(RECORDED.to_string(), Value::String("typed".into()));
        stamp(&mut p, "web", "https://a.org/", "copied");
        assert_eq!(p[SOURCE], "web");
        assert_eq!(p[SOURCE_ID], "https://a.org/");
        assert_eq!(p[RECORDED], "typed");
        assert!(p[CAPTURED].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn the_catalog_names_each_source_once() {
        let c = catalog();
        let mut names: Vec<_> = c.iter().map(|s| s.source).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), c.len());
    }
}
