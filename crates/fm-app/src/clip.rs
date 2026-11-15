//! A page kept from formicaria's own browser — `decisions.md` 2026-11-15, *formicaria gets its own
//! browser*.
//!
//! **Nothing here fetches anything.** The person's browser already has the page: the add-on sends
//! its address, the note it built from the page (the shared TypeScript in `ui/src/lib/web/clip.ts`,
//! so a recipe kept from a page has the shape every recipe has) and a copy of the page as it was.
//! This module checks what arrived and composes what is written; `dispatch`'s `clip_save` stores
//! the copy as an asset and writes the note, in one call.
//!
//! **Every save keeps its copy** (owner, 2026-11-15: *"nothing kept out of the vault"*). The copy is
//! an asset like any other: in the vault, off the planning views, and off git when it is larger than
//! the vault's `git_assets_max`, exactly as every attachment is.
use serde::Deserialize;
use std::collections::BTreeMap;

/// A copy of a page larger than this is not one page (a feed, an endless scroll, a mistake).
pub const SNAPSHOT_LIMIT: usize = 32 * 1024 * 1024;

/// A note's text larger than this is not something a person kept from one page.
pub const BODY_LIMIT: usize = 4 * 1024 * 1024;

/// What the add-on sends. Every field is the page's own or the note the add-on built from it.
#[derive(Debug, Default, Deserialize)]
pub struct ClipRequest {
    /// The vault to keep it in; empty means the default one.
    #[serde(default)]
    pub vault: String,
    /// The page's address, as the browser had it.
    pub url: String,
    #[serde(default)]
    pub title: String,
    /// The note's text, built by the add-on.
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
    /// A paper's citation, built from the page's `citation_*` tags: when present the note is made by
    /// `create_paper`, so a paper kept from its page is the same note as one pasted as BibTeX.
    #[serde(default)]
    pub bibtex: String,
    /// The page as it was, serialised by the browser.
    #[serde(default)]
    pub snapshot: String,
}

/// The address a kept page may carry: a web page, `http` or `https`. It is only *recorded* — the
/// app never opens it — but a note's `source_url` is a link a person will press, so it must be one.
pub fn address(url: &str) -> Result<String, String> {
    let url = url.trim();
    let ok = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() > "https://".len()
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if !ok {
        return Err("Only a web page (http or https) can be kept from the browser.".into());
    }
    Ok(url.to_string())
}

/// The page's host, for the credit line when the page names no site.
pub fn host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?.split(':').next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The copy's file name: the page's title, made safe for a file, ending in `.html`.
pub fn snapshot_name(title: &str, url: &str) -> String {
    let base: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || " -_.,()".contains(c) { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let base = if base.is_empty() { host(url).unwrap_or_else(|| "page".into()) } else { base };
    let cut: String = base.chars().take(80).collect();
    format!("{}.html", cut.trim_end_matches(['.', ' ']))
}

/// The note's text with the copy linked at its end. `hash` is the asset's sha256 in hex.
pub fn with_snapshot(body: &str, hash: Option<&str>) -> String {
    let mut out = body.trim_end().to_string();
    if let Some(h) = hash {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("[A copy of the page as it was saved](asset:sha256-{h})"));
    }
    out.push('\n');
    out
}

/// A property's value as the text `apply_property` types, the way a hand-edited file would hold it.
pub fn written(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Check a request before anything is written. `Err` is a sentence for the person.
pub fn check(req: &ClipRequest) -> Result<(), String> {
    address(&req.url)?;
    if req.body.len() > BODY_LIMIT {
        return Err("This page's text is too large to keep as one note.".into());
    }
    if req.snapshot.len() > SNAPSHOT_LIMIT {
        return Err("This page is too large to keep a copy of (over 32 MB).".into());
    }
    if req.title.trim().is_empty() && req.bibtex.trim().is_empty() {
        return Err("The page has no title, so there is nothing to name the note by.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_pages_are_kept() {
        assert!(address("https://example.org/a").is_ok());
        assert!(address("http://example.org").is_ok());
        assert!(address("file:///etc/passwd").is_err());
        assert!(address("javascript:alert(1)").is_err());
        assert!(address("about:blank").is_err());
        assert!(address("https://").is_err());
        assert!(address("https://exa mple.org").is_err());
    }

    #[test]
    fn host_drops_www_port_and_credentials() {
        assert_eq!(host("https://www.Example.org:8443/x?y").as_deref(), Some("example.org"));
        assert_eq!(host("https://user@site.net/").as_deref(), Some("site.net"));
        assert_eq!(host("nonsense"), None);
    }

    #[test]
    fn snapshot_name_is_safe_and_bounded() {
        assert_eq!(snapshot_name("Pasta / alla Norma?", "https://a.org"), "Pasta alla Norma.html");
        assert_eq!(snapshot_name("", "https://www.a.org/x"), "a.org.html");
        let long = "x".repeat(300);
        assert_eq!(snapshot_name(&long, "https://a.org").len(), 80 + ".html".len());
        assert!(!snapshot_name("../../etc", "https://a.org").contains('/'));
    }

    #[test]
    fn the_copy_is_linked_at_the_end() {
        assert_eq!(
            with_snapshot("Text\n\n", Some("ab12")),
            "Text\n\n[A copy of the page as it was saved](asset:sha256-ab12)\n"
        );
        assert_eq!(with_snapshot("Text", None), "Text\n");
    }

    #[test]
    fn a_request_is_checked_before_anything_is_written() {
        let ok =
            ClipRequest { url: "https://a.org".into(), title: "T".into(), ..Default::default() };
        assert!(check(&ok).is_ok());
        let untitled = ClipRequest { url: "https://a.org".into(), ..Default::default() };
        assert!(check(&untitled).is_err());
        let paper = ClipRequest {
            url: "https://a.org".into(),
            bibtex: "@article{x, title={T}}".into(),
            ..Default::default()
        };
        assert!(check(&paper).is_ok());
        let huge = ClipRequest {
            url: "https://a.org".into(),
            title: "T".into(),
            snapshot: "x".repeat(SNAPSHOT_LIMIT + 1),
            ..Default::default()
        };
        assert!(check(&huge).is_err());
    }
}
