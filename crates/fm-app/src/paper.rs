//! **Turning what a researcher already has into a paper note — offline.**
//!
//! A paper in formicaria is an ordinary [`Kind::Note`](fm_model::Kind) carrying flat scalar
//! frontmatter, because `views.rs`'s `base_query` hardcodes `Kind(Note)` for board/agenda/timeline
//! — an asset note could never appear in a planning view — and because the paper, not the PDF, is
//! the thing you tag, schedule and think about. The PDF is a blob the note references.
//!
//! **Everything here is pure text → fields, with no network.** That is not a simplification, it is
//! the standing ruling: `fm-agent-run`'s `Cargo.toml` records that `ureq` and its TLS stack are
//! *"**agent-only** deps — the notes core links neither (the owner's ruling: the core stays
//! minimal, an agent may carry its own heavier deps)"*. So a DOI cannot be *resolved* here. What it
//! can do is recognise one, and read a citation the user already holds:
//!
//! - **The identifier in the PDF's own text.** `ingest` already runs `pdftotext`; most modern
//!   papers print their DOI or arXiv id on page one, so the commonest case costs nothing at all.
//! - **A pasted BibTeX entry.** Every publisher page and every reference manager exports it, and it
//!   carries the full record — title, authors, year, venue, DOI — with no lookup.
//! - **A pasted identifier or URL**, which is recorded as such.
//!
//! **Hand-rolled, no regex**, matching [`crate::refs`]: *"pure, no deps, no regex"*. The shapes here
//! are small and fixed, and a dependency for them would have to be argued against `deny.toml`.

/// A paper's identity, as printed on the paper or pasted by the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identifier {
    /// A DOI, normalised to the bare `10.…/…` form — never the `https://doi.org/` wrapper, so two
    /// spellings of one paper are one value and `Prop{Eq}` can match them.
    Doi(String),
    /// An arXiv id, bare (`2401.12345`, or the pre-2007 `math/0309136`), version suffix kept when
    /// the user gave one: `v2` is a different PDF and therefore a different blob.
    ArXiv(String),
    /// Anything else that is at least a URL — recorded rather than guessed at.
    Url(String),
}

impl Identifier {
    /// The frontmatter key this identifier belongs under.
    pub fn key(&self) -> &'static str {
        match self {
            Identifier::Doi(_) => "doi",
            Identifier::ArXiv(_) => "arxiv",
            Identifier::Url(_) => "url",
        }
    }
    pub fn value(&self) -> &str {
        match self {
            Identifier::Doi(v) | Identifier::ArXiv(v) | Identifier::Url(v) => v,
        }
    }
}

/// Trailing punctuation a DOI or id picks up from running text — a sentence's full stop, a closing
/// bracket from `(doi:…)`. A DOI may legitimately *contain* most of these, so they are only ever
/// trimmed from the end.
fn trim_trailing(s: &str) -> &str {
    s.trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '>', '"', '\''])
}

/// Is this the start of a DOI? `10.` then 4–9 digits then `/`.
fn doi_at(bytes: &[u8], i: usize) -> Option<usize> {
    if !bytes[i..].starts_with(b"10.") {
        return None;
    }
    let mut j = i + 3;
    let digits_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    let n = j - digits_start;
    if !(4..=9).contains(&n) || j >= bytes.len() || bytes[j] != b'/' {
        return None;
    }
    // The suffix runs to whitespace. DOIs are case-insensitive but case-preserving, so it is kept.
    let mut k = j + 1;
    while k < bytes.len() && !bytes[k].is_ascii_whitespace() {
        k += 1;
    }
    (k > j + 1).then_some(k)
}

/// The first DOI in `text`, if any.
pub fn find_doi(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if let Some(end) = doi_at(bytes, i) {
            let raw = trim_trailing(&text[i..end]);
            if raw.len() > 8 {
                return Some(raw.to_string());
            }
        }
    }
    None
}

/// The first arXiv id in `text`, if any — `arXiv:2401.12345v2` or a bare `2401.12345` after the
/// marker. Only ever *after* the marker: a bare `2401.12345` in running text is as likely to be a
/// figure number or a price as an identifier, and guessing wrong writes a false citation.
pub fn find_arxiv(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    while let Some(rel) = lower[from..].find("arxiv:") {
        let start = from + rel + "arxiv:".len();
        let rest = text[start..].trim_start();
        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '/')
            .collect();
        let id = trim_trailing(&id).to_string();
        // New style `NNNN.NNNNN` or old style `archive/NNNNNNN`.
        let new_style = id.split_once('.').is_some_and(|(a, b)| {
            a.len() == 4
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.trim_end_matches(|c: char| c.is_ascii_alphanumeric()).is_empty()
                && b.len() >= 4
        });
        let old_style = id.split_once('/').is_some_and(|(a, b)| {
            !a.is_empty()
                && a.bytes().all(|c| c.is_ascii_alphabetic())
                && b.len() >= 7
                && b.bytes().take(7).all(|c| c.is_ascii_digit())
        });
        if new_style || old_style {
            return Some(id);
        }
        from = start;
    }
    None
}

