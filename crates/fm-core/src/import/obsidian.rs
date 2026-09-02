//! Obsidian → [`Page`]. **Pure**: text in, one page out, no filesystem and no graph.
//!
//! Obsidian is the easy half, and it is worth saying why: it is already one note per file with
//! YAML frontmatter, so the note *model* needs no translation at all. What does need translating
//! is the two things Obsidian spells differently from formicaria — `[[wikilinks]]` (which here are
//! `[label](note:<ULID>)`) and inline `#tags` (which here live in frontmatter, exclusively). Both
//! need the whole graph, so this file only *records* them; [`super::resolve`] rewrites them.
//!
//! Frontmatter is read through the same [`crate::frontmatter::split_fence`] the app uses on its
//! own notes rather than a second fence matcher. That is deliberate: the CRLF case behind it cost
//! a whole vault opening empty on Windows once, and a copy of that logic is a copy that can drift.

use super::Page;
use serde_yaml_ng::Value;

/// Frontmatter keys that mean something to a note rather than being a custom property.
/// `id`/`schema` are dropped outright: they belong to *our* file format, and honouring one out of
/// a foreign file would let an imported note claim an identity the vault has already issued.
const DROPPED_KEYS: &[&str] = &["id", "schema", "type", "vault", "updated"];

/// Parse one Obsidian `.md` file. `title` comes from the filename; frontmatter `title:` wins.
pub fn parse(text: &str, title: &str) -> Page {
    let mut page = Page::new(title);

    let body = match crate::frontmatter::split_fence(text) {
        Some((yaml, body)) => {
            read_frontmatter(&mut page, yaml);
            body
        }
        // No frontmatter is the common case in Obsidian, and it is not an error.
        None => text,
    };

    page.body = body.trim_matches('\n').to_string();
    super::scan_wikilinks(&page.body.clone(), &mut page.links);
    super::scan_hashtags(&page.body.clone(), &mut page.tags);
    super::scan_local_images(&page.body.clone(), &mut page.attachments);
    page.finish();
    page
}

fn read_frontmatter(page: &mut Page, yaml: &str) {
    // A frontmatter block we cannot parse is not worth failing an import over — the body is the
    // substance. Record it so the report can say so rather than silently dropping metadata.
    let Ok(value) = serde_yaml_ng::from_str::<Value>(yaml) else {
        page.warnings.push("its frontmatter is not valid YAML and was skipped".into());
        return;
    };
    let Value::Mapping(map) = value else { return };

    for (k, v) in map {
        let Some(key) = k.as_str() else { continue };
        match key {
            "title" => {
                if let Some(t) = v.as_str().filter(|t| !t.trim().is_empty()) {
                    page.title = t.trim().to_string();
                }
            }
            "tags" | "tag" => page.tags.extend(string_list(&v)),
            "aliases" | "alias" => page.aliases.extend(string_list(&v)),
            "status" => {
                if let Some(s) = v.as_str() {
                    page.status = Some(s.trim().to_string());
                }
            }
            "due" | "deadline" => page.due = v.as_str().and_then(super::parse_date_value),
            "start" | "scheduled" => page.start = v.as_str().and_then(super::parse_date_value),
            // Obsidian templates commonly stamp one of these; it is a far better `created` than
            // the mtime, and better still than the day the import ran.
            "created" | "date" | "created_at" => {
                page.created = v.as_str().and_then(super::parse_timestamp_value);
            }
            k if DROPPED_KEYS.contains(&k) => {}
            k => {
                page.props.insert(k.to_string(), crate::frontmatter::yaml_to_prop(&v));
            }
        }
    }
}

/// A scalar, or a sequence, or nothing. A **scalar splits on commas only** — never on spaces:
/// `decisions.md`, "A tag may contain a space, and both doors must agree what that means".
/// Obsidian's own tags cannot contain a space, so this is lossless for them and correct for the
/// multi-word keywords people put in `aliases`.
fn string_list(v: &Value) -> Vec<String> {
    match v {
        Value::String(s) => s.split(',').map(clean).filter(|t| !t.is_empty()).collect(),
        Value::Sequence(items) => items
            .iter()
            .filter_map(|i| i.as_str())
            .map(clean)
            .filter(|t| !t.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Obsidian writes a nested tag as `a/b` and sometimes with a leading `#`; neither is a
/// separator here, so only the `#` goes.
fn clean(s: &str) -> String {
    s.trim().trim_start_matches('#').trim().replace(',', " ").trim().to_string()
}
