//! **The oracle**: what a Logseq graph contains, read **without** the importer.
//!
//! Every scanner here is written separately from `fm_core::import`, on purpose: an oracle that
//! called `scan_wikilinks` would inherit that function's blind spots and report them as 100%. The
//! algorithms differ too: brackets and parentheses are *balanced* here (a label like
//! `[Draft]_notes.pdf` or a path like `shot(1).png` is common in a real graph), where the importer
//! stops at the first closing character. Where the two disagree, the report says so, and the
//! question of which one is right is settled by what Logseq itself displays.
//!
//! Nothing here writes to the source.

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use time::Date;

/// One source file under `pages/` or `journals/`.
#[derive(Debug, Default)]
pub struct SourcePage {
    /// Path relative to the graph root, `/`-separated — the importer's `source_key`.
    pub key: String,
    pub journal: Option<Date>,
    /// The title a reader should see on the imported note.
    pub title: String,
    /// `[[target]]` and `[label](target.md)` occurrences, as the target page's name.
    pub links: Vec<String>,
    /// `((uuid))` references, including the `[label](((uuid)))` spelling.
    pub block_refs: usize,
    pub attachments: Vec<Attachment>,
    /// `hl-page:: N` blocks, and the PDF the page's `file::` names.
    pub highlights: Vec<u32>,
    pub highlight_file: Option<PathBuf>,
    /// `{{name …}}` macro names, in order.
    pub macros: Vec<String>,
    /// The words a reader could search for: prose, minus markup, URLs and bookkeeping.
    pub words: BTreeSet<String>,
}

/// A file a page points at.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// As written in the page.
    pub written: String,
    /// The file it resolves to inside the graph, or `None` when it is not there (a Zotero
    /// attachment, for one, lives in Zotero's own storage).
    pub file: Option<PathBuf>,
}

impl Attachment {
    /// The name a reader would recognise the file by.
    pub fn basename(&self) -> String {
        let decoded = percent_decode(&self.written);
        decoded.rsplit('/').next().unwrap_or(&decoded).to_string()
    }
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub root: PathBuf,
    pub pages: Vec<SourcePage>,
    /// Lower-cased name → index into `pages`. A page answers to its decoded filename, its
    /// `title::`, each `alias::`, and (a journal) its display title.
    pub names: HashMap<String, usize>,
    /// `draws/*.excalidraw`, relative to the root.
    pub drawings: Vec<String>,
    /// The journal title format the graph is configured with.
    pub journal_format: String,
}

impl Inventory {
    pub fn page_named(&self, name: &str) -> Option<&SourcePage> {
        self.names.get(&key(name)).map(|&i| &self.pages[i])
    }
}

/// Logseq's default `:journal/page-title-format`.
const DEFAULT_JOURNAL_FORMAT: &str = "MMM do, yyyy";

/// Block properties that are Logseq's own bookkeeping, not anything a person wrote.
const BOOKKEEPING: &[&str] =
    &["id", "collapsed", "heading", "ls-type", "hl-page", "hl-color", "hl-stamp", "hl-type"];

/// Task markers and priority cookies — syntax, not words.
const MARKERS: &[&str] =
    &["TODO", "DOING", "NOW", "LATER", "WAITING", "IN-PROGRESS", "DONE", "CANCELED", "CANCELLED"];

pub fn inventory(root: &Path) -> Inventory {
    let journal_format = journal_format(root);
    let mut inv = Inventory { root: root.to_path_buf(), journal_format, ..Default::default() };

    for dir in ["pages", "journals"] {
        let mut files = Vec::new();
        collect(&root.join(dir), &mut files);
        files.sort();
        for path in files {
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let page = read_page(root, &path, dir == "journals", &text);
            inv.pages.push(page);
        }
    }

    // Names last, once every page is known. First writer wins, as Logseq's own is unspecified.
    for (i, p) in inv.pages.iter().enumerate() {
        let stem = Path::new(&p.key).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let mut names = vec![decode_page_name(stem), p.title.clone()];
        if let Some(d) = p.journal {
            names.push(format_journal(d, &inv.journal_format));
        }
        names.extend(page_property_list(
            &std::fs::read_to_string(inv.root.join(&p.key)).unwrap_or_default(),
            "alias",
        ));
        for n in names {
            inv.names.entry(key(&n)).or_insert(i);
        }
    }

    let mut draws = Vec::new();
    collect(&root.join("draws"), &mut draws);
    inv.drawings = draws
        .iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("excalidraw"))
        .filter_map(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    inv.drawings.sort();
    inv
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else {
            out.push(p);
        }
    }
}

