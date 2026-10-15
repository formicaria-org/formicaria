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
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub mod logseq;
pub mod obsidian;

#[cfg(test)]
mod tests;

/// Directories that are never a note: version control, the apps' own state, and the caches they
/// keep beside it. Walking into `.git` on a large graph is also the difference between a scan that
/// feels instant and one that does not.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".obsidian",
    ".trash",
    ".stfolder",
    "node_modules",
    "bak",
    "version-files",
    ".recycle",
];

/// Files an import has no landing site for. Counted and named in the report — *"left behind"* is a
/// thing the user should be told, not something to discover later.
const LEFT_BEHIND_EXT: &[&str] = &["org", "canvas", "edn"];

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
    /// On a Logseq highlights page, the PDF its `file::` property names, as written.
    pub highlight_file: Option<String>,
    pub warnings: Vec<String>,
}

impl Page {
    pub fn new(title: &str) -> Self {
        Page { title: title.to_string(), ..Default::default() }
    }

    /// Tidy what the parsers accumulated. Called once, by the parser, before handing the page on.
    pub fn finish(&mut self) {
        for v in [
            &mut self.tags,
            &mut self.aliases,
            &mut self.links,
            &mut self.block_refs,
            &mut self.attachments,
        ] {
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
    /// The attachments by kind: extension, count, bytes — largest first. What the person needs in
    /// front of them to choose what to leave out.
    pub kinds: Vec<(String, usize, u64)>,
    /// Excalidraw drawings, which become whiteboard notes.
    pub drawings: usize,
    /// Every page's path relative to the folder — the `source_key`s an import would write. How
    /// the preview recognises a graph already imported somewhere.
    pub keys: Vec<String>,
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

    let mut kinds: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    for path in &files {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "md" || ext == "excalidraw" {
            if let Ok(rel) = path.strip_prefix(source) {
                out.keys.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        if ext == "md" {
            if is_journal(source, path) {
                out.journals += 1;
            } else {
                out.pages += 1;
            }
        } else if ext == "excalidraw" {
            out.drawings += 1;
        } else if LEFT_BEHIND_EXT.contains(&ext.as_str()) {
            *left.entry(ext).or_default() += 1;
        } else {
            let len = path.metadata().map(|m| m.len()).unwrap_or(0);
            out.attachments += 1;
            out.attachment_bytes += len;
            let k = kinds.entry(if ext.is_empty() { "(none)".into() } else { ext }).or_default();
            k.0 += 1;
            k.1 += len;
        }
    }
    out.left_behind = left.into_iter().collect();
    out.kinds = kinds.into_iter().map(|(k, (n, b))| (k, n, b)).collect();
    out.kinds.sort_by_key(|k| std::cmp::Reverse(k.2));

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
///
/// **Every** percent-escape is decoded, not only `%2F`: Logseq escapes `:` `?` `"` `#` `\\` and
/// friends in a filename too, and decoding only the slash left 186 pages of one real graph titled
/// `Something%3A Else` and unreachable by the `[[Something: Else]]` that names them.
pub fn page_name(stem: &str) -> String {
    decode_url(&stem.replace("___", "/"))
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
            if let Some(end) = wikilink_end(s, i) {
                if let Some(name) = link_target(&s[i + 2..end]) {
                    out.push(name.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
}

/// Where the `]]` of a wikilink opened at `open` is, if it is one.
///
/// Its inside holds no `](` and no line break. That is what keeps `[[Draft]_notes.pdf](…)` — a
/// Markdown link whose *label* starts with a bracket — from being read as a wikilink that runs on
/// to the next `]]`, lines further down, taking everything between with it. A lone `[` or `]` is
/// allowed — a real page is called `… Notes on [Draft]Ideas` — and nested `[[…]]` balance.
fn wikilink_end(s: &str, open: usize) -> Option<usize> {
    // Nested links count: Logseq's `[[Contact #[[Ada]] about it]]` is one page name.
    let mut depth = 0usize;
    let mut j = open + 2;
    while j < s.len() {
        let rest = &s[j..];
        if rest.starts_with("[[") {
            depth += 1;
            j += 2;
        } else if rest.starts_with("]]") {
            if depth == 0 {
                return Some(j);
            }
            depth -= 1;
            j += 2;
        } else if rest.starts_with('\n') || rest.starts_with("](") {
            return None;
        } else {
            j += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    None
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
        if rest.starts_with("[[") {
            if let Some(end) = wikilink_end(s, i + 1) {
                let name = s[i + 3..end].trim();
                if !name.is_empty() {
                    out.push(name.replace(',', " ").trim().to_string());
                }
                i = end + 2;
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

/// Every local file a `![alt](path)` **or** `[label](path)` points at. A link to a PDF, a slide
/// deck or an archive is as much an attachment as an image is — collecting images alone left every
/// such link in a real graph pointing at a path that no longer exists. An `http(s)`/`data` target
/// is somebody else's URL and stays as it is; a `.md` target is a page, not a file.
pub(crate) fn scan_local_files(s: &str, out: &mut Vec<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // A real `[[wikilink]]` is a page, not a file — skip it whole. `[[Draft]_notes.pdf](…)`
        // is not one (see `wikilink_end`), so it falls through to be read as the link it is.
        if b[i] == b'[' && b.get(i + 1) == Some(&b'[') {
            if let Some(end) = wikilink_end(s, i) {
                i = end + 2;
                continue;
            }
        }
        if b[i] == b'[' {
            if let Some(span) = md_span(s, i) {
                let url = s[span.0..span.1].trim();
                let path = url.split('#').next().unwrap_or(url);
                if is_local(url) && !path.to_ascii_lowercase().ends_with(".md") {
                    out.push(decode_url(path));
                }
                i = span.2;
                continue;
            }
        }
        i += 1;
    }
}

/// Files a Logseq macro names: `{{zotero-imported-file KEY, "paper.pdf"}}` and the drawio
/// plugin's `{{renderer :drawio, 1696423408310.svg}}`, which keeps its drawings under
/// `assets/storages/logseq-drawio-plugin/`.
pub(crate) fn scan_macro_files(s: &str, out: &mut Vec<String>) {
    for (name, args) in macros(s) {
        if let Some(f) = macro_file(name, args) {
            out.push(f);
        }
    }
}

/// `{{name args}}` occurrences, in order.
fn macros(s: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut at = 0;
    std::iter::from_fn(move || {
        while at < s.len() {
            let start = at + s[at..].find("{{")?;
            match macro_end(s, start) {
                Some(end) => {
                    at = end + 2;
                    let inner = s[start + 2..end].trim();
                    return Some(inner.split_once(char::is_whitespace).unwrap_or((inner, "")));
                }
                None => at = start + 2,
            }
        }
        None
    })
}

/// Where the `}}` of a Logseq macro opened at `open` is, if one is. A macro is `{{name …}}` with a
/// **word** for a name, on **one line**, holding no other `{{`. Anything else is not a macro: LaTeX
/// is full of `{{` (`\frac{{a}}{b}`), and taking the next `}}` wherever it was swallowed whole
/// stretches of a note into one span.
fn macro_end(s: &str, open: usize) -> Option<usize> {
    let rest = s.get(open..)?.strip_prefix("{{")?;
    let name = rest.trim_start();
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let len = rest.find("}}")?;
    let inner = &rest[..len];
    (!inner.contains('\n') && !inner.contains("{{")).then_some(open + 2 + len)
}

fn macro_file(name: &str, args: &str) -> Option<String> {
    match name {
        "zotero-imported-file" => args.split('"').nth(1).map(str::to_string),
        "renderer" => {
            let mut parts = args.split(',').map(str::trim);
            (parts.next() == Some(":drawio"))
                .then(|| parts.next())
                .flatten()
                .map(|f| format!("assets/storages/logseq-drawio-plugin/{f}"))
        }
        _ => None,
    }
}

/// The target of the first Markdown link in `s` — what a `file:: [x.pdf](../assets/x.pdf)`
/// property points at.
pub(crate) fn first_link_target(s: &str) -> Option<&str> {
    let open = s.find('[')?;
    let (us, ue, _) = md_span(s, open)?;
    Some(s[us..ue].trim())
}

/// `(url_start, url_end, one_past_close)` for a `[label](url)` whose `[` is at `open`.
///
/// **Balanced**, both halves. A label may hold brackets (`[[Draft]_notes.pdf]`) and a path may
/// hold parentheses (`Photo (1).jpeg`) — common in a real graph — and stopping at the
/// first closing character cut both short. A backslash escapes the character after it, and a line
/// break ends the attempt: a link does not span lines.
fn md_span(s: &str, open: usize) -> Option<(usize, usize, usize)> {
    let b = s.as_bytes();
    let balanced = |from: usize, o: u8, c: u8| -> Option<usize> {
        let mut depth = 0i32;
        let mut j = from;
        while j < b.len() {
            match b[j] {
                b'\\' => j += 1,
                b'\n' => return None,
                x if x == o => depth += 1,
                x if x == c => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j);
                    }
                }
                _ => {}
            }
            j += 1;
        }
        None
    };
    let close = balanced(open, b'[', b']')?;
    if b.get(close + 1) != Some(&b'(') {
        return None;
    }
    let paren = balanced(close + 1, b'(', b')')?;
    Some((close + 2, paren, paren + 1))
}

/// A target in this graph rather than somewhere else: no URL scheme (`https:`, `zotero:`,
/// `mailto:`, …), not a fragment, not a `((block ref))`.
fn is_local(url: &str) -> bool {
    !url.is_empty() && !has_scheme(url) && !url.starts_with('#') && !url.starts_with("((")
}

fn has_scheme(url: &str) -> bool {
    url.find(':').is_some_and(|at| {
        let scheme = &url[..at];
        scheme.len() >= 2
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
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
    /// Which note a block lives on — what a `[label](((uuid)))` links to.
    block_pages: HashMap<String, Id>,
    assets: HashMap<String, Asset>,
    /// Files the person's attachment rules left out, and why — so the note can say so.
    left_out: HashMap<String, String>,
    /// How deep [`inline_block`] is: a block's text may reference another block.
    depth: std::cell::Cell<u8>,
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
    pub fn add_block(&mut self, uuid: &str, text: &str, page: Id) {
        let key = uuid.to_ascii_lowercase();
        self.blocks.insert(key.clone(), text.to_string());
        self.block_pages.insert(key, page);
    }
    pub fn block_page(&self, uuid: &str) -> Option<Id> {
        self.block_pages.get(&uuid.trim().to_ascii_lowercase()).copied()
    }
    pub fn leave_out(&mut self, reference: &str, why: &str) {
        self.left_out.insert(name_key(reference), why.to_string());
    }
    pub fn left_out(&self, reference: &str) -> Option<&str> {
        self.left_out
            .get(&name_key(reference))
            .or_else(|| self.left_out.get(&name_key(reference.rsplit('/').next()?)))
            .map(String::as_str)
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
    /// Pages linked to that have no file, as written — each becomes a tag on the note
    /// (`decisions.md#data`, 2026-10-15).
    pub tags: Vec<String>,
}

/// Turn one body's foreign syntax into ours. **Pure** — attachments were hashed before this ran.
pub fn rewrite(body: &str, graph: &Graph, out_stats: &mut Rewritten) -> String {
    let b = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;

    while i < b.len() {
        // Code and math are copied through untouched: nothing inside them is Logseq syntax, and
        // LaTeX's `{{a}}` read as a macro was mangling formulas.
        if let Some(end) = verbatim_end(body, i) {
            out.push_str(&body[i..end]);
            i = end;
            continue;
        }

        // A bracketed hashtag is a tag and is already in the frontmatter. In the text it reads as
        // Logseq shows it — `#two words`, not `#[[two words]]`, whose brackets were left on screen
        // in 705 places of one real graph. Handled before the link arm, which would otherwise
        // claim it.
        if b[i] == b'#' && body[i + 1..].starts_with("[[") {
            if let Some(end) = wikilink_end(body, i + 1) {
                // A tag's name may hold another (`#[[Meeting with #[[Ada]]]]`): its brackets go too.
                let name = body[i + 3..end].replace("#[[", "#").replace("[[", "").replace("]]", "");
                out.push('#');
                out.push_str(&name);
                i = end + 2;
                continue;
            }
        }

        // `{{name args}}` — an embed, a video, a Zotero file, a drawing, a query. **Nothing leaves
        // here with its braces**: a macro this app cannot run is shown as what it said, not as
        // syntax.
        if let Some(end) = macro_end(body, i) {
            out.push_str(&macro_of(body[i + 2..end].trim(), graph, out_stats));
            i = end + 2;
            continue;
        }

        // `![[Target]]` — an embed of a page, or of a file.
        if body[i..].starts_with("![[") {
            if let Some(end) = wikilink_end(body, i + 1) {
                let inner = &body[i + 3..end];
                if let Some(text) = embed_target(inner, graph, out_stats) {
                    out.push_str(&text);
                    i = end + 2;
                    continue;
                }
            }
        }

        // `[[Target]]` / `[[Target|alias]]`
        if body[i..].starts_with("[[") {
            if let Some(end) = wikilink_end(body, i) {
                let inner = &body[i + 2..end];
                if let Some(text) = link_of(inner, graph, out_stats) {
                    out.push_str(&text);
                    i = end + 2;
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
                        out.push_str(&inline_block(text, graph, out_stats));
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
                    // `{:height 530, :width 748}` straight after an image is Logseq's sizing, which
                    // the read view does by itself. Left in, it is text under every resized image.
                    if is_img && body[i..].starts_with("{:") {
                        if let Some(close) = body[i..].find('}') {
                            i += close + 1;
                        }
                    }
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

/// The end of a span starting at `i` that must be copied as it is: a backslash escape, a code
/// span or fence, display math `$$…$$`, or inline math `$…$`. The read view's `extractMath`
/// (`ui/src/lib/render.ts`) rules, **narrowed to one line for the inline forms**: a Logseq body is
/// an outline, so a span that runs on past its bullet is a stray delimiter, not a formula.
fn verbatim_end(s: &str, i: usize) -> Option<usize> {
    let b = s.as_bytes();
    match b[i] {
        b'\\' => Some((i + 2).min(s.len())).filter(|&e| s.is_char_boundary(e)).or(Some(i + 1)),
        b'`' => {
            let run = b[i..].iter().take_while(|&&c| c == b'`').count();
            let fence = &s[i..i + run];
            // A fence (three or more) may span lines; an inline code span closes on its own line —
            // in an outline every bullet is a line, and one stray backtick matched to the next one
            // a page later took everything between out of the conversion.
            let scope = if run >= 3 { &s[i + run..] } else { s[i + run..].split('\n').next()? };
            Some(scope.find(fence).map_or(i + run, |at| i + run + at + run))
        }
        b'$' if b.get(i + 1) == Some(&b'$') => s[i + 2..].find("$$").map(|at| i + 2 + at + 2),
        b'$' => {
            let next = *b.get(i + 1)?;
            if next.is_ascii_whitespace() || next.is_ascii_digit() {
                return None;
            }
            let mut j = i + 1;
            while j < b.len() {
                match b[j] {
                    // Inline math closes on its own line, for the same reason as inline code: a
                    // shell `$(find …)` in a note is not a formula running to the next `$`.
                    b'\n' => return None,
                    b'$' if b.get(j + 1) == Some(&b'$') => return None,
                    b'$' if !b[j - 1].is_ascii_whitespace()
                        && !b.get(j + 1).is_some_and(u8::is_ascii_digit) =>
                    {
                        return Some(j + 1);
                    }
                    _ => {}
                }
                j += 1;
            }
            None
        }
        _ => None,
    }
}

/// A referenced block's words, **converted like any other text** — they were captured from the
/// source as written, links, images and further references included, and pasted raw they put
/// Logseq syntax back into the note. A reference to a reference is followed a few levels deep and
/// then shown as plain words, so blocks that reference each other cannot loop.
fn inline_block(text: &str, graph: &Graph, s: &mut Rewritten) -> String {
    const MAX_DEPTH: u8 = 4;
    let depth = graph.depth.get();
    if depth >= MAX_DEPTH {
        return text.to_string();
    }
    graph.depth.set(depth + 1);
    let out = rewrite(text, graph, s);
    graph.depth.set(depth);
    out
}

/// What one `{{…}}` macro becomes. Always *something without braces*.
fn macro_of(inner: &str, graph: &Graph, s: &mut Rewritten) -> String {
    let (name, args) = inner.split_once(char::is_whitespace).unwrap_or((inner, ""));
    let args = args.trim();
    match name {
        "embed" => {
            if let Some(rest) = args.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
                return embed_target(rest, graph, s)
                    .unwrap_or_else(|| dangling(rest, graph, s).unwrap_or_default());
            }
            let uuid = args.trim_start_matches("((").trim_end_matches("))").trim();
            match graph.block(uuid) {
                Some(text) => {
                    s.blocks_inlined += 1;
                    inline_block(text, graph, s)
                }
                None => {
                    s.blocks_unresolved += 1;
                    "⟨referenced block not imported⟩".into()
                }
            }
        }
        // A video is a link a reader can follow. Not an embed: the app frames nothing from
        // another site (`frame-src`).
        "video" | "youtube" if has_scheme(args) => {
            let label = if args.contains("youtube.com") || args.contains("youtu.be") {
                "YouTube video"
            } else {
                "video"
            };
            format!("[{label}]({args})")
        }
        "zotero-imported-file" => {
            let file = args.split('"').nth(1).unwrap_or(args);
            match graph.asset(file) {
                Some(a) => {
                    s.assets_linked += 1;
                    format!("[{}](asset:sha256-{})", escape_label(file), a.hash)
                }
                None => match graph.left_out(file) {
                    Some(why) => format!("{file} ({why})"),
                    None => format!("{file} (a Zotero attachment, kept in Zotero)"),
                },
            }
        }
        "renderer" if macro_file(name, args).is_some() => {
            let path = macro_file(name, args).unwrap_or_default();
            let file = path.rsplit('/').next().unwrap_or(&path).to_string();
            match graph.asset(&path) {
                Some(a) => {
                    s.assets_linked += 1;
                    format!("![{}](asset:sha256-{})", escape_label(&file), a.hash)
                }
                None => format!("drawing {file} (not in the graph)"),
            }
        }
        // A query, or any macro this app has no meaning for: shown as what it said, as code, so
        // it reads as a thing that was in the note rather than as a sentence.
        _ => {
            let text = if args.is_empty() { name.to_string() } else { format!("{name} {args}") };
            format!("`{}`", text.replace('`', "'"))
        }
    }
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
    if let Some(why) = graph.left_out(target) {
        return Some(format!("{target} ({why})"));
    }
    // An embed of a *file* that is not there names the file; anything else is a page with no file.
    if looks_like_file(target) {
        s.dangling.push(target.to_string());
        return Some(format!("{target} (not in the graph)"));
    }
    dangling(target, graph, s)
}

/// A link to a page that has no file: counted, and — unless it names a day — made a **tag** on
/// the note, keeping its words in the text without the brackets (`decisions.md#data`,
/// 2026-10-15). In Logseq such a link is how a person groups everything by an author or a kind;
/// a tag is that grouping with no empty note behind it.
fn dangling(target: &str, _graph: &Graph, s: &mut Rewritten) -> Option<String> {
    s.dangling.push(target.to_string());
    if !looks_like_date(target) {
        s.tags.push(target.to_string());
    }
    Some(target.to_string())
}

/// `photo.png`, `deck.pdf` — a name with a short extension.
fn looks_like_file(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty()
            && (1..=5).contains(&ext.len())
            && ext.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// A journal day by any of its usual names: `Oct 14th, 2026`, `2026-10-14`, `14 Oct 2026`, … — a
/// four-digit year with a month name or with numbers around it. A tag per day would be noise.
fn looks_like_date(name: &str) -> bool {
    const MONTHS: [&str; 12] =
        ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let lower = name.to_ascii_lowercase();
    let has_year = lower
        .split(|c: char| !c.is_ascii_digit())
        .any(|n| n.len() == 4 && n.starts_with(['1', '2']));
    if !has_year || name.len() > 32 {
        return false;
    }
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    let month = words.iter().any(|w| w.len() >= 3 && MONTHS.iter().any(|m| w.starts_with(m)));
    let numeric = lower.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '_' | '/' | '.'));
    month || numeric
}

fn link_of(inner: &str, graph: &Graph, s: &mut Rewritten) -> Option<String> {
    // **The whole name first.** In Logseq `|` is part of a page name (`Paper | Publisher Reader
    // View`); only Obsidian reads it as `[[page|shown]]`.
    if let Some(id) = graph.page(inner.trim()) {
        s.links_resolved += 1;
        return Some(format!("[{}]({})", escape_label(inner.trim()), fm_model::note_ref(id)));
    }
    let target = link_target(inner)?;
    // `[[Page|shown]]` — the alias is what the reader sees, so it is the label.
    let label =
        inner.split_once('|').map(|(_, a)| a.trim()).filter(|a| !a.is_empty()).unwrap_or(target);
    match graph.page(target) {
        Some(id) => {
            s.links_resolved += 1;
            Some(format!("[{}]({})", escape_label(label), fm_model::note_ref(id)))
        }
        // No note is invented for it — that would put an empty card in every view.
        None => dangling(target, graph, s).map(|_| label.to_string()),
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
    // `[see this](((uuid)))` — a link to a block: to the note that holds it.
    if let Some(uuid) = url.strip_prefix("((").and_then(|r| r.strip_suffix("))")) {
        let text = if label.is_empty() { graph.block(uuid).unwrap_or("") } else { label };
        return Some(match graph.block_page(uuid) {
            Some(id) => {
                s.links_resolved += 1;
                format!("[{}]({})", escape_label(text), fm_model::note_ref(id))
            }
            None => text.to_string(),
        });
    }
    if !is_local(url) {
        return None;
    }
    // `…/deck.pdf#page=3`: the file is the part before the fragment; the fragment rides along.
    let (path, fragment) = match url.find('#') {
        Some(at) => (&url[..at], &url[at..]),
        None => (url, ""),
    };
    let decoded = decode_url(path);
    if let Some(asset) = graph.asset(&decoded) {
        s.assets_linked += 1;
        let label = if label.is_empty() { &asset.filename } else { label };
        return Some(format!(
            "{}[{}](asset:sha256-{}{fragment})",
            if is_img { "!" } else { "" },
            escape_label(label),
            asset.hash
        ));
    }
    let shown = || -> String {
        if label.is_empty() {
            decoded.rsplit('/').next().unwrap_or(&decoded).to_string()
        } else {
            label.to_string()
        }
    };
    if let Some(why) = graph.left_out(&decoded) {
        let file = decoded.rsplit('/').next().unwrap_or(&decoded);
        return Some(if shown() == file {
            format!("{file} ({why})")
        } else {
            format!("{} ({file}, {why})", shown())
        });
    }
    // `[text](Some Note.md)` — Obsidian's other way of spelling a link.
    if let Some(stem) = decoded.strip_suffix(".md") {
        let stem = stem.rsplit('/').next().unwrap_or(stem);
        return Some(match graph.page(&page_name(stem)) {
            Some(id) => {
                s.links_resolved += 1;
                format!("[{}]({})", escape_label(label), fm_model::note_ref(id))
            }
            None => shown(),
        });
    }
    // A file the note points at that is not in the graph. Its name stays; a dead path does not.
    let file = decoded.rsplit('/').next().unwrap_or(&decoded);
    Some(if shown() == file {
        format!("{file} (not in the graph)")
    } else {
        format!("{} ({file}, not in the graph)", shown())
    })
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
    "schema",
    "id",
    "type",
    "title",
    "status",
    "start",
    "due",
    "hard",
    "created",
    "updated",
    "tags",
    "assets",
    "code",
    "vault",
    "thread_of",
    "reply_to",
    "proposes",
    "base",
    // A logged entry is hidden the same way; an imported `log:` must not take a note with it.
    "log",
    // A whiteboard is a note whose `view` is `board`; an imported `view::` must not make one.
    "view",
    SOURCE_KEY,
    SOURCE_LIBRARY,
    IMPORT_HASH,
];

/// How deep a source tree may nest before we stop. A graph is a handful of levels; anything past
/// this is a link loop or a mistyped path like `/`, and `scan` is asked on a keystroke.
const MAX_DEPTH: usize = 24;

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Create an empty note for a page that is only ever linked to. Off by default: those notes
    /// are `Kind::Note`, so `thread::notes_base` shows them in **every** board, agenda, timeline
    /// and feed — and nothing in the app is virtualised.
    pub create_stubs: bool,
    /// Re-emit pages already imported into this vault, under their old ids, so the caller can
    /// **replace** each one the person has not edited (`decisions.md#data`, 2026-10-15). Off, an
    /// import only adds.
    pub replace: bool,
    /// Which attachments come over. The default brings every one.
    pub attachments: AttachmentRules,
}

/// The person's rules for attachments (`decisions.md#data`, 2026-10-15). Empty = bring them all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachmentRules {
    /// Leave out a file larger than this many bytes.
    pub max_bytes: Option<u64>,
    /// Leave out files with these extensions — lower-case, no dot.
    pub skip_kinds: Vec<String>,
}

impl AttachmentRules {
    /// `None` when `path` comes over; otherwise the words a note says about why it did not.
    pub fn refuses(&self, path: &Path) -> Option<String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !ext.is_empty() && self.skip_kinds.iter().any(|k| k.eq_ignore_ascii_case(&ext)) {
            return Some(format!("left out of the import: .{ext} files were not brought over"));
        }
        let len = path.metadata().map(|m| m.len()).unwrap_or(0);
        match self.max_bytes {
            Some(max) if len > max => Some(format!(
                "left out of the import: {}, over the {} limit",
                size_label(len),
                size_label(max)
            )),
            _ => None,
        }
    }
}

/// A size as a person reads it: `740 KB`, `3.2 MB`, `1.4 GB`. Decimal units, as the import form
/// asks for its limit in MB.
pub fn size_label(bytes: u64) -> String {
    match bytes {
        b if b < 1000 => format!("{b} bytes"),
        b if b < 1_000_000 => format!("{} KB", b.div_ceil(1000)),
        b if b < 10_000_000 => format!("{:.1} MB", b as f64 / 1e6),
        b if b < 1_000_000_000 => format!("{} MB", (b as f64 / 1e6).round()),
        b => format!("{:.1} GB", b as f64 / 1e9),
    }
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
    /// Attachments the person's rules left out, and a sample of their names.
    pub attachments_left_out: usize,
    pub left_out_names: Vec<String>,
    /// Pages linked to that have no file and became tags.
    pub tags_from_links: usize,
    /// Excalidraw drawings that became whiteboard notes.
    pub drawings: usize,
    /// Set by the writer on a replace: earlier notes rewritten, and earlier notes kept because
    /// the person had edited (or deleted) them since — with a sample of their titles.
    pub notes_replaced: usize,
    pub notes_kept_edited: usize,
    pub kept_edited_names: Vec<String>,
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
    /// Ids in `notes` that are an earlier import's notes, re-emitted for [`Options::replace`]. The
    /// writer replaces each only if [`unedited`] says so.
    pub replacing: HashSet<Id>,
}

/// The key under which an imported note carries the hash of what the import wrote.
pub const IMPORT_HASH: &str = "import_hash";

/// The hash of everything about a note a person can edit: title, status, dates, tags, body and
/// custom properties (as their labels, so a value that comes back from YAML as a number hashes the
/// same as the text it was written from). `created`/`updated` are not in it; the
/// [`IMPORT_HASH`] itself is not in it.
pub fn content_hash(o: &Object) -> String {
    let mut s = String::new();
    let mut field = |t: &str| {
        s.push_str(t);
        s.push('\u{1f}');
    };
    field(o.title.as_deref().unwrap_or(""));
    field(o.status.as_deref().unwrap_or(""));
    field(&o.due.map(|d| d.to_string()).unwrap_or_default());
    field(&o.start.map(|d| d.to_string()).unwrap_or_default());
    field(&o.tags.join("\u{1e}"));
    field(o.body.trim());
    for (k, v) in &o.extra {
        if k != IMPORT_HASH {
            field(k);
            field(&v.display());
        }
    }
    crate::blob::sha256_hex(s.as_bytes())
}

/// Whether `current` — an earlier import's note, as it is now — is still exactly what that import
/// wrote, so a replace may overwrite it. `fresh` is the note this import would write in its place.
///
/// A note carrying an [`IMPORT_HASH`] is unedited iff it still hashes to it. One from before the
/// hash existed counts as unedited only if its `updated` is still the source file's modification
/// time — which the import stamped, and which any save in the app moves. That errs one way only:
/// a doubtful note is **kept**.
pub fn unedited(current: &Object, fresh: &Object) -> bool {
    match current.extra.get(IMPORT_HASH) {
        Some(PropertyValue::Text(h)) => content_hash(current) == *h,
        _ => current.updated.unix_timestamp() == fresh.updated.unix_timestamp(),
    }
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
    let format = detect(source).ok_or_else(|| {
        StoreError::Io("that folder is not a Logseq graph or an Obsidian vault".into())
    })?;

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
    walk(source, &mut files)
        .map_err(|e| StoreError::Io(format!("could not read that folder: {e}")))?;

    let mut report = Report { format: format.as_str().to_string(), ..Default::default() };

    // Split the walk once: Markdown is a page, a drawing is a whiteboard, everything else is a
    // candidate attachment.
    let ext_of =
        |p: &Path| p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let (markdown, others): (Vec<_>, Vec<_>) = files.into_iter().partition(|p| ext_of(p) == "md");

    let mut left: BTreeMap<String, usize> = BTreeMap::new();
    let mut file_index: HashMap<String, PathBuf> = HashMap::new();
    let mut drawings: Vec<PathBuf> = Vec::new();
    for path in &others {
        let ext = ext_of(path);
        if ext == "excalidraw" {
            drawings.push(path.clone());
            continue;
        }
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

    // ── pass one: parse, mint ids, build the graph ─────────────────────────────
    let git_dates = git_added_dates(source);
    let journal_format = match format {
        Format::Logseq => logseq_journal_format(source),
        Format::Obsidian => String::new(),
    };
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
                report
                    .warnings
                    .push(format!("{source_key} could not be read ({e}) and was skipped"));
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

        let id = existing.get(&source_key).copied().unwrap_or_else(new_id);
        graph.add_page(&name, id);
        // **And by the title it shows.** A `title::` names the page as surely as its filename —
        // Logseq writes `[[That Title]]`, not the escaped stem.
        graph.add_page(&page.title, id);
        if let Some(day) = journal {
            // `2026_09_02` is a filename, not a title. The day it stands for is what a person
            // reading a list of notes needs to see.
            page.title = day.date().to_string();
            page.tags.push("journal".into());
            page.finish();
            // A journal answers to the day as Logseq titles it — `[[Oct 14th, 2026]]` — which is
            // how every link to a day is written.
            graph.add_page(&page.title, id);
            if !journal_format.is_empty() {
                graph.add_page(&format_journal(day.date(), &journal_format), id);
            }
        }
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
            graph.add_block(uuid, text, id);
        }
        parsed.push(Parsed {
            id,
            source_key,
            dir: path.parent().map(Path::to_path_buf),
            page,
            updated: mtime_of(&path),
        });
    }

    // ── attachments: hashed once, before any rewriting ─────────────────────────
    let mut attachments = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut left_out: BTreeSet<String> = BTreeSet::new();
    for item in &parsed {
        // Every spelling reaches here: `![](../assets/x.png)` and `[x.pdf](../assets/x.pdf)` from
        // `attachments`, files a macro names, and `![[x.png]]`, which the wikilink scanner recorded
        // as a link because only the graph can tell a page from a file.
        let refs = item.page.attachments.iter().chain(item.page.links.iter());
        for reference in refs {
            if graph.page(reference).is_some()
                || graph.asset(reference).is_some()
                || graph.left_out(reference).is_some()
            {
                continue;
            }
            let Some(path) = resolve_file(source, item.dir.as_deref(), reference, &file_index)
            else {
                continue;
            };
            let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            // The person's rules. A file left out is still *named* where the note mentions it.
            if let Some(why) = opts.attachments.refuses(&path) {
                graph.leave_out(reference, &why);
                graph.leave_out(&base, &why);
                if left_out.insert(base.clone()) {
                    report.attachments_left_out += 1;
                }
                continue;
            }
            if !seen.insert(path.clone()) {
                continue;
            }
            // **Copied, always.** A vault holds every byte it owns (`decisions.md#vault`,
            // 2026-10-15): no link, no pointer, nothing whose bytes another folder can change.
            match crate::ingest::ingest_file(vault, &path) {
                Ok(ing) => {
                    if ing.deduped {
                        report.attachments_deduped += 1;
                    } else {
                        report.attachments_added += 1;
                    }
                    let asset = Asset { hash: ing.hash.clone(), filename: ing.filename.clone() };
                    graph.add_asset(reference, asset.clone());
                    graph.add_asset(&base, asset.clone());
                    if let Ok(rel) = path.strip_prefix(&src_c) {
                        graph.add_asset(&rel.to_string_lossy(), asset);
                    }
                    attachments.push(ing);
                }
                Err(e) => report.warnings.push(format!("{reference} could not be attached ({e})")),
            }
        }
    }
    report.left_out_names = left_out.into_iter().take(12).collect();

    // ── pass two: rewrite and build the notes ──────────────────────────────────
    let mut notes = Vec::new();
    let mut replacing = HashSet::new();
    let mut dangling: BTreeMap<String, usize> = BTreeMap::new();
    let mut stats = Rewritten::default();

    for item in &parsed {
        let earlier = existing.contains_key(&item.source_key);
        if earlier && !opts.replace {
            // Skipped whole: the note already in the vault is the user's now, and it is still in
            // the graph above, so everyone else's links to it resolve.
            report.notes_already_imported += 1;
            continue;
        }
        let mut s = Rewritten::default();
        let body = rewrite(&item.page.body, &graph, &mut s);
        // A page property holding a link or a file is resolved like the body: `file::` brings
        // its PDF, `item-type:: [[journalArticle]]` its tag.
        let props: BTreeMap<String, PropertyValue> = item
            .page
            .props
            .iter()
            .map(|(k, v)| (k.clone(), rewrite_value(v, &graph, &mut s)))
            .collect();
        for name in &s.dangling {
            *dangling.entry(name.clone()).or_default() += 1;
        }
        stats.links_resolved += s.links_resolved;
        stats.blocks_inlined += s.blocks_inlined;
        stats.blocks_unresolved += s.blocks_unresolved;
        stats.assets_linked += s.assets_linked;
        report.tags_from_links += s.tags.len();

        let note = build(item, body, props, &s.tags, format, &mut report);
        if earlier {
            replacing.insert(note.id);
        }
        notes.push(note);
    }

    // ── drawings: an Excalidraw scene is a whiteboard note ─────────────────────
    for path in &drawings {
        let Ok(rel) = path.strip_prefix(source) else { continue };
        let source_key = rel.to_string_lossy().replace('\\', "/");
        let earlier = existing.get(&source_key).copied();
        if earlier.is_some() && !opts.replace {
            report.notes_already_imported += 1;
            continue;
        }
        let scene = match std::fs::read_to_string(path) {
            Ok(t)
                if serde_json::from_str::<serde_json::Value>(&t)
                    .is_ok_and(|v| v.get("elements").is_some_and(|e| e.is_array())) =>
            {
                t
            }
            // Not a scene we can draw: say so, as any other file with no landing site.
            _ => {
                *left.entry("excalidraw".into()).or_default() += 1;
                continue;
            }
        };
        let mut obj = Object::new(Kind::Note, scene);
        if let Some(id) = earlier {
            obj.id = id;
            replacing.insert(id);
        }
        obj.title = path.file_stem().and_then(|s| s.to_str()).map(str::to_string);
        obj.created = git_dates.get(&source_key).copied().unwrap_or_else(|| mtime_of(path));
        obj.updated = mtime_of(path);
        obj.extra.insert("view".into(), PropertyValue::Text("board".into()));
        obj.extra.insert(SOURCE_KEY.into(), PropertyValue::Text(source_key));
        obj.extra.insert(SOURCE_LIBRARY.into(), PropertyValue::Text(format.as_str().into()));
        let hash = content_hash(&obj);
        obj.extra.insert(IMPORT_HASH.into(), PropertyValue::Text(hash));
        notes.push(obj);
        report.drawings += 1;
    }
    report.left_behind = left.into_iter().collect();

    // ── stubs, only if asked ───────────────────────────────────────────────────
    if opts.create_stubs {
        for name in dangling.keys() {
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

    report.notes_added = notes.len() - report.stubs_created - replacing.len();
    report.links_resolved = stats.links_resolved;
    report.blocks_inlined = stats.blocks_inlined;
    report.blocks_unresolved = stats.blocks_unresolved;
    report.dangling = dangling.values().sum();
    report.dangling_names = dangling.keys().take(12).cloned().collect();

    Ok(Converted { format, notes, attachments, report, replacing })
}

/// A property value after pass two: text is rewritten like a body, a list item by item.
fn rewrite_value(v: &PropertyValue, graph: &Graph, s: &mut Rewritten) -> PropertyValue {
    match v {
        PropertyValue::Text(t) => PropertyValue::Text(rewrite(t, graph, s)),
        PropertyValue::List(items) => {
            PropertyValue::List(items.iter().map(|i| rewrite_value(i, graph, s)).collect())
        }
        other => other.clone(),
    }
}

/// Logseq's `:journal/page-title-format`, or its default. A commented-out line is not a setting.
fn logseq_journal_format(source: &Path) -> String {
    let cfg = std::fs::read_to_string(source.join("logseq/config.edn")).unwrap_or_default();
    cfg.lines()
        .filter(|l| !l.trim_start().starts_with(';'))
        .find_map(|l| l.split(":journal/page-title-format").nth(1)?.split('"').nth(1))
        .unwrap_or("MMM do, yyyy")
        .to_string()
}

/// A day in a Logseq journal title format — the date-fns tokens those formats use.
pub fn format_journal(d: time::Date, fmt: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const DAYS: [&str; 7] =
        ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let month = MONTHS[d.month() as usize - 1];
    let weekday = DAYS[d.weekday().number_days_from_monday() as usize];
    let day = d.day();
    let suffix = match (day % 10, day % 100) {
        (1, n) if n != 11 => "st",
        (2, n) if n != 12 => "nd",
        (3, n) if n != 13 => "rd",
        _ => "th",
    };
    // Longest token first, so `MMMM` is never read as `MMM` and a stray `M`.
    let tokens: [(&str, String); 10] = [
        ("yyyy", format!("{:04}", d.year())),
        ("MMMM", month.to_string()),
        ("MMM", month[..3].to_string()),
        ("MM", format!("{:02}", d.month() as u8)),
        ("EEEE", weekday.to_string()),
        ("EEE", weekday[..3].to_string()),
        ("do", format!("{day}{suffix}")),
        ("dd", format!("{day:02}")),
        ("d", day.to_string()),
        ("M", (d.month() as u8).to_string()),
    ];
    let mut out = String::new();
    let mut rest = fmt;
    'next: while let Some(c) = rest.chars().next() {
        for (token, value) in &tokens {
            if let Some(r) = rest.strip_prefix(token) {
                out.push_str(value);
                rest = r;
                continue 'next;
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
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
fn build(
    item: &Parsed,
    body: String,
    props: BTreeMap<String, PropertyValue>,
    link_tags: &[String],
    format: Format,
    report: &mut Report,
) -> Object {
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
    // Pages linked to with no file of their own, as tags — once each, whatever the case.
    for t in link_tags {
        if !obj.tags.iter().any(|have| have.eq_ignore_ascii_case(t)) {
            obj.tags.push(t.clone());
        }
    }

    for (key, value) in props {
        // A source property may not take a name the note format already owns — see RESERVED_KEYS.
        let key = if RESERVED_KEYS.contains(&key.as_str()) {
            report.renamed_properties += 1;
            format!("{}_{key}", format.as_str())
        } else {
            key
        };
        obj.extra.insert(key, value);
    }
    obj.extra.insert(SOURCE_KEY.into(), PropertyValue::Text(item.source_key.clone()));
    obj.extra.insert(SOURCE_LIBRARY.into(), PropertyValue::Text(format.as_str().into()));
    // Last, over everything else: what this import wrote, so a later *replace* can tell an
    // untouched note from one the person edited.
    let hash = content_hash(&obj);
    obj.extra.insert(IMPORT_HASH.into(), PropertyValue::Text(hash));
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
    tries
        .into_iter()
        .find(|p| {
            let c = canonical(p);
            c.starts_with(&root) && c.is_file()
        })
        .map(|p| canonical(&p))
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
