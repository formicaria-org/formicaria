//! Logseq → [`Page`]. **Pure**: text in, one page out, no filesystem and no graph.
//!
//! # What this file is allowed to know
//!
//! Exactly one source file. Everything that needs more than that — resolving `[[Name]]` to a
//! ULID, inlining a `((block-ref))` that lives on another page, hashing an attachment — belongs
//! to [`super::resolve`], because it needs the whole graph or the disk. So inline tokens are left
//! **in place** here and merely *recorded*; pass two rewrites them. If this file ever needs a
//! `Path`, the seam has leaked.
//!
//! # The structural decision, and where it comes from
//!
//! Logseq's atom is the **block**: every bullet can carry `id:: <uuid>` and be addressed by it.
//! formicaria's atom is the **file**, foundationally — `decisions.md` refuses per-block ids three
//! separate times ("No per-block ids/timestamps — block-level structure is explicitly out of
//! scope"; "Rejected: per-block ids/timestamps — voids the plan"). So a page becomes **one note
//! whose body is the outline, verbatim, as nested Markdown lists**. `id::` is dropped, and a
//! `((ref))` is resolved to the referenced block's *text* — which is what Logseq itself renders,
//! so nothing a reader could see is lost.
//!
//! A `TODO` block becomes a plain `- [ ]` checkbox and is deliberately **not** lifted into
//! `status`; a block's `SCHEDULED:`/`DEADLINE:` is deliberately **not** lifted into `due`. That is
//! the standing ruling on inline actions, not an omission: a checkbox "stays plain Markdown in the
//! body … and does not auto-appear in any planning view", promoted only by a deliberate gesture.
//! Only a **page-level** property becomes a note-level field.

use super::Page;

/// Markers Logseq puts at the head of a bullet.
const OPEN_MARKERS: &[&str] = &["TODO", "DOING", "NOW", "LATER", "WAITING", "IN-PROGRESS"];
const DONE_MARKERS: &[&str] = &["DONE", "CANCELED", "CANCELLED"];

/// Logseq bookkeeping with no meaning here, which would only be noise in a note body. `id` is
/// dropped as a *property* but captured first — it is what a `((ref))` points at.
const DROPPED_PROPS: &[&str] = &["id", "collapsed", "heading", "logseq.order-list-type"];

/// Parse one Logseq `.md` file.
///
/// `title` comes from the filename, which this function cannot see, so the caller passes it in;
/// a page-level `title::` overrides it.
pub fn parse(text: &str, title: &str) -> Page {
    let mut page = Page::new(title);
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    // ── page properties ────────────────────────────────────────────────────────
    // Logseq writes them two ways: bare `key:: value` lines at the top of the file, or inside
    // the first bullet. Both mean "the first block", so accept both and stop at the first line
    // that is neither a property nor blank.
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        let inner = line.strip_prefix("- ").unwrap_or(line).trim();
        match split_property(inner) {
            Some((key, value)) => {
                apply_page_property(&mut page, key, value);
                i += 1;
            }
            None => break,
        }
    }

    // ── the outline ────────────────────────────────────────────────────────────
    let mut out: Vec<String> = Vec::new();
    // Index in `out` of the bullet a continuation line belongs to. `id:: <uuid>` names the block
    // *above* it, which is the only reason this is tracked.
    let mut current_block: Option<usize> = None;
    // The nesting depth of that bullet. A continuation line carries its own indentation, but that
    // indentation is *continuation*, not nesting — reading it as depth pushes every property line
    // and wrapped paragraph one list level deeper than the block it belongs to.
    let mut current_depth = 0usize;
    let mut in_logbook = false;

    while i < lines.len() {
        let raw = lines[i];
        i += 1;
        let trimmed = raw.trim();

        // `:LOGBOOK:` … `:END:` is time-tracking exhaust. Drop the whole span.
        if trimmed == ":LOGBOOK:" {
            in_logbook = true;
            continue;
        }
        if in_logbook {
            in_logbook = trimmed != ":END:";
            continue;
        }

        if trimmed.is_empty() {
            // Keep paragraph breaks; never lead with one, never double one up.
            if out.last().is_some_and(|l: &String| !l.is_empty()) {
                out.push(String::new());
            }
            continue;
        }

        let bullet = trimmed == "-" || trimmed.starts_with("- ");
        let depth = if bullet { depth_of(raw) } else { current_depth };
        let indent = "  ".repeat(depth);
        let content = if bullet { trimmed[1..].trim() } else { trimmed };

        // A `key:: value` line belongs to the block above it whether or not it wears a bullet.
        if let Some((key, value)) = split_property(content) {
            let own = "  ".repeat(current_depth);
            if let Some(line) = block_property(&mut page, &out, current_block, key, value, &own) {
                collect_inline(&mut page, value);
                out.push(line);
            }
            continue;
        }

        if bullet {
            let (marker, body) = split_marker(content);
            let body = strip_priority(body);
            collect_inline(&mut page, body);
            out.push(match marker {
                Marker::Open => format!("{indent}- [ ] {body}"),
                Marker::Done => format!("{indent}- [x] {body}"),
                Marker::None => format!("{indent}- {body}"),
            });
            current_block = Some(out.len() - 1);
            current_depth = depth;
            continue;
        }

        // Not a bullet: a wrapped continuation line, or a file that simply is not an outline.
        collect_inline(&mut page, content);
        out.push(if current_block.is_some() {
            format!("{indent}  {content}")
        } else {
            content.to_string()
        });
    }

    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    page.body = out.join("\n");
    page.finish();
    page
}

