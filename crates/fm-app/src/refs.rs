//! Cross-vault reference handling for a note *body*: find the references it makes
//! (`note:<ulid>`, `asset:sha256-<hex>`, `sha256:<hex>`), and strip them out.
//!
//! Kept as one small pure module — no `Store`, no fs, no deps — so `copy_note`
//! composes it now and the deferred linked-notes tier can reuse `references`
//! later. The stripping is what enforces the **no-leak / vault-separation**
//! default: a copied note carries only its prose unless the user opts in, so it
//! can never point at a note or blob living in another vault.
//!
//! Deliberately no `regex` dependency — these are a handful of fixed-shape tokens
//! and Markdown link/image spans, cheaper to scan by hand than to pull the crate.

use fm_model::PropertyValue;

/// What replaces a stripped reference. Carries **none** of the original label,
/// name or path — that is the point (a filename could itself name private work).
pub const REDACTION: &str = "⟨removed on copy⟩";

/// The first-degree references a body makes: `(asset_hashes, note_ids)`, each
/// lowercased/normalised and de-duplicated. Assets come from `asset:sha256-<hex>`
/// and bare `sha256:<hex>`; notes from `note:<ulid>`.
pub fn references(body: &str) -> (Vec<String>, Vec<String>) {
    let mut assets = Vec::new();
    scan(body, "asset:sha256-", &mut |rest| take_hex(rest).map(|h| assets.push(h)));
    scan(body, "sha256:", &mut |rest| take_hex(rest).map(|h| assets.push(h)));
    let mut notes = Vec::new();
    scan(body, "note:", &mut |rest| take_ulid(rest).map(|u| notes.push(u)));
    dedup(&mut assets);
    dedup(&mut notes);
    (assets, notes)
}

/// Rewrite a body so no cross-vault reference survives. `note:` links are **always**
/// removed; asset references (our `asset:`/`sha256:` schemes, and any *local* — non
/// `http(s)`/`data` — Markdown image) are removed unless `keep_assets`. Each removed
/// Markdown link/image span (`[label](url)` / `![alt](url)`) is replaced whole — label
/// and all — by [`REDACTION`]; a bare scheme token is replaced in place.
pub fn strip_cross_vault(body: &str, keep_assets: bool) -> String {
    let b = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < b.len() {
        // A Markdown link `[..](..)` or image `![..](..)`?
        let is_img = b[i] == b'!' && i + 1 < b.len() && b[i + 1] == b'[';
        if b[i] == b'[' || is_img {
            let bracket = if is_img { i + 1 } else { i };
            if let Some(span) = markdown_span(b, bracket) {
                let url = body[span.url.clone()].trim();
                if strip_url(url, is_img, keep_assets) {
                    out.push_str(REDACTION);
                    i = span.end;
                    continue;
                }
            }
        }
        // A bare scheme token, not inside a Markdown link (rare, but still a leak)?
        if let Some(len) = bare_token(&body[i..], keep_assets) {
            out.push_str(REDACTION);
            i += len;
            continue;
        }
        // Ordinary text: copy one whole UTF-8 char.
        let ch = utf8_len(b[i]);
        out.push_str(&body[i..i + ch]);
        i += ch;
    }
    out
}

// ── internals ───────────────────────────────────────────────────────────────

/// The byte ranges of a Markdown `[label](url)` starting at the `[` at `open`.
struct Span {
    url: std::ops::Range<usize>,
    end: usize, // one past the closing ')'
}
fn markdown_span(b: &[u8], open: usize) -> Option<Span> {
    let close = find(b, open + 1, b']')?;
    if close + 1 >= b.len() || b[close + 1] != b'(' {
        return None;
    }
    let paren = find(b, close + 2, b')')?;
    Some(Span { url: (close + 2)..paren, end: paren + 1 })
}

/// Should the URL inside a Markdown link/image be stripped?
fn strip_url(url: &str, is_img: bool, keep_assets: bool) -> bool {
    let u = url.trim();
    if is_note(u) {
        return true; // note links always go
    }
    if is_asset(u) {
        return !keep_assets;
    }
    // A local image (relative path / bare filename) is an out-of-vault pointer too;
    // an external http(s)/data image is not, so it stays.
    if is_img && !u.starts_with("http://") && !u.starts_with("https://") && !u.starts_with("data:") {
        return !keep_assets;
    }
    false
}

/// A bare `note:`/`asset:`/`sha256:` token at the start of `s`; returns its byte length.
fn bare_token(s: &str, keep_assets: bool) -> Option<usize> {
    if let Some(rest) = s.strip_prefix("note:") {
        return take_ulid(rest).map(|u| "note:".len() + u.len());
    }
    if keep_assets {
        return None;
    }
    if let Some(rest) = s.strip_prefix("asset:sha256-") {
        return take_hex(rest).map(|h| "asset:sha256-".len() + h.len() + fragment_len(&rest[h.len()..]));
    }
    if let Some(rest) = s.strip_prefix("sha256:") {
        return take_hex(rest).map(|h| "sha256:".len() + h.len() + fragment_len(&rest[h.len()..]));
    }
    None
}