/// Recognise what the user pasted: a DOI, an arXiv id, or a URL. Accepts the wrapped spellings
/// (`https://doi.org/10.…`, `arxiv.org/abs/…`) and normalises them to the bare identifier, so the
/// same paper pasted two ways lands on one value.
pub fn parse_identifier(input: &str) -> Option<Identifier> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(doi) = find_doi(s) {
        return Some(Identifier::Doi(doi));
    }
    if let Some(id) = find_arxiv(s) {
        return Some(Identifier::ArXiv(id));
    }
    // `arxiv.org/abs/2401.12345` — the marker is the path, not an `arXiv:` prefix.
    let lower = s.to_ascii_lowercase();
    for marker in ["arxiv.org/abs/", "arxiv.org/pdf/"] {
        if let Some(rel) = lower.find(marker) {
            let rest = &s[rel + marker.len()..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '/')
                .collect();
            let id = id.trim_end_matches(".pdf").to_string();
            if !id.is_empty() {
                return Some(Identifier::ArXiv(id));
            }
        }
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(Identifier::Url(s.to_string()));
    }
    None
}

/// The identifier a PDF prints on itself. Only the front of the text is scanned: a DOI in the
/// *bibliography* belongs to somebody else's paper, and citing it as this one's is worse than
/// finding nothing.
pub fn identifier_in_text(text: &str) -> Option<Identifier> {
    const FRONT: usize = 4_000;
    // **Cut on a character, not a byte.** `FRONT` is a byte budget and `&str[..n]` panics when `n`
    // lands inside a multi-byte character — which `pdftotext` output does routinely, because real
    // papers carry ’ “ — ﬁ and accented names. This runs inside `asset_note` with the global
    // `Mutex` held and nothing catches unwinds, so the panic poisoned the lock and every later
    // command failed until the process restarted: one dropped PDF, one dead app.
    let mut end = text.len().min(FRONT);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let head = &text[..end];
    // arXiv first: an arXiv preprint's DOI, when it has one, is usually the publisher's rather
    // than the copy in front of you.
    find_arxiv(head)
        .map(Identifier::ArXiv)
        .or_else(|| find_doi(head).map(Identifier::Doi))
}

/// The flat, scalar fields a paper note carries. Every one is a `String`, because that is what
/// `apply_property` writes and what `fm-query` can group and filter on — and because
/// `PropertyValue` has no map variant, so nothing structured could live in frontmatter anyway.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PaperFields {
    pub title: Option<String>,
    /// Semicolon-separated, which is how a name containing a comma ("Vaswani, A.") stays one name.
    pub authors: Option<String>,
    pub year: Option<String>,
    pub venue: Option<String>,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub url: Option<String>,
    /// The BibTeX entry type (`article`, `inproceedings`, …), kept verbatim: it is the closest
    /// thing to Zotero's item type and throwing it away is unrecoverable.
    pub entry_type: Option<String>,
    /// The citation key, when the source had one — a stable handle a user already uses in LaTeX.
    pub cite_key: Option<String>,
}

impl PaperFields {
    pub fn is_empty(&self) -> bool {
        *self == PaperFields::default()
    }
    /// The frontmatter to write, in a fixed order so two identical papers produce identical files.
    pub fn properties(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for (k, v) in [
            ("authors", &self.authors),
            ("year", &self.year),
            ("venue", &self.venue),
            ("doi", &self.doi),
            ("arxiv", &self.arxiv),
            ("url", &self.url),
            ("entry_type", &self.entry_type),
            ("cite_key", &self.cite_key),
        ] {
            if let Some(v) = v.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                out.push((k, v.to_string()));
            }
        }
        out
    }
}

