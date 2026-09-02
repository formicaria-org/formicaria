//! Importing a Logseq graph or an Obsidian vault.
//!
//! # Why this is a conversion and not an adoption
//!
//! Pointing a vault at someone's Logseq folder *looks* like it should already work — `inspect_path`
//! even promises "adoption is free". It cannot, for three reasons in code: [`crate::FileStore`]'s
//! reindex is a **flat** `read_dir` (so `journals/` and `pages/` are invisible), a file with no
//! `---` fence fails `frontmatter::from_file` and is *skipped*, and `path_for(id)` makes the
//! **filename the id**. So an import mints a ULID per page, writes `<ULID>.md`, and rewrites the
//! source's link and tag syntax into ours.
//!
//! That is deliberately **not** Track V4 adoption, which is the opposite premise: V4 renders a
//! `.md` you keep owning elsewhere, with a transient id and *nothing written into your repo*. This
//! is for a graph you are migrating away from. The one idea worth borrowing from V4 is "use git for
//! what only git knows" — see [`created_from_git`].
//!
//! **The source folder is opened read-only and is never written to.** That invariant is what keeps
//! an import from becoming the thing V4 exists to avoid, and it is asserted in the tests.
//!
//! # The shape of the work
//!
//! Two passes, because a link needs an id that does not exist while the file holding it is being
//! read:
//!
//! 1. [`scan`] / [`parse_all`] — walk, parse each file into a [`Page`], mint a ULID for each, and
//!    build the graph (page names, aliases, and the `id::`-bearing blocks a `((ref))` points at).
//! 2. [`rewrite`] — with the graph in hand, turn `[[Name]]` into `[Name](note:<ULID>)`, `((uuid))`
//!    into the text it names, and a local image into `![alt](asset:sha256-…)`.
//!
//! [`rewrite`] is **pure** — attachments are hashed *before* it runs and handed to it as a map — so
//! the whole translation is testable with no filesystem at all.

use crate::{Ingested, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue, Stamp};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub mod logseq;
pub mod obsidian;

#[cfg(test)]
mod tests;

/// Directories that are never a note: version control, the apps' own state, and the caches they
/// keep beside it. Walking into `.git` on a large graph is also the difference between a scan that
/// feels instant and one that does not.
const SKIP_DIRS: &[&str] =
    &[".git", ".obsidian", ".trash", ".stfolder", "node_modules", "bak", "version-files", ".recycle"];

/// Files an import has no landing site for. Counted and named in the report — *"left behind"* is a
/// thing the user should be told, not something to discover later.
const LEFT_BEHIND_EXT: &[&str] = &["org", "canvas", "excalidraw", "edn"];

// ── what a source file becomes ──────────────────────────────────────────────────

/// One source page, parsed. The intermediate between a parser (which sees one file) and
/// [`rewrite`] (which sees the graph).
#[derive(Debug, Default, Clone)]
pub struct Page {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub props: BTreeMap<String, PropertyValue>,
    pub status: Option<String>,
    pub due: Option<Stamp>,
    pub start: Option<Stamp>,
    /// Set only when the *file itself* says so (a journal's date, an Obsidian `created:`).
    /// Otherwise the walker supplies one — see [`created_from_git`].
    pub created: Option<OffsetDateTime>,
    /// Page names this body links to, as written. Recorded, not resolved.
    pub links: Vec<String>,
    /// `((uuid))` references this body makes.
    pub block_refs: Vec<String>,
    /// `id:: <uuid>` → that block's own text, for whoever references it.
    pub block_texts: BTreeMap<String, String>,
    /// Local image/file paths as written in the body.
    pub attachments: Vec<String>,
    pub warnings: Vec<String>,
}

impl Page {
    pub fn new(title: &str) -> Self {
        Page { title: title.to_string(), ..Default::default() }
    }

    /// Tidy what the parsers accumulated. Called once, by the parser, before handing the page on.
    pub fn finish(&mut self) {
        for v in [&mut self.tags, &mut self.aliases, &mut self.links, &mut self.block_refs, &mut self.attachments] {
            v.retain(|s| !s.trim().is_empty());
            v.sort();
            v.dedup();
        }
    }
}

/// Which app wrote the folder. There is no "plain folder of Markdown" arm on purpose: without a
/// graph there is nothing to resolve links against, and a silent half-import is worse than a
/// refusal that names what it wanted to see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Logseq,
    Obsidian,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Logseq => "logseq",
            Format::Obsidian => "obsidian",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Format::Logseq => "Logseq",
            Format::Obsidian => "Obsidian",
        }
    }
}

// ── the preview ─────────────────────────────────────────────────────────────────