/// How much of a trailing `#…` fragment belongs to the token just consumed.
///
/// An anchored reference (`asset:sha256-<hex>#page=4`) carries the page a highlight sits on. Left
/// behind by a redaction it becomes visible debris — `⟨removed on copy⟩#page=4` — against
/// [`REDACTION`]'s stated contract of carrying *none* of the original. It is only a page number, so
/// this is tidiness rather than a leak of substance, but the contract is the contract.
///
/// Ends at whitespace or at any Markdown/prose delimiter, so a fragment cannot swallow the rest of
/// a sentence when someone writes `see sha256:abc#page=2, and then…`.
fn fragment_len(after_hash: &str) -> usize {
    if !after_hash.starts_with('#') {
        return 0;
    }
    let end = after_hash[1..]
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | ',' | ';' | '"' | '\''))
        .map(|i| i + 1)
        .unwrap_or(after_hash.len());
    end
}

fn is_note(u: &str) -> bool {
    u.strip_prefix("note:").is_some_and(|r| take_ulid(r).is_some())
}
fn is_asset(u: &str) -> bool {
    u.starts_with("asset:") || u.starts_with("sha256:")
}

/// The leading run of hex as a lowercase hash — `None` unless it is a full 64-char
/// sha256 (so a short accidental hex run is not mistaken for a blob).
fn take_hex(s: &str) -> Option<String> {
    let hex: String = s.chars().take_while(|c| c.is_ascii_hexdigit()).take(64).collect();
    (hex.len() == 64).then(|| hex.to_ascii_lowercase())
}

/// The leading 26 Crockford-base32 chars of a ULID (letters/digits), or `None`.
fn take_ulid(s: &str) -> Option<String> {
    let id: String = s.chars().take_while(|c| c.is_ascii_alphanumeric()).take(26).collect();
    (id.len() == 26).then_some(id)
}

/// Find `byte` in `b` at or after `from`.
fn find(b: &[u8], from: usize, byte: u8) -> Option<usize> {
    (from..b.len()).find(|&j| b[j] == byte)
}

/// Scan every occurrence of `token`, handing the text *after* it to `f`.
fn scan(body: &str, token: &str, f: &mut dyn FnMut(&str) -> Option<()>) {
    let mut from = 0;
    while let Some(rel) = body[from..].find(token) {
        let at = from + rel + token.len();
        let _ = f(&body[at..]);
        from = at;
    }
}

fn dedup(v: &mut Vec<String>) {
    v.sort();
    v.dedup();
}

/// Byte length of the UTF-8 char whose leading byte is `first`.
fn utf8_len(first: u8) -> usize {
    match first {
        b if b < 0x80 => 1,
        b if b >> 5 == 0b110 => 2,
        b if b >> 4 == 0b1110 => 3,
        _ => 4,
    }
}

/// Strip cross-vault references out of a **property value**, recursing into lists.
///
/// Frontmatter is text the user typed just as much as the body is, so a custom
/// property can hold a `note:`/`asset:` pointer — and a copy that carried one would
/// point outside its new vault forever, in git history. `decisions.md` states that
/// cannot happen; without this it could, through any property `to_file` round-trips
/// via `Object::extra`. Non-text variants have no room for a reference and pass
/// through untouched.
pub fn strip_value(v: &PropertyValue, keep_assets: bool) -> PropertyValue {
    match v {
        PropertyValue::Text(s) => PropertyValue::Text(strip_cross_vault(s, keep_assets)),
        PropertyValue::List(items) => {
            PropertyValue::List(items.iter().map(|i| strip_value(i, keep_assets)).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "abc123def4567890abc123def4567890abc123def4567890abc123def4567890";
    const ULID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    #[test]
    fn references_finds_assets_and_notes_normalised() {
        let body = format!(
            "See ![pic](asset:sha256-{H}) and [n](note:{U}) and raw sha256:{H}.",
            H = HASH,
            U = ULID
        );
        let (assets, notes) = references(&body);
        assert_eq!(assets, vec![HASH.to_string()]); // deduped across both spellings
        assert_eq!(notes, vec![ULID.to_string()]);
    }

    #[test]
    fn strip_default_removes_links_assets_and_notes_with_no_label_leak() {
        let body = format!("Idea: ![secret-plan.png](asset:sha256-{HASH}) links [Lab roadmap](note:{ULID}) end.");
        let out = strip_cross_vault(&body, false);
        assert!(!out.contains("note:"));
        assert!(!out.contains("asset:"));
        assert!(!out.contains("sha256"));
        assert!(!out.contains("secret-plan")); // the filename label is gone too
        assert!(!out.contains("Lab roadmap"));
        assert!(out.contains("Idea:") && out.contains("end.")); // prose survives
        assert!(out.contains(REDACTION));
    }

    #[test]
    fn strip_keep_assets_retains_assets_but_still_removes_note_links() {
        let body = format!("![pic](asset:sha256-{HASH}) and [n](note:{ULID})");
        let out = strip_cross_vault(&body, true);
        assert!(out.contains(&format!("asset:sha256-{HASH}"))); // asset kept
        assert!(!out.contains("note:")); // note link still stripped
    }

    #[test]
    fn strip_keeps_external_links_and_images() {
        let body = "See [docs](https://example.com) and ![remote](https://x/y.png).";
        assert_eq!(strip_cross_vault(body, false), body); // nothing to strip
    }

    #[test]
    fn strip_removes_local_relative_image_by_default() {
        let out = strip_cross_vault("![p](./local.png)", false);
        assert!(!out.contains("local.png"));
        assert!(out.contains(REDACTION));
    }

    #[test]
    fn strip_preserves_unicode_prose() {
        let body = format!("café → π ≈ 3.14 [n](note:{ULID}) café", ULID = ULID);
        let out = strip_cross_vault(&body, false);
        assert!(out.starts_with("café → π ≈ 3.14 "));
        assert!(out.ends_with(" café"));
        assert!(!out.contains("note:"));
    }
}