/// Strip one layer of `{}` or `""` and collapse whitespace — BibTeX wraps values, and braces also
/// appear *inside* a value to protect capitalisation (`{BERT}`), which no reader wants to see.
fn clean_value(v: &str) -> String {
    let v = v.trim();
    let v = v.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(v);
    let v = v.strip_prefix('"').and_then(|r| r.strip_suffix('"')).unwrap_or(v);
    v.chars().filter(|c| *c != '{' && *c != '}').collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse one BibTeX entry. Returns `None` for anything that is not one, so a caller can fall
/// through to treating the paste as an identifier or a title.
///
/// Deliberately forgiving about everything that does not change meaning — trailing commas, `=`
/// spacing, quoted or braced values, unknown fields — and deliberately not a full BibTeX
/// implementation: `@string` macros, concatenation and cross-references are not supported, and a
/// paste using them simply yields the fields it could read.
pub fn parse_bibtex(input: &str) -> Option<PaperFields> {
    let at = input.find('@')?;
    let rest = &input[at + 1..];
    let open = rest.find('{')?;
    let entry_type = rest[..open].trim().to_ascii_lowercase();
    if entry_type.is_empty() || !entry_type.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let body = &rest[open + 1..];
    // The citation key is everything up to the first comma.
    let (key, fields) = body.split_once(',')?;

    let mut out = PaperFields {
        entry_type: Some(entry_type),
        cite_key: Some(key.trim().to_string()).filter(|k| !k.is_empty()),
        ..Default::default()
    };

    // Walk `name = value` pairs, tracking brace depth so a comma inside `{…}` is not a separator.
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut parts: Vec<&str> = Vec::new();
    for (i, c) in fields.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    parts.push(&fields[start..i]);
                    start = fields.len();
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                parts.push(&fields[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < fields.len() {
        parts.push(&fields[start..]);
    }

    for part in parts {
        let Some((name, value)) = part.split_once('=') else { continue };
        let name = name.trim().to_ascii_lowercase();
        let value = clean_value(value);
        if value.is_empty() {
            continue;
        }
        match name.as_str() {
            "title" => out.title = Some(value),
            // BibTeX separates names with ` and `; we join with `; ` so a "Last, First" name,
            // which contains a comma, survives as one author.
            "author" => {
                out.authors = Some(
                    value.split(" and ").map(str::trim).filter(|s| !s.is_empty())
                        .collect::<Vec<_>>().join("; "),
                )
            }
            "year" | "date" => {
                // `date = {2017-06-12}` is the biblatex spelling; a paper's year is the year.
                out.year = Some(value.split(['-', '/']).next().unwrap_or(&value).to_string())
            }
            "journal" | "journaltitle" | "booktitle" | "publisher" | "school" => {
                out.venue.get_or_insert(value);
            }
            "doi" => out.doi = Some(value),
            "eprint" | "archiveprefix" if name == "eprint" => out.arxiv = Some(value),
            "url" => out.url = Some(value),
            _ => {}
        }
    }
    (out.title.is_some() || out.doi.is_some() || out.authors.is_some()).then_some(out)
}

/// A BibTeX entry for a paper note — the thing a user pastes into a manuscript.
///
/// Generated with `format!` and no dependency. `hayagriva` would give CSL and 2,600 styles and is
/// the obvious upgrade, but it is a dependency that owes its own `decisions.md` entry, and one
/// BibTeX entry does not need it.
pub fn to_bibtex(title: &str, props: &std::collections::BTreeMap<String, String>) -> String {
    let get = |k: &str| props.get(k).map(String::as_str).unwrap_or("").trim();
    let entry_type = match get("entry_type") {
        "" => "article",
        t => t,
    };
    let key = match get("cite_key") {
        "" => {
            // `firstauthorYEAR` — the convention every reference manager falls back to.
            let first = get("authors").split(';').next().unwrap_or("").trim().to_string();
            let surname: String = first
                .split(',')
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            let year = get("year");
            match (surname.is_empty(), year.is_empty()) {
                (true, true) => "paper".to_string(),
                (true, false) => format!("paper{year}"),
                (false, true) => surname,
                (false, false) => format!("{surname}{year}"),
            }
        }
        k => k.to_string(),
    };
    let mut out = format!("@{entry_type}{{{key},\n");
    if !title.is_empty() {
        out.push_str(&format!("  title = {{{title}}},\n"));
    }
    // Back to BibTeX's own ` and ` separator — the inverse of the parse above.
    let authors = get("authors");
    if !authors.is_empty() {
        let joined =
            authors.split(';').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" and ");
        out.push_str(&format!("  author = {{{joined}}},\n"));
    }
    // **`booktitle` for a proceedings, `journal` for a journal.** The parse maps both onto one
    // `venue` — a paper note has one place-it-appeared and does not need two — but the *emit* must
    // pick the field the entry type actually takes, or the entry is wrong in LaTeX. A round trip
    // through our own parser would never notice, since both map back to `venue`.
    let venue_field = match entry_type {
        "inproceedings" | "conference" | "incollection" => "booktitle",
        "phdthesis" | "mastersthesis" => "school",
        "book" | "techreport" | "manual" => "publisher",
        _ => "journal",
    };
    for (field, key) in [(venue_field, "venue"), ("year", "year"), ("doi", "doi"), ("url", "url")] {
        let v = get(key);
        if !v.is_empty() {
            out.push_str(&format!("  {field} = {{{v}}},\n"));
        }
    }
    let arxiv = get("arxiv");
    if !arxiv.is_empty() {
        out.push_str(&format!("  eprint = {{{arxiv}}},\n  archivePrefix = {{arXiv}},\n"));
    }
    // Trailing comma removed: valid either way, but every exporter omits it and a diff is quieter.
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push_str("}\n");
    out
}