/// What is in the folder, answered **without parsing anything**. This is the `check_path` half of
/// the pair: facts, cheap enough to ask on a keystroke, so the surface can state the size of the
/// import before the button is pressed.
#[derive(Debug, Default, Clone)]
pub struct Scan {
    pub format: Option<Format>,
    pub pages: usize,
    pub journals: usize,
    pub attachments: usize,
    pub attachment_bytes: u64,
    /// Files with no landing site here (`.org`, `.canvas`, …), by extension and count.
    pub left_behind: Vec<(String, usize)>,
    /// Why this folder cannot be imported. Empty means it can.
    pub problems: Vec<String>,
}

impl Scan {
    pub fn ok(&self) -> bool {
        self.problems.is_empty() && self.format.is_some() && self.pages + self.journals > 0
    }
}

/// Look at a folder and say what importing it would involve.
pub fn scan(source: &Path) -> Scan {
    let mut out = Scan::default();

    if !source.is_dir() {
        out.problems.push(if source.exists() {
            "that is a file, not a folder — point at the graph's folder".into()
        } else {
            "there is no folder at that path".into()
        });
        return out;
    }

    out.format = detect(source);
    if out.format.is_none() {
        out.problems.push(
            "that folder does not look like a Logseq graph or an Obsidian vault — we look for \
             `logseq/`, a `journals/` or `pages/` folder, or `.obsidian/`"
                .into(),
        );
        return out;
    }

    let mut left: BTreeMap<String, usize> = BTreeMap::new();
    let mut files = Vec::new();
    if let Err(e) = walk(source, &mut files) {
        out.problems.push(format!("that folder could not be read: {e}"));
        return out;
    }

    for path in &files {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "md" {
            if is_journal(source, path) {
                out.journals += 1;
            } else {
                out.pages += 1;
            }
        } else if LEFT_BEHIND_EXT.contains(&ext.as_str()) {
            *left.entry(ext).or_default() += 1;
        } else {
            out.attachments += 1;
            out.attachment_bytes += path.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    out.left_behind = left.into_iter().collect();

    if out.pages + out.journals == 0 {
        out.problems.push("there are no Markdown files in that folder".into());
    }
    out
}

/// `logseq/` or the journals/pages pair says Logseq; `.obsidian/` says Obsidian. Logseq is tested
/// first because a graph that has been opened in both carries both markers, and the Logseq reading
/// is the lossier one to get wrong (its outline and `key::` properties need real translation,
/// where an Obsidian file read as Logseq would merely pass through).
pub fn detect(source: &Path) -> Option<Format> {
    if source.join("logseq").is_dir()
        || (source.join("journals").is_dir() || source.join("pages").is_dir())
    {
        return Some(Format::Logseq);
    }
    source.join(".obsidian").is_dir().then_some(Format::Obsidian)
}

// ── walking ─────────────────────────────────────────────────────────────────────

/// Every file under `root`, recursively, skipping the directories in [`SKIP_DIRS`] and anything
/// else hidden. Recursive **because the sources are** — which is exactly what `FileStore`'s flat
/// reindex cannot do, and the reason an import has to exist at all.
fn walk(root: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let ty = entry.file_type()?;
            if ty.is_dir() {
                // Symlinks are not followed and depth is capped: `scan` is asked on a keystroke
                // and can be pointed at `/`, so a link back to a parent must not become a walk
                // that never returns.
                if depth + 1 > MAX_DEPTH
                    || ty.is_symlink()
                    || name.starts_with('.')
                    || SKIP_DIRS.contains(&name.as_str())
                {
                    continue;
                }
                stack.push((path, depth + 1));
            } else if ty.is_file() && !name.starts_with('.') {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn is_journal(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|r| r.components().next())
        .is_some_and(|c| c.as_os_str().eq_ignore_ascii_case("journals"))
}

// ── page names ──────────────────────────────────────────────────────────────────

/// A source filename back into the page name a `[[link]]` would spell.
///
/// Logseq escapes a namespace separator in the filename — `Parent%2FChild.md` in older graphs and
/// `Parent___Child.md` since the `triple-lowbar` default — so both un-escape to `Parent/Child`, or
/// a link to a namespaced page silently fails to resolve.
pub fn page_name(stem: &str) -> String {
    stem.replace("___", "/").replace("%2F", "/").replace("%2f", "/")
}

/// The key both a page name and a `[[link]]` are looked up by. Logseq and Obsidian both treat page
/// names case-insensitively, so matching case-sensitively would break links that work today.
pub fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

// ── dates ───────────────────────────────────────────────────────────────────────

/// A Logseq/Obsidian date property: `<2026-09-02 Tue>`, `[[2026-09-02]]` or a bare `2026-09-02`.
/// Anything else is left alone rather than guessed at.
pub fn parse_date_value(raw: &str) -> Option<Stamp> {
    let t = raw.trim();
    let t = t.trim_start_matches('<').trim_end_matches('>');
    let t = t.trim_start_matches("[[").trim_end_matches("]]");
    let t = t.trim();
    // `<2026-09-02 Tue 10:00>` — take the date, and the time when it is a plain `HH:MM`.
    let mut parts = t.split_whitespace();
    let day = parts.next()?;
    match parts.last().filter(|p| p.len() == 5 && p.as_bytes()[2] == b':') {
        Some(time) => format!("{day}T{time}").parse().ok(),
        None => day.parse().ok(),
    }
}

/// The same, as an instant, for `created`. Accepts a full RFC 3339 stamp (what an Obsidian
/// template usually writes) or a bare day, which becomes midnight UTC.
pub fn parse_timestamp_value(raw: &str) -> Option<OffsetDateTime> {
    let t = raw.trim();
    if let Ok(dt) = OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339) {
        return Some(dt);
    }
    let stamp: Stamp = parse_date_value(t)?;
    Some(stamp.date.midnight().assume_utc())
}

/// A Logseq journal filename (`2026_09_02.md`, or `2026-09-02.md`) as the day it stands for.
pub fn journal_date(stem: &str) -> Option<OffsetDateTime> {
    let norm = stem.replace('_', "-");
    parse_timestamp_value(&norm)
}

fn mtime_of(path: &Path) -> OffsetDateTime {
    path.metadata()
        .and_then(|m| m.modified())
        .map(OffsetDateTime::from)
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
}

// ── inline token scanners (pure; used by both parsers) ──────────────────────────
//
// Hand-rolled, deliberately. `refs.rs` already makes this call for the same reason — "these are a
// handful of fixed-shape tokens and Markdown link/image spans, cheaper to scan by hand than to pull
// the crate" — and adding `regex` to what ships would need a `decisions.md` entry of its own.

/// `[[Name]]`, `[[Name|alias]]`, `[[Name#Heading]]` → the page name, as written.
pub(crate) fn scan_wikilinks(s: &str, out: &mut Vec<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        // `#[[two words]]` is a **tag**, not a link — `scan_hashtags` owns it. Counting it here
        // too reported every multi-word tag as a dangling link, and would have rewritten one into
        // `#[label](note:…)` the moment a page happened to share its name.
        if b[i] == b'[' && b[i + 1] == b'[' && !(i > 0 && b[i - 1] == b'#') {
            if let Some(end) = s[i + 2..].find("]]") {
                let inner = &s[i + 2..i + 2 + end];
                if let Some(name) = link_target(inner) {
                    out.push(name.to_string());
                }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
}

/// The page half of a wikilink body: before `|` (the alias) and before `#` (a heading anchor,
/// which has no representation here — the atom is the file).
fn link_target(inner: &str) -> Option<&str> {
    let target = inner.split('|').next()?.split('#').next()?.trim();
    (!target.is_empty()).then_some(target)
}

/// `((uuid))` → the uuid.
pub(crate) fn scan_block_refs(s: &str, out: &mut Vec<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'(' && b[i + 1] == b'(' {
            if let Some(end) = s[i + 2..].find("))") {
                let inner = s[i + 2..i + 2 + end].trim();
                if !inner.is_empty() && !inner.contains(char::is_whitespace) {
                    out.push(inner.to_ascii_lowercase());
                }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
}

/// `#tag` and `#[[two words]]`.
///
/// A `#` counts only at the start of the text or after whitespace, which is what keeps a URL
/// fragment (`…/page#section`) and a CSS colour out of the tag list. A `#` followed by a space is
/// a Markdown heading and not a tag — and note the converse, which is why nothing is escaped on
/// the way out: CommonMark (and GFM, and `marked`) require that space for an ATX heading, so a
/// bare `#tag` at the start of a line renders as the literal text it is.
pub(crate) fn scan_hashtags(s: &str, out: &mut Vec<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'#' || (i > 0 && !(b[i - 1] as char).is_whitespace()) {
            i += 1;
            continue;
        }
        let rest = &s[i + 1..];
        if let Some(inner) = rest.strip_prefix("[[") {
            if let Some(end) = inner.find("]]") {
                let name = inner[..end].trim();
                if !name.is_empty() {
                    out.push(name.replace(',', " ").trim().to_string());
                }
                i += 1 + 2 + end + 2;
                continue;
            }
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '"' | '\'' | ')' | ']'))
            .unwrap_or(rest.len());
        let name = rest[..end].trim_end_matches('.');
        if !name.is_empty() && !name.starts_with('#') {
            out.push(name.to_string());
        }
        i += 1 + end.max(1);
    }
}

/// `![alt](path)` where `path` is local — an `http(s)`/`data` image is somebody else's URL and
/// stays exactly as it is.
pub(crate) fn scan_local_images(s: &str, out: &mut Vec<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'!' && b[i + 1] == b'[' {
            if let Some(span) = md_span(s, i + 1) {
                let url = s[span.0..span.1].trim();
                if is_local(url) {
                    out.push(decode_url(url));
                }
                i = span.2;
                continue;
            }
        }
        i += 1;
    }
}

/// `(url_start, url_end, one_past_close)` for a `[label](url)` whose `[` is at `open`.
fn md_span(s: &str, open: usize) -> Option<(usize, usize, usize)> {
    let b = s.as_bytes();
    let close = (open + 1..b.len()).find(|&j| b[j] == b']')?;
    if close + 1 >= b.len() || b[close + 1] != b'(' {
        return None;
    }
    let paren = (close + 2..b.len()).find(|&j| b[j] == b')')?;
    Some((close + 2, paren, paren + 1))
}

fn is_local(url: &str) -> bool {
    !url.is_empty()
        && !url.starts_with("http://")
        && !url.starts_with("https://")
        && !url.starts_with("data:")
        && !url.starts_with('#')
}

/// `%20` and friends — Obsidian percent-encodes a space in a Markdown link but not in an embed.
///
/// Decodes to **bytes** and only then to text: a percent-escape is a byte, so `%C3%A9` is one `é`
/// and not two mojibake characters. Rebuilding it char-by-char is the classic way to turn every
/// accented filename in a graph into a file that cannot be found.
fn decode_url(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ── pass two: resolution ────────────────────────────────────────────────────────

/// One attachment, already hashed into the blob store.
#[derive(Debug, Clone)]
pub struct Asset {
    pub hash: String,
    pub filename: String,
}

/// Everything pass two needs that a single file cannot supply: which ULID a page name got, what
/// an `id::`-bearing block said, and where an attachment's bytes ended up.
///
/// Building this is the whole reason an import is two passes — a `[[link]]` in the first file read
/// may name a page that will not be read until the last.
#[derive(Debug, Default)]
pub struct Graph {
    pages: HashMap<String, Id>,
    blocks: HashMap<String, String>,
    assets: HashMap<String, Asset>,
}

impl Graph {
    pub fn page(&self, name: &str) -> Option<Id> {
        self.pages.get(&name_key(name)).copied()
    }
    pub fn block(&self, uuid: &str) -> Option<&str> {
        self.blocks.get(&uuid.to_ascii_lowercase()).map(String::as_str)
    }
    pub fn asset(&self, reference: &str) -> Option<&Asset> {
        self.assets.get(&name_key(reference)).or_else(|| {
            // An `![[image.png]]` names a file, not a path; fall back to its bare name.
            let base = reference.rsplit('/').next()?;
            self.assets.get(&name_key(base))
        })
    }
    pub fn add_page(&mut self, name: &str, id: Id) {
        self.pages.entry(name_key(name)).or_insert(id);
    }
    pub fn add_asset(&mut self, reference: &str, asset: Asset) {
        self.assets.insert(name_key(reference), asset);
    }
}

/// What a rewrite had to leave behind, so the report can say it out loud.
#[derive(Debug, Default, Clone)]
pub struct Rewritten {
    pub links_resolved: usize,
    /// Page names linked to that no file defines. Logseq creates a page on reference, so these
    /// are normal and numerous — they are *counted*, and only become notes if the user asks.
    pub dangling: Vec<String>,
    pub blocks_inlined: usize,
    pub blocks_unresolved: usize,
    pub assets_linked: usize,
}

/// Turn one body's foreign syntax into ours. **Pure** — attachments were hashed before this ran.
pub fn rewrite(body: &str, graph: &Graph, out_stats: &mut Rewritten) -> String {
    let b = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;

    while i < b.len() {
        // A bracketed hashtag is a tag and is already in the frontmatter; it passes through as the
        // text the author wrote. Handled before the link arm, which would otherwise claim it.
        if b[i] == b'#' && body[i + 1..].starts_with("[[") {
            if let Some(end) = body[i + 3..].find("]]") {
                out.push_str(&body[i..i + 3 + end + 2]);
                i += 3 + end + 2;
                continue;
            }
        }

        // `{{embed [[Name]]}}` / `{{embed ((uuid))}}`
        if body[i..].starts_with("{{embed ") {
            if let Some(end) = body[i..].find("}}") {
                let inner = body[i + 8..i + end].trim();
                if let Some(text) = embed_of(inner, graph, out_stats) {
                    out.push_str(&text);
                    i += end + 2;
                    continue;
                }
            }
        }

        // `![[Target]]` — an embed of a page, or of a file.
        if body[i..].starts_with("![[") {
            if let Some(end) = body[i + 3..].find("]]") {
                let inner = &body[i + 3..i + 3 + end];
                if let Some(text) = embed_target(inner, graph, out_stats) {
                    out.push_str(&text);
                    i += 3 + end + 2;
                    continue;
                }
            }
        }

        // `[[Target]]` / `[[Target|alias]]`
        if body[i..].starts_with("[[") {
            if let Some(end) = body[i + 2..].find("]]") {
                let inner = &body[i + 2..i + 2 + end];
                if let Some(text) = link_of(inner, graph, out_stats) {
                    out.push_str(&text);
                    i += 2 + end + 2;
                    continue;
                }
            }
        }

        // `((uuid))` → the words that block actually says, which is what Logseq renders.
        if body[i..].starts_with("((") {
            if let Some(end) = body[i + 2..].find("))") {
                let uuid = body[i + 2..i + 2 + end].trim();
                match graph.block(uuid) {
                    Some(text) => {
                        out.push_str(text);
                        out_stats.blocks_inlined += 1;
                        i += 2 + end + 2;
                        continue;
                    }
                    // A reference to a block on a page that was not imported. Leaving the raw
                    // `((uuid))` would be debris; the honest thing is to say what it was.
                    None if is_uuidish(uuid) => {
                        out.push_str("⟨referenced block not imported⟩");
                        out_stats.blocks_unresolved += 1;
                        i += 2 + end + 2;
                        continue;
                    }
                    None => {}
                }
            }
        }

        // `![alt](path)` and `[label](path)` over a local target.
        if b[i] == b'[' || (b[i] == b'!' && i + 1 < b.len() && b[i + 1] == b'[') {
            let is_img = b[i] == b'!';
            let bracket = if is_img { i + 1 } else { i };
            if let Some((us, ue, end)) = md_span(body, bracket) {
                let label = &body[bracket + 1..us - 2];
                let url = body[us..ue].trim();
                if let Some(text) = md_link(label, url, is_img, graph, out_stats) {
                    out.push_str(&text);
                    i = end;
                    continue;
                }
            }
        }

        let ch = utf8_len(b[i]);
        out.push_str(&body[i..i + ch]);
        i += ch;
    }
    out
}

fn embed_of(inner: &str, graph: &Graph, s: &mut Rewritten) -> Option<String> {
    if let Some(rest) = inner.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
        return embed_target(rest, graph, s);
    }
    let uuid = inner.strip_prefix("((")?.strip_suffix("))")?.trim();
    let text = graph.block(uuid)?;
    s.blocks_inlined += 1;
    Some(text.to_string())
}

/// An embed resolves to a page (an inline note card) or to a file (an image). Both are the `!`
/// spelling of an ordinary link, which is exactly what `render.ts` already draws.
fn embed_target(inner: &str, graph: &Graph, s: &mut Rewritten) -> Option<String> {
    let target = link_target(inner)?;
    if let Some(id) = graph.page(target) {
        s.links_resolved += 1;
        return Some(format!("![{}]({})", escape_label(target), fm_model::note_ref(id)));
    }
    if let Some(asset) = graph.asset(target) {
        s.assets_linked += 1;
        return Some(format!("![{}](asset:sha256-{})", escape_label(&asset.filename), asset.hash));
    }
    s.dangling.push(target.to_string());
    None
}

fn link_of(inner: &str, graph: &Graph, s: &mut Rewritten) -> Option<String> {
    let target = link_target(inner)?;
    // `[[Page|shown]]` — the alias is what the reader sees, so it is the label.
    let label = inner.split_once('|').map(|(_, a)| a.trim()).filter(|a| !a.is_empty()).unwrap_or(target);
    match graph.page(target) {
        Some(id) => {
            s.links_resolved += 1;
            Some(format!("[{}]({})", escape_label(label), fm_model::note_ref(id)))
        }
        None => {
            // Left verbatim on purpose: a page that exists only as a reference is normal in
            // Logseq, and inventing a note for it would put an empty card in every view.
            s.dangling.push(target.to_string());
            None
        }
    }
}

/// A Markdown link or image whose target is a local file: an attachment becomes a blob reference,
/// and a link to another note's `.md` becomes a real note link.
fn md_link(
    label: &str,
    url: &str,
    is_img: bool,
    graph: &Graph,
    s: &mut Rewritten,
) -> Option<String> {
    if !is_local(url) {
        return None;
    }
    let decoded = decode_url(url);
    if let Some(asset) = graph.asset(&decoded) {
        s.assets_linked += 1;
        let label = if label.is_empty() { &asset.filename } else { label };
        return Some(format!(
            "{}[{}](asset:sha256-{})",
            if is_img { "!" } else { "" },
            escape_label(label),
            asset.hash
        ));
    }
    // `[text](Some Note.md)` — Obsidian's other way of spelling a link.
    let stem = decoded.strip_suffix(".md")?.rsplit('/').next()?;
    let id = graph.page(&page_name(stem))?;
    s.links_resolved += 1;
    Some(format!("[{}]({})", escape_label(label), fm_model::note_ref(id)))
}

/// A label sits inside `[...]`, so the three characters that could close it early are escaped —
/// the same set `NotePanel`'s own `escapeLabel` handles when it inserts a reference.
fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

/// Loose enough for both Logseq's uuids and a hand-typed one, strict enough that prose in double
/// parentheses is not mistaken for a reference.
fn is_uuidish(s: &str) -> bool {
    s.len() >= 8 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn utf8_len(first: u8) -> usize {
    match first {
        b if b < 0x80 => 1,
        b if b >> 5 == 0b110 => 2,
        b if b >> 4 == 0b1110 => 3,
        _ => 4,
    }
}

// ── the conversion ──────────────────────────────────────────────────────────────

/// Where an imported note records what it came from. Reserved names, and the two `papers-plan.md`
/// Phase E asks for — so the Zotero port inherits the convention instead of inventing a second one.
pub const SOURCE_KEY: &str = "source_key";
pub const SOURCE_LIBRARY: &str = "source_library";

/// Frontmatter keys that already mean something, and must never be taken by an imported property.
///
/// **This is a data-loss guard, not tidiness.** `to_file` writes `extra` into the *same* YAML
/// mapping as the well-known keys and `Mapping::insert` overwrites, so a Logseq `created:: 2021` or
/// an Obsidian `type:` would replace ours — and `from_file` then fails to parse the note's own
/// `created`, turning it into a `SkippedNote` that appears in no view at all. The four thread and
/// proposal keys are here for a second reason: `thread::notes_base` *hides* a note carrying them,
/// so an imported `base:` would silently vanish from every board, agenda and timeline.
const RESERVED_KEYS: &[&str] = &[
    "schema", "id", "type", "title", "status", "start", "due", "hard", "created", "updated",
    "tags", "assets", "code", "vault", "thread_of", "reply_to", "proposes", "base",
    SOURCE_KEY, SOURCE_LIBRARY,
];

/// How deep a source tree may nest before we stop. A graph is a handful of levels; anything past
/// this is a link loop or a mistyped path like `/`, and `scan` is asked on a keystroke.
const MAX_DEPTH: usize = 24;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Create an empty note for a page that is only ever linked to. Off by default: those notes
    /// are `Kind::Note`, so `thread::notes_base` shows them in **every** board, agenda, timeline
    /// and feed — and nothing in the app is virtualised.
    pub create_stubs: bool,
}

/// What an import did, in the words the surface will repeat.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub format: String,
    pub notes_added: usize,
    pub notes_already_imported: usize,
    pub stubs_created: usize,
    pub attachments_added: usize,
    pub attachments_deduped: usize,
    pub links_resolved: usize,
    pub dangling: usize,
    /// A sample, not the whole list — enough to recognise a systematic failure.
    pub dangling_names: Vec<String>,
    pub blocks_inlined: usize,
    pub blocks_unresolved: usize,
    pub renamed_properties: usize,
    pub left_behind: Vec<(String, usize)>,
    pub warnings: Vec<String>,
    pub committed: bool,
}

/// Notes and attachments, ready to be written. Deliberately **not** written here: this function
/// takes no `Store` and no lock, which is what lets the caller do all of this — including the
/// subprocess-per-PDF text extraction — with the app's mutex released.
pub struct Converted {
    pub format: Format,
    /// `vault` is left empty; the caller stamps it. See `MultiStore::put`.
    pub notes: Vec<Object>,
    pub attachments: Vec<Ingested>,
    pub report: Report,
}

/// Read a Logseq graph or Obsidian vault and convert it.
///
/// `existing` maps a [`SOURCE_KEY`] to the note that already carries it, from a previous import
/// into this vault. Those pages are **not** re-emitted — an import adds what it has not seen and
/// never rewrites a note, because `refuse_if_stale` compares the *indexed* mtime and so would not
/// catch an edit made through the app: "update in place" would silently overwrite the user's own
/// work. They still enter the graph, so links to them resolve to the note that is already there.
pub fn convert(
    source: &Path,
    vault: &Path,
    existing: &HashMap<String, Id>,
    opts: Options,
) -> Result<Converted, StoreError> {
    let format = detect(source)
        .ok_or_else(|| StoreError::Io("that folder is not a Logseq graph or an Obsidian vault".into()))?;

    // **The source may not contain the destination.** Otherwise the vault's own `<ULID>.md` files
    // and blobs land inside the graph — breaking the read-only promise — and the next import reads
    // its own output back in.
    let (src_c, vault_c) = (canonical(source), canonical(vault));
    if vault_c.starts_with(&src_c) || src_c.starts_with(&vault_c) {
        return Err(StoreError::Io(
            "the vault and the folder being imported cannot be inside one another — choose a \
             folder outside the graph"
                .into(),
        ));
    }

    let mut files = Vec::new();
    walk(source, &mut files).map_err(|e| StoreError::Io(format!("could not read that folder: {e}")))?;

    let mut report = Report { format: format.as_str().to_string(), ..Default::default() };

    // Split the walk once: Markdown is a page, everything else is a candidate attachment.
    let (markdown, others): (Vec<_>, Vec<_>) = files
        .into_iter()
        .partition(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("md")));

    let mut left: BTreeMap<String, usize> = BTreeMap::new();
    let mut file_index: HashMap<String, PathBuf> = HashMap::new();
    for path in &others {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if LEFT_BEHIND_EXT.contains(&ext.as_str()) {
            *left.entry(ext).or_default() += 1;
            continue;
        }
        if let Ok(rel) = path.strip_prefix(source) {
            file_index.insert(name_key(&rel.to_string_lossy()), path.clone());
        }
        if let Some(base) = path.file_name().and_then(|n| n.to_str()) {
            file_index.entry(name_key(base)).or_insert_with(|| path.clone());
        }
    }
    report.left_behind = left.into_iter().collect();

    // ── pass one: parse, mint ids, build the graph ─────────────────────────────
    let git_dates = git_added_dates(source);
    let mut graph = Graph::default();
    let mut parsed: Vec<Parsed> = Vec::with_capacity(markdown.len());

    for path in markdown {
        let Ok(rel) = path.strip_prefix(source) else { continue };
        let source_key = rel.to_string_lossy().replace('\\', "/");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");
        let name = page_name(stem);

        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            // A file we cannot read as text is not a note. Say which one rather than stopping.
            Err(e) => {
                report.warnings.push(format!("{source_key} could not be read ({e}) and was skipped"));
                continue;
            }
        };

        let mut page = match format {
            Format::Logseq => logseq::parse(&text, &name),
            Format::Obsidian => obsidian::parse(&text, &name),
        };

        // A journal's date is in its filename and is the best `created` there is; otherwise the
        // file's own frontmatter, then git, then the mtime.
        let journal = journal_date(stem).filter(|_| is_journal(source, &path));
        let created = journal
            .or(page.created)
            .or_else(|| git_dates.get(&source_key).copied())
            .unwrap_or_else(|| mtime_of(&path));
        page.created = Some(created);
        if let Some(day) = journal {
            // `2026_09_02` is a filename, not a title. The day it stands for is what a person
            // reading a list of notes needs to see.
            page.title = day.date().to_string();
            page.tags.push("journal".into());
            page.finish();
        }

        let id = existing.get(&source_key).copied().unwrap_or_else(new_id);
        graph.add_page(&name, id);
        // **Also by its path.** Obsidian writes a link to a note in a subfolder either way —
        // `[[Roadmap]]` or `[[Projects/Roadmap]]` — and both are correct there, so registering
        // only the filename silently dangles every folder-qualified link in the vault.
        if let Some(rel) = source_key.strip_suffix(".md") {
            graph.add_page(rel, id);
        }
        for alias in &page.aliases {
            graph.add_page(alias, id);
        }
        for (uuid, text) in &page.block_texts {
            graph.blocks.insert(uuid.clone(), text.clone());
        }
        parsed.push(Parsed { id, source_key, dir: path.parent().map(Path::to_path_buf), page, updated: mtime_of(&path) });
    }

    // ── attachments: hashed once, before any rewriting ─────────────────────────
    let mut attachments = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for item in &parsed {
        // Both spellings reach here: `![](../assets/x.png)` from `attachments`, and `![[x.png]]`,
        // which the wikilink scanner recorded as a link because only the graph can tell them apart.
        let refs = item.page.attachments.iter().chain(item.page.links.iter());
        for reference in refs {
            if graph.page(reference).is_some() || graph.asset(reference).is_some() {
                continue;
            }
            let Some(path) = resolve_file(source, item.dir.as_deref(), reference, &file_index) else {
                continue;
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            match crate::ingest::ingest_file(vault, &path) {
                Ok(ing) => {
                    if ing.deduped {
                        report.attachments_deduped += 1;
                    } else {
                        report.attachments_added += 1;
                    }
                    graph.add_asset(reference, Asset { hash: ing.hash.clone(), filename: ing.filename.clone() });
                    if let Some(base) = path.file_name().and_then(|n| n.to_str()) {
                        graph.add_asset(base, Asset { hash: ing.hash.clone(), filename: ing.filename.clone() });
                    }
                    attachments.push(ing);
                }
                Err(e) => report.warnings.push(format!("{reference} could not be attached ({e})")),
            }
        }
    }

    // ── pass two: rewrite and build the notes ──────────────────────────────────
    let mut notes = Vec::new();
    let mut dangling: BTreeMap<String, usize> = BTreeMap::new();
    let mut stats = Rewritten::default();

    for item in &parsed {
        if existing.contains_key(&item.source_key) {
            // Skipped whole: the note already in the vault is the user's now, and it is still in
            // the graph above, so everyone else's links to it resolve.
            report.notes_already_imported += 1;
            continue;
        }
        let mut s = Rewritten::default();
        let body = rewrite(&item.page.body, &graph, &mut s);
        for name in &s.dangling {
            *dangling.entry(name.clone()).or_default() += 1;
        }
        stats.links_resolved += s.links_resolved;
        stats.blocks_inlined += s.blocks_inlined;
        stats.blocks_unresolved += s.blocks_unresolved;
        stats.assets_linked += s.assets_linked;

        notes.push(build(item, body, format, &mut report));
    }

    // ── stubs, only if asked ───────────────────────────────────────────────────
    if opts.create_stubs {
        for (name, _) in &dangling {
            if graph.page(name).is_some() {
                continue;
            }
            let mut obj = Object::new(Kind::Note, String::new());
            obj.title = Some(name.clone());
            obj.tags = vec!["imported-stub".into()];
            obj.extra.insert(SOURCE_LIBRARY.into(), PropertyValue::Text(format.as_str().into()));
            notes.push(obj);
            report.stubs_created += 1;
        }
    }

    report.notes_added = notes.len() - report.stubs_created;
    report.links_resolved = stats.links_resolved;
    report.blocks_inlined = stats.blocks_inlined;
    report.blocks_unresolved = stats.blocks_unresolved;
    report.dangling = dangling.values().sum();
    report.dangling_names = dangling.keys().take(12).cloned().collect();

    Ok(Converted { format, notes, attachments, report })
}

/// A fresh id, minted through the one place that mints them.
///
/// `Object::new` is fm-core's only route to a ULID — the `ulid` crate is fm-model's dependency,
/// not ours, and adding it here to save an allocation per page would widen what this crate links
/// for no benefit anyone can see.
fn new_id() -> Id {
    Object::new(Kind::Note, String::new()).id
}

/// One source file after parsing, before resolution.
struct Parsed {
    id: Id,
    source_key: String,
    dir: Option<PathBuf>,
    page: Page,
    updated: OffsetDateTime,
}

/// The [`Object`] a parsed page becomes. Built whole and written with **one** `put` — which is
/// both what keeps `created`/`updated` settable (`apply_property` refuses them) and what stops an
/// import becoming tens of thousands of round trips, each a file rewrite plus an index update.
fn build(item: &Parsed, body: String, format: Format, report: &mut Report) -> Object {
    let page = &item.page;
    let mut obj = Object::new(Kind::Note, body);
    obj.id = item.id;
    obj.title = Some(page.title.clone()).filter(|t| !t.trim().is_empty());
    obj.status = page.status.clone();
    obj.due = page.due;
    obj.start = page.start;
    obj.created = page.created.unwrap_or(item.updated);
    obj.updated = item.updated;
    obj.tags = page.tags.clone();

    for (key, value) in &page.props {
        // A source property may not take a name the note format already owns — see RESERVED_KEYS.
        let key = if RESERVED_KEYS.contains(&key.as_str()) {
            report.renamed_properties += 1;
            format!("{}_{key}", format.as_str())
        } else {
            key.clone()
        };
        obj.extra.insert(key, value.clone());
    }
    obj.extra.insert(SOURCE_KEY.into(), PropertyValue::Text(item.source_key.clone()));
    obj.extra.insert(SOURCE_LIBRARY.into(), PropertyValue::Text(format.as_str().into()));
    obj
}

/// A referenced file, found relative to the note, then to the graph root, then by bare name.
///
/// **Never outside the source folder.** A body is untrusted text — it arrives from whoever wrote
/// the graph — and `../../../.ssh/id_rsa` is a perfectly ordinary-looking Markdown image target.
fn resolve_file(
    source: &Path,
    dir: Option<&Path>,
    reference: &str,
    index: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    let reference = reference.trim();
    if reference.is_empty() || !is_local(reference) {
        return None;
    }
    let root = canonical(source);
    let mut tries: Vec<PathBuf> = Vec::new();
    if let Some(d) = dir {
        tries.push(d.join(reference));
    }
    tries.push(source.join(reference));
    if let Some(hit) = index.get(&name_key(reference)) {
        tries.push(hit.clone());
    }
    if let Some(base) = reference.rsplit('/').next() {
        if let Some(hit) = index.get(&name_key(base)) {
            tries.push(hit.clone());
        }
    }
    tries.into_iter().find(|p| {
        let c = canonical(p);
        c.starts_with(&root) && c.is_file()
    }).map(|p| canonical(&p))
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// When a file was **added**, for every file in the repo, in **one** `git log`.
///
/// Track V4's "use git for what only git knows", and the reason an imported note can carry the day
/// it was written rather than the day it was imported. One subprocess, not one per page: a spawn
/// per file is precisely the `papers-plan.md` B5 mistake (5,000 files × two spawns), and a graph
/// has as many files as that.
///
/// Best-effort throughout — no git, no repo, a shallow clone or a malformed answer all yield an
/// empty map and the caller falls back to the mtime. An import must never fail over history.
fn git_added_dates(root: &Path) -> HashMap<String, OffsetDateTime> {
    let mut out = HashMap::new();
    let Ok(res) = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--diff-filter=A", "--format=%aI", "--name-only", "--reverse"])
        .output()
    else {
        return out;
    };
    if !res.status.success() {
        return out;
    }
    let text = String::from_utf8_lossy(&res.stdout);
    let mut current: Option<OffsetDateTime> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        match OffsetDateTime::parse(line, &time::format_description::well_known::Rfc3339) {
            Ok(dt) => current = Some(dt),
            // `--reverse` walks oldest first, so the first date a path is seen under is the one
            // that added it; a later rename must not overwrite it.
            Err(_) => {
                if let Some(dt) = current {
                    out.entry(line.to_string()).or_insert(dt);
                }
            }
        }
    }
    out
}