// ── properties ──────────────────────────────────────────────────────────────────

/// A page property maps onto a real note field where one exists and into `extra` otherwise —
/// which is what makes "group a board by an imported property" work with no code change.
fn apply_page_property(page: &mut Page, key: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    match key {
        "title" => page.title = value.to_string(),
        // **Commas only.** `decisions.md`, "A tag may contain a space, and both doors must agree
        // what that means" — splitting on spaces is what silently turned `Machine Learning` into
        // two unrelated tags, and it is exactly an imported keyword that suffers.
        "tags" => page.tags.extend(split_list(value)),
        "alias" => page.aliases.extend(split_list(value)),
        "status" => page.status = Some(value.to_string()),
        // The one date lift that IS permitted: a page-level deadline is a property of the note,
        // not of some bullet inside it.
        "deadline" | "due" => page.due = super::parse_date_value(value),
        "scheduled" | "start" => page.start = super::parse_date_value(value),
        // A page that states its own creation date is telling us something truer than the file's
        // mtime — which, after a sync or a checkout, is the day the file was copied.
        "created" | "date" => page.created = super::parse_timestamp_value(value),
        k if DROPPED_PROPS.contains(&k) => {}
        k => {
            page.props.insert(k.to_string(), crate::frontmatter::scalar_property(value));
        }
    }
}

/// A property *inside* a block. `id::` is captured (a `((ref))` points at it) then dropped, and
/// so is pure bookkeeping. Anything else the user typed is kept as readable text — carrying it
/// as prose beats throwing it away.
///
/// Returns the line to emit, or `None` when the property is consumed.
fn block_property(
    page: &mut Page,
    out: &[String],
    current_block: Option<usize>,
    key: &str,
    value: &str,
    indent: &str,
) -> Option<String> {
    if key == "id" {
        if let Some(text) = current_block.map(|i| bullet_text(&out[i])) {
            page.block_texts.insert(value.trim().to_ascii_lowercase(), text);
        }
        return None;
    }
    if DROPPED_PROPS.contains(&key) {
        return None;
    }
    Some(format!("{indent}  {key}: {value}"))
}

/// A bullet's own words — what a `((uuid))` elsewhere is replaced by.
fn bullet_text(line: &str) -> String {
    let t = line.trim_start();
    let t = t.strip_prefix("- ").unwrap_or(t);
    let t = t.strip_prefix("[ ] ").or_else(|| t.strip_prefix("[x] ")).unwrap_or(t);
    t.trim().to_string()
}

// ── block markers ───────────────────────────────────────────────────────────────

enum Marker {
    Open,
    Done,
    None,
}

fn split_marker(content: &str) -> (Marker, &str) {
    for m in OPEN_MARKERS {
        if let Some(rest) = strip_word(content, m) {
            return (Marker::Open, rest);
        }
    }
    for m in DONE_MARKERS {
        if let Some(rest) = strip_word(content, m) {
            return (Marker::Done, rest);
        }
    }
    (Marker::None, content)
}

/// A marker only when it is a whole word at the head, so a bullet reading "Doing the laundry"
/// is not mistaken for a `DOING` block.
fn strip_word<'a>(s: &'a str, word: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(word)?;
    if rest.is_empty() {
        return Some(rest);
    }
    rest.starts_with(' ').then(|| rest.trim_start())
}

/// Logseq priority cookies (`[#A]`) have no home here and read as noise in prose.
fn strip_priority(s: &str) -> &str {
    for p in ["[#A]", "[#B]", "[#C]"] {
        if let Some(rest) = s.strip_prefix(p) {
            return rest.trim_start();
        }
    }
    s
}

// ── inline tokens: recorded here, rewritten in pass two ─────────────────────────

fn collect_inline(page: &mut Page, s: &str) {
    super::scan_wikilinks(s, &mut page.links);
    super::scan_block_refs(s, &mut page.block_refs);
    super::scan_hashtags(s, &mut page.tags);
    super::scan_local_images(s, &mut page.attachments);
}

// ── small helpers ───────────────────────────────────────────────────────────────

/// `key:: value`. `None` for anything else — including ordinary prose that happens to contain
/// `::`, which is why a key may not hold whitespace.
fn split_property(s: &str) -> Option<(&str, &str)> {
    let at = s.find("::")?;
    let key = s[..at].trim();
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return None;
    }
    Some((key, s[at + 2..].trim()))
}

/// Indentation depth: tabs, or two spaces per level. Both are Logseq defaults.
fn depth_of(raw: &str) -> usize {
    let tabs = raw.chars().take_while(|c| *c == '\t').count();
    if tabs > 0 {
        return tabs;
    }
    raw.chars().take_while(|c| *c == ' ').count() / 2
}

fn split_list(s: &str) -> Vec<String> {
    s.split(',').map(clean_tag).filter(|t| !t.is_empty()).collect()
}

/// Strip the `[[ ]]` a Logseq tag list often wraps entries in. A comma is the separator, so it
/// can never survive inside one tag.
fn clean_tag(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("[[").unwrap_or(t);
    let t = t.strip_suffix("]]").unwrap_or(t);
    t.trim().replace(',', " ").trim().to_string()
}