pub fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// `Parent___Child` → `Parent/Child`, then every `%XX` decoded — Logseq escapes `:`, `?`, `/` and
/// friends in a filename, and the page's real name has them.
pub fn decode_page_name(stem: &str) -> String {
    percent_decode(&stem.replace("___", "/"))
}

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ── the journal title ───────────────────────────────────────────────────────────

fn journal_format(root: &Path) -> String {
    let cfg = std::fs::read_to_string(root.join("logseq/config.edn")).unwrap_or_default();
    for line in cfg.lines() {
        let t = line.trim_start();
        if t.starts_with(';') {
            continue;
        }
        if let Some(rest) = t.split(":journal/page-title-format").nth(1) {
            if let Some(v) = rest.split('"').nth(1) {
                return v.to_string();
            }
        }
    }
    DEFAULT_JOURNAL_FORMAT.to_string()
}

/// The subset of date-fns tokens Logseq's journal formats use.
pub fn format_journal(d: Date, fmt: &str) -> String {
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
    let ordinal = match (day % 10, day % 100) {
        (1, n) if n != 11 => "st",
        (2, n) if n != 12 => "nd",
        (3, n) if n != 13 => "rd",
        _ => "th",
    };
    // Longest token first, so `MMMM` is not read as `MMM` + `M`.
    let tokens: [(&str, String); 10] = [
        ("yyyy", format!("{:04}", d.year())),
        ("MMMM", month.to_string()),
        ("MMM", month[..3].to_string()),
        ("MM", format!("{:02}", d.month() as u8)),
        ("EEEE", weekday.to_string()),
        ("EEE", weekday[..3].to_string()),
        ("do", format!("{day}{ordinal}")),
        ("dd", format!("{day:02}")),
        ("d", day.to_string()),
        ("M", (d.month() as u8).to_string()),
    ];
    let mut out = String::new();
    let mut rest = fmt;
    'outer: while !rest.is_empty() {
        for (tok, val) in &tokens {
            if let Some(r) = rest.strip_prefix(tok) {
                out.push_str(val);
                rest = r;
                continue 'outer;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

fn journal_date(stem: &str) -> Option<Date> {
    let parts: Vec<&str> = stem.split(['_', '-']).collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u8 = parts[1].parse().ok()?;
    let d: u8 = parts[2].parse().ok()?;
    Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
}

// ── one page ────────────────────────────────────────────────────────────────────

/// `key:: value`, with or without a leading bullet. The key holds no whitespace.
fn property(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    let t = t.strip_prefix("- ").unwrap_or(t);
    let (k, v) = t.split_once("::")?;
    if k.is_empty() || k.contains(char::is_whitespace) {
        return None;
    }
    Some((k, v.trim()))
}

/// The leading run of property lines — the page's own properties.
fn page_properties(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match property(line) {
            Some((k, v)) => out.push((k.to_string(), v.to_string())),
            None => break,
        }
    }
    out
}

fn page_property_list(text: &str, name: &str) -> Vec<String> {
    page_properties(text)
        .into_iter()
        .filter(|(k, _)| k == name)
        .flat_map(|(_, v)| {
            v.split(',')
                .map(|s| {
                    s.trim().trim_start_matches("[[").trim_end_matches("]]").trim().to_string()
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn read_page(root: &Path, path: &Path, in_journals: bool, text: &str) -> SourcePage {
    let key_path = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let journal = if in_journals { journal_date(stem) } else { None };
    let props = page_properties(text);
    let n_props = {
        // How many leading lines the page properties occupy, blank lines included.
        let mut n = 0;
        for line in text.lines() {
            if line.trim().is_empty() || property(line).is_some() {
                n += 1;
            } else {
                break;
            }
        }
        n
    };

    let title = match journal {
        // The importer's ruling: a journal is titled by the ISO day it stands for.
        Some(d) => d.to_string(),
        None => props
            .iter()
            .find(|(k, _)| k == "title")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| decode_page_name(stem)),
    };

    let dir = path.parent().unwrap_or(root);
    let mut page = SourcePage { key: key_path, journal, title, ..Default::default() };

    let mut in_fence = false;
    let mut in_logbook = false;
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        let content = t.strip_prefix("- ").unwrap_or(t);
        if content.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            words_into(content, &mut page.words);
            continue;
        }
        if t == ":LOGBOOK:" {
            in_logbook = true;
            continue;
        }
        if in_logbook {
            in_logbook = t != ":END:";
            continue;
        }

        if let Some((k, v)) = property(line) {
            if i < n_props {
                // A page property. `tags`/`alias` become fields, not links; `file`/`file-path`
                // name the PDF a highlights page annotates.
                match k {
                    "tags" | "alias" => {
                        words_into(&v.replace("[[", " ").replace("]]", " "), &mut page.words);
                        continue;
                    }
                    "file-path" => {
                        page.highlight_file = resolve(root, dir, v);
                        continue;
                    }
                    "title" => {
                        words_into(v, &mut page.words);
                        continue;
                    }
                    _ => {}
                }
            } else if k == "hl-page" {
                if let Ok(n) = v.parse() {
                    page.highlights.push(n);
                }
                continue;
            }
            if BOOKKEEPING.contains(&k) || k.starts_with("logseq.") {
                continue;
            }
            scan_inline(root, dir, v, &mut page);
            continue;
        }

        // A task marker and a priority cookie at the head of a bullet are syntax.
        let mut body = content;
        for m in MARKERS {
            if let Some(r) = body.strip_prefix(m) {
                if r.is_empty() || r.starts_with(' ') {
                    body = r.trim_start();
                    break;
                }
            }
        }
        for p in ["[#A]", "[#B]", "[#C]"] {
            if let Some(r) = body.strip_prefix(p) {
                body = r.trim_start();
            }
        }
        scan_inline(root, dir, body, &mut page);
    }
    page
}

/// Scan one line of prose: links, attachments, block refs, macros — and the words left over.
fn scan_inline(root: &Path, dir: &Path, s: &str, page: &mut SourcePage) {
    let mut prose = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Inline code: its words count, nothing inside it is markup.
        if b[i] == b'`' {
            if let Some(end) = s[i + 1..].find('`') {
                prose.push(' ');
                prose.push_str(&s[i + 1..i + 1 + end]);
                prose.push(' ');
                i += end + 2;
                continue;
            }
        }
        // `{{name args}}` — a word for a name, on one line. LaTeX's `{{` is not a macro.
        if s[i..].starts_with("{{")
            && s[i + 2..].trim_start().starts_with(|c: char| c.is_ascii_alphabetic())
        {
            if let Some(end) = find_balanced(s, i, "{{", "}}") {
                let inner = &s[i + 2..end];
                macro_of(root, inner, page);
                i = end + 2;
                prose.push(' ');
                continue;
            }
        }
        // `((uuid))`
        if s[i..].starts_with("((") {
            if let Some(end) = s[i + 2..].find("))") {
                if is_uuid(&s[i + 2..i + 2 + end]) {
                    page.block_refs += 1;
                    i += end + 4;
                    prose.push(' ');
                    continue;
                }
            }
        }
        // `#[[tag]]` — a tag, its words count.
        if s[i..].starts_with("#[[") {
            if let Some(end) = find_balanced(s, i + 1, "[[", "]]") {
                prose.push(' ');
                prose.push_str(&s[i + 3..end]);
                prose.push(' ');
                i = end + 2;
                continue;
            }
        }
        // `[[page]]`, `![[page]]`
        if s[i..].starts_with("[[") {
            if let Some(end) = find_balanced(s, i, "[[", "]]") {
                let inner = &s[i + 2..end];
                page.links.push(inner.trim().to_string());
                prose.push(' ');
                prose.push_str(inner);
                prose.push(' ');
                i = end + 2;
                continue;
            }
        }
        // `[label](target)` / `![alt](target)` — brackets and parentheses balanced.
        if b[i] == b'[' {
            if let Some(close) = find_balanced(s, i, "[", "]") {
                if s[close + 1..].starts_with('(') {
                    if let Some(paren) = find_balanced(s, close + 1, "(", ")") {
                        let label = &s[i + 1..close];
                        link_of(root, dir, s[close + 2..paren].trim(), page);
                        prose.push(' ');
                        prose.push_str(label);
                        prose.push(' ');
                        i = paren + 1;
                        // `{:height 100, :width 200}` straight after an image is sizing.
                        if s[i..].starts_with("{:") {
                            if let Some(e) = s[i..].find('}') {
                                i += e + 1;
                            }
                        }
                        continue;
                    }
                }
            }
        }
        // A bare URL is not prose either.
        if s[i..].starts_with("http://") || s[i..].starts_with("https://") {
            let end = s[i..].find(char::is_whitespace).map_or(s.len(), |e| i + e);
            i = end;
            prose.push(' ');
            continue;
        }
        let c = s[i..].chars().next().unwrap();
        prose.push(c);
        i += c.len_utf8();
    }
    words_into(&prose, &mut page.words);
}

/// Index of the `close` that balances the `open` at `start`, counting nesting.
fn find_balanced(s: &str, start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = start;
    while i < s.len() {
        if s[i..].starts_with(open) {
            depth += 1;
            i += open.len();
        } else if s[i..].starts_with(close) {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += close.len();
        } else {
            i += s[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    None
}

fn macro_of(root: &Path, inner: &str, page: &mut SourcePage) {
    let inner = inner.trim();
    let (name, args) = inner.split_once(char::is_whitespace).unwrap_or((inner, ""));
    page.macros.push(name.to_string());
    match name {
        "embed" => {
            let a = args.trim();
            if let Some(n) = a.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
                page.links.push(n.trim().to_string());
            } else if a.starts_with("((") {
                page.block_refs += 1;
            }
        }
        "zotero-imported-file" => {
            if let Some(name) = args.split('"').nth(1) {
                let file = root.join("assets").join(name);
                page.attachments.push(Attachment {
                    written: name.to_string(),
                    file: file.is_file().then_some(file),
                });
                words_into(name, &mut page.words);
            }
        }
        "renderer" => {
            // `{{renderer :drawio, 1696423408310.svg}}` — the drawio plugin keeps its drawings here.
            let mut parts = args.split(',').map(str::trim);
            if parts.next() == Some(":drawio") {
                if let Some(f) = parts.next() {
                    let file = root.join("assets/storages/logseq-drawio-plugin").join(f);
                    page.attachments.push(Attachment {
                        written: f.to_string(),
                        file: file.is_file().then_some(file),
                    });
                }
            }
        }
        _ => {}
    }
}

fn link_of(root: &Path, dir: &Path, target: &str, page: &mut SourcePage) {
    let target = target.trim_start_matches('<').trim_end_matches('>');
    // `[label](((uuid)))` — a link to a block.
    if let Some(u) = target.strip_prefix("((").and_then(|r| r.strip_suffix("))")) {
        if is_uuid(u) {
            page.block_refs += 1;
        }
        return;
    }
    if target.is_empty() || target.starts_with('#') || has_scheme(target) {
        return;
    }
    let decoded = percent_decode(target);
    if let Some(stem) = decoded.strip_suffix(".md") {
        let stem = stem.rsplit('/').next().unwrap_or(stem);
        page.links.push(decode_page_name(stem));
        return;
    }
    page.attachments
        .push(Attachment { written: target.to_string(), file: resolve(root, dir, target) });
}

fn has_scheme(t: &str) -> bool {
    match t.find(':') {
        Some(at) => {
            let scheme = &t[..at];
            !scheme.is_empty()
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
                && !t[..at].contains('/')
        }
        None => false,
    }
}

/// A local target, relative to the page's directory, resolved inside the graph.
fn resolve(root: &Path, dir: &Path, target: &str) -> Option<PathBuf> {
    let decoded = percent_decode(target.trim());
    let mut candidates = vec![dir.join(&decoded), root.join(&decoded)];
    // Logseq writes `../../../../assets/x` from a page several namespaces deep, and resolves it
    // as the graph's own `assets/` however many `..` there are.
    if let Some(at) = decoded.rfind("assets/") {
        candidates.push(root.join(&decoded[at..]));
    }
    for c in candidates {
        let n = normalise(&c);
        if n.starts_with(root) && n.is_file() {
            return Some(n);
        }
    }
    None
}

fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Words a person could search for: alphanumeric runs of four or more, lower-cased, not
/// all digits.
pub fn words_into(s: &str, out: &mut BTreeSet<String>) {
    for w in s.split(|c: char| !c.is_alphanumeric()) {
        if w.chars().count() >= 4 && !w.chars().all(|c| c.is_ascii_digit()) {
            out.insert(w.to_lowercase());
        }
    }
}
