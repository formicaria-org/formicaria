//! **Import conformance**: does a Logseq graph survive `run_import`?
//!
//! The plan is `docs/context/logseq-import-plan-2026-10-14.md`. In short: [`oracle`] reads the
//! source graph *without* the importer, the real `run_import` command imports it into a fresh
//! vault, and each check compares the two. Each check is a number against a bar, so "does the
//! port work" has a yes/no answer — and the numbers say how far off a no is.
//!
//! Shared by two tests in `import_corpus.rs`: the public mini-graph (in `pixi run ci`, ratcheted)
//! and the owner's private graph (`pixi run import-corpus`, never in the repo).

#![allow(dead_code)]

pub mod oracle;

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::{BlobStore, FileStore, MultiStore, Store};
use fm_model::Object;
use fm_query::Filter;
use oracle::Inventory;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

/// How many failures a check keeps as examples. Enough to see a pattern, not the whole list.
const SAMPLES: usize = 20;

/// One measured property of the import.
#[derive(Debug)]
pub struct Check {
    pub id: &'static str,
    pub what: &'static str,
    pub ok: usize,
    pub total: usize,
    /// The fraction `ok / total` must reach. `None` = reported, not judged.
    pub bar: Option<f64>,
    pub failures: Vec<String>,
}

impl Check {
    fn new(id: &'static str, what: &'static str, bar: Option<f64>) -> Self {
        Check { id, what, ok: 0, total: 0, bar, failures: Vec::new() }
    }
    fn record(&mut self, pass: bool, failure: impl FnOnce() -> String) {
        self.total += 1;
        if pass {
            self.ok += 1;
        } else if self.failures.len() < SAMPLES {
            self.failures.push(failure());
        }
    }
    pub fn ratio(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.ok as f64 / self.total as f64
        }
    }
    pub fn passed(&self) -> bool {
        self.bar.is_none_or(|b| self.ratio() >= b)
    }
}

pub struct Scorecard {
    pub checks: Vec<Check>,
    pub report: Value,
    pub seconds: f64,
}

impl Scorecard {
    pub fn failing(&self) -> Vec<&'static str> {
        self.checks.iter().filter(|c| !c.passed()).map(|c| c.id).collect()
    }

    pub fn to_markdown(&self) -> String {
        let mut s = String::from("# Import conformance scorecard\n\n");
        s.push_str(&format!("Import took {:.1} s.\n\n", self.seconds));
        s.push_str("| Check | What | Result | Bar | |\n|---|---|---|---|---|\n");
        for c in &self.checks {
            let bar = c.bar.map_or("reported".to_string(), |b| format!("{:.1}%", b * 100.0));
            let mark = match (c.bar, c.passed()) {
                (None, _) => "·",
                (_, true) => "✅",
                (_, false) => "❌",
            };
            s.push_str(&format!(
                "| {} | {} | {}/{} ({:.1}%) | {} | {} |\n",
                c.id,
                c.what,
                c.ok,
                c.total,
                c.ratio() * 100.0,
                bar,
                mark
            ));
        }
        for c in self.checks.iter().filter(|c| !c.failures.is_empty()) {
            s.push_str(&format!("\n## {} — {}\n\n", c.id, c.what));
            for f in &c.failures {
                s.push_str(&format!("- {}\n", f.replace('\n', " ⏎ ")));
            }
        }
        s.push_str("\n## The importer's own report\n\n```json\n");
        s.push_str(&serde_json::to_string_pretty(&self.report).unwrap_or_default());
        s.push_str("\n```\n");
        s
    }

    pub fn to_json(&self) -> Value {
        json!({
            "seconds": self.seconds,
            "failing": self.failing(),
            "checks": self.checks.iter().map(|c| json!({
                "id": c.id, "what": c.what, "ok": c.ok, "total": c.total,
                "bar": c.bar, "passed": c.passed(), "failures": c.failures,
            })).collect::<Vec<_>>(),
            "report": self.report,
        })
    }

    /// Write `scorecard.json` and `report.md` into `dir`.
    pub fn write(&self, dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("scorecard.json"),
            serde_json::to_vec_pretty(&self.to_json()).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("report.md"), self.to_markdown()).unwrap();
    }
}

// ── running the import ──────────────────────────────────────────────────────────

pub fn call(app: &App, cmd: &str, args: Value) -> Result<Value, String> {
    let out = dispatch(cmd, &args, &[], app, &NoHost)?;
    Ok(serde_json::from_slice(&out.into_bytes()).unwrap_or(Value::Null))
}

/// Size and mtime of every file under `root` — enough to see a write, without re-reading
/// gigabytes of attachments.
fn fingerprint(root: &Path) -> BTreeMap<PathBuf, (u64, Option<SystemTime>)> {
    fn walk(d: &Path, out: &mut BTreeMap<PathBuf, (u64, Option<SystemTime>)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
            if m.is_dir() {
                walk(&p, out);
            } else {
                out.insert(p, (m.len(), m.modified().ok()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
}

/// Import `source` into a new vault under `work`, twice, and measure the result.
///
/// `work` must not exist or be empty; it holds the vault and the app's config.
pub fn run(source: &Path, work: &Path) -> Scorecard {
    let inv = oracle::inventory(source);
    let before = fingerprint(source);

    let vault = work.join("vault");
    let app = new_app(work);

    let started = Instant::now();
    let first = call(
        &app,
        "run_import",
        json!({ "source": source.to_string_lossy(), "vault": "", "name": "imported",
                "path": vault.to_string_lossy() }),
    )
    .unwrap_or_else(|e| panic!("the import failed: {e}"));
    let seconds = started.elapsed().as_secs_f64();
    let second = call(
        &app,
        "run_import",
        json!({ "source": source.to_string_lossy(), "vault": "imported", "name": "",
                "path": "" }),
    )
    .unwrap_or_else(|e| panic!("the second import failed: {e}"));
    // And a third, replacing: nothing was edited in between, so every note is rewritten.
    let third = call(
        &app,
        "run_import",
        json!({ "source": source.to_string_lossy(), "vault": "imported", "name": "",
                "path": "", "replace": true }),
    )
    .unwrap_or_else(|e| panic!("the replacing import failed: {e}"));
    let previous = call(
        &app,
        "check_import",
        json!({ "source": source.to_string_lossy(), "vault": "", "name": "again",
                "path": work.join("again").to_string_lossy() }),
    )
    .unwrap_or(Value::Null);
    drop(app);

    let after = fingerprint(source);
    let store = FileStore::open(&vault).unwrap();
    let notes: Vec<Object> = store.candidates(&Filter::new()).unwrap().0;
    let blobs = BlobStore::new(&vault);

    let mut checks = vec![c1_every_page_is_one_note(&inv, &notes), c2_titles(&inv, &notes)];
    let by_key = by_source_key(&notes);
    let ids: HashMap<&str, String> =
        by_key.iter().map(|(k, o)| (*k, fm_model::note_ref(o.id))).collect();
    checks.push(c3_links_reach_their_page(&inv, &by_key, &ids));
    checks.push(c4_implicit_pages(&inv));
    let (c5, c5b, c6) = c5_c6_attachments(&inv, &by_key, &blobs);
    checks.extend([c5, c5b, c6]);
    checks.push(c7_no_debris(&notes));
    checks.push(c8_text_survives(&inv, &by_key));
    checks.push(c9_findable(&inv, &by_key, &store));
    checks.push(c10_source_untouched(&before, &after));
    checks.push(c11_second_import_adds_nothing(&second));
    checks.push(c11b_one_asset_note_per_file(&notes));
    checks.push(c14_replace_rewrites_what_was_not_edited(&third, &inv));
    checks.push(c15_an_earlier_import_is_recognised(&previous));
    checks.push(c13_drawings(&inv, &notes));

    Scorecard { checks, report: first, seconds }
}

fn source_key(o: &Object) -> Option<&str> {
    match o.extra.get(fm_core::import::SOURCE_KEY) {
        Some(fm_model::PropertyValue::Text(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn by_source_key(notes: &[Object]) -> HashMap<&str, &Object> {
    notes.iter().filter_map(|o| source_key(o).map(|k| (k, o))).collect()
}

/// Everything a reader sees of a note, as one string: title, tags, body, and every custom field.
fn visible_text(o: &Object) -> String {
    let mut s = String::new();
    s.push_str(o.title.as_deref().unwrap_or(""));
    s.push('\n');
    s.push_str(&o.tags.join("\n"));
    s.push('\n');
    s.push_str(&o.body);
    for (k, v) in &o.extra {
        s.push('\n');
        s.push_str(k);
        s.push(' ');
        s.push_str(&v.display());
    }
    s
}

/// An app with an empty vault list whose config lives in `work`, so nothing touches the real one.
pub fn new_app(work: &Path) -> App {
    std::fs::create_dir_all(work).unwrap();
    App::new(
        MultiStore::open(&[] as &[(String, PathBuf)]).unwrap(),
        Vec::<VaultConfig>::new(),
        Some(work.join("vaults.json")),
        true,
    )
}

/// An app that already knows one vault — what a restart of the app would open.
pub fn reopen_app(name: &str, vault: &Path) -> App {
    App::new(
        MultiStore::open(&[(name.to_string(), vault.to_path_buf())]).unwrap(),
        vec![VaultConfig { name: name.into(), path: vault.to_path_buf(), restic: None }],
        None,
        false,
    )
}

// ── the checks ──────────────────────────────────────────────────────────────────

fn c1_every_page_is_one_note(inv: &Inventory, notes: &[Object]) -> Check {
    let mut c =
        Check::new("C1", "every source page is exactly one note, journals on their day", Some(1.0));
    let mut count: HashMap<&str, usize> = HashMap::new();
    for o in notes {
        if let Some(k) = source_key(o) {
            *count.entry(k).or_default() += 1;
        }
    }
    let by_key = by_source_key(notes);
    for p in &inv.pages {
        let n = count.get(p.key.as_str()).copied().unwrap_or(0);
        let dated = match (p.journal, by_key.get(p.key.as_str())) {
            (Some(day), Some(o)) => o.created.date() == day,
            _ => true,
        };
        c.record(n == 1 && dated, || {
            if n != 1 {
                format!("{} → {n} notes", p.key)
            } else {
                format!("{} is not dated on its day", p.key)
            }
        });
    }
    c
}

fn c2_titles(inv: &Inventory, notes: &[Object]) -> Check {
    let mut c = Check::new("C2", "each note's title is the name Logseq shows", Some(1.0));
    let by_key = by_source_key(notes);
    for p in &inv.pages {
        let Some(o) = by_key.get(p.key.as_str()) else { continue };
        let got = o.title.as_deref().unwrap_or("");
        c.record(got == p.title, || format!("{}: expected {:?}, got {:?}", p.key, p.title, got));
    }
    c
}

/// A link whose target page exists must become a `note:` reference to *that* note. Counted per
/// occurrence: a page linking the same target twice needs two references.
fn c3_links_reach_their_page(
    inv: &Inventory,
    by_key: &HashMap<&str, &Object>,
    ids: &HashMap<&str, String>,
) -> Check {
    let mut c = Check::new("C3", "every link to an existing page reaches that note", Some(1.0));
    for p in &inv.pages {
        let Some(o) = by_key.get(p.key.as_str()) else { continue };
        let text = visible_text(o);
        let mut want: BTreeMap<&str, usize> = BTreeMap::new();
        for l in &p.links {
            if let Some(t) = inv.page_named(l) {
                *want.entry(t.key.as_str()).or_default() += 1;
            }
        }
        for (target, n) in want {
            let have = ids.get(target).map_or(0, |r| text.matches(&format!("({r})")).count());
            for i in 0..n {
                c.record(i < have, || format!("{} → {target} ({have} of {n} linked)", p.key));
            }
        }
    }
    c
}

/// Links to pages that exist only as a reference — authors, property values, section names.
/// Reported, not judged: what they should become is an open question for the owner.
fn c4_implicit_pages(inv: &Inventory) -> Check {
    let mut c = Check::new("C4", "links to implicit pages (no file) — reported", None);
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for p in &inv.pages {
        for l in &p.links {
            let exists = inv.page_named(l).is_some();
            if !exists {
                *freq.entry(oracle::key(l)).or_default() += 1;
            }
            c.record(exists, String::new);
        }
    }
    let mut top: Vec<_> = freq.into_iter().collect();
    top.sort_by_key(|t| std::cmp::Reverse(t.1));
    c.failures = top.into_iter().take(SAMPLES).map(|(n, k)| format!("{k}× [[{n}]]")).collect();
    c
}

fn c5_c6_attachments(
    inv: &Inventory,
    by_key: &HashMap<&str, &Object>,
    blobs: &BlobStore,
) -> (Check, Check, Check) {
    let mut c5 =
        Check::new("C5", "every attachment in the graph is a blob the note references", Some(1.0));
    let mut c5b = Check::new("C5b", "every PDF highlight links to its page of the PDF", Some(1.0));
    let mut c6 =
        Check::new("C6", "an attachment not in the graph is still named in the note", Some(1.0));
    let mut hashes: HashMap<PathBuf, String> = HashMap::new();
    let mut hash = |p: &Path| -> String {
        hashes
            .entry(p.to_path_buf())
            .or_insert_with(|| fm_core::blob::sha256_file(p).unwrap_or_default())
            .clone()
    };
    for p in &inv.pages {
        let Some(o) = by_key.get(p.key.as_str()) else { continue };
        let text = visible_text(o);
        for a in &p.attachments {
            match &a.file {
                Some(f) => {
                    let h = hash(f);
                    let ok = text.contains(&format!("asset:sha256-{h}")) && blobs.exists(&h);
                    c5.record(ok, || format!("{}: {}", p.key, a.written));
                }
                None => {
                    let name = a.basename();
                    c6.record(text.contains(&name), || format!("{}: {name}", p.key));
                }
            }
        }
        if let Some(pdf) = &p.highlight_file {
            let h = hash(pdf);
            for n in &p.highlights {
                c5b.record(text.contains(&format!("asset:sha256-{h}#page={n}")), || {
                    format!("{}: page {n}", p.key)
                });
            }
        }
    }
    (c5, c5b, c6)
}

/// Source syntax that must never be visible in an imported note. Fenced code is exempt — what is
/// written inside a fence is shown as written.
const DEBRIS: &[(&str, &str)] = &[
    ("](../assets/", "a link still pointing into the old graph"),
    ("{:height", "image sizing left as text"),
    ("{:width", "image sizing left as text"),
    ("{{", "a Logseq macro left as text"),
    ("((", "a raw block reference"),
    ("#[[", "a tag still in its brackets"),
];

fn c7_no_debris(notes: &[Object]) -> Check {
    let mut c = Check::new("C7", "no Logseq syntax left visible in a note", Some(1.0));
    for o in notes.iter().filter(|o| source_key(o).is_some()) {
        let body = outside_fences(&o.body);
        let title = o.title.as_deref().unwrap_or("");
        let mut found: Vec<String> = DEBRIS
            .iter()
            .filter(|(pat, _)| match *pat {
                // `((` is debris only when a uuid follows it; prose has parentheses.
                "((" => has_raw_block_ref(&body),
                p => body.contains(p),
            })
            .map(|(pat, why)| format!("{pat} ({why})"))
            .collect();
        if ["%3A", "%3F", "%2F", "___"].iter().any(|p| title.contains(p)) {
            found.push(format!("escaped title {title:?}"));
        }
        let key = source_key(o).unwrap_or("").to_string();
        c.record(found.is_empty(), || format!("{key}: {}", found.join(", ")));
    }
    c
}

fn outside_fences(body: &str) -> String {
    let mut out = String::new();
    let mut fence = false;
    for line in body.lines() {
        let t = line.trim_start().trim_start_matches("- ").trim_start();
        if t.starts_with("```") {
            fence = !fence;
            continue;
        }
        if !fence {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn has_raw_block_ref(s: &str) -> bool {
    s.match_indices("((").any(|(i, _)| {
        let rest = &s[i + 2..];
        rest.len() >= 38
            && rest[..36].chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && rest[36..].starts_with("))")
    })
}

/// Every word a reader could search for in the source page is somewhere in its note.
fn c8_text_survives(inv: &Inventory, by_key: &HashMap<&str, &Object>) -> Check {
    let mut c = Check::new("C8", "the words of each page survive into its note", Some(0.995));
    for p in &inv.pages {
        let Some(o) = by_key.get(p.key.as_str()) else { continue };
        let mut have = BTreeSet::new();
        oracle::words_into(&visible_text(o), &mut have);
        for w in &p.words {
            c.record(have.contains(w), || format!("{}: {w:?}", p.key));
        }
    }
    c
}

/// Search, through the app's own `search` command, for the rarest words of each page.
fn c9_findable(inv: &Inventory, by_key: &HashMap<&str, &Object>, store: &FileStore) -> Check {
    let mut c = Check::new("C9", "each page is found by searching its rarest words", Some(0.98));
    let mut df: HashMap<&str, usize> = HashMap::new();
    for p in &inv.pages {
        for w in &p.words {
            *df.entry(w.as_str()).or_default() += 1;
        }
    }
    for p in &inv.pages {
        let Some(o) = by_key.get(p.key.as_str()) else { continue };
        let want = o.id.to_string();
        let mut rare: Vec<&str> = p
            .words
            .iter()
            .map(String::as_str)
            .filter(|w| w.len() >= 5 && w.chars().all(|c| c.is_ascii_alphanumeric()))
            .collect();
        rare.sort_by_key(|w| (df.get(w).copied().unwrap_or(0), *w));
        for w in rare.into_iter().take(3) {
            let hits = fm_app::commands::search(store, w).unwrap_or_default();
            c.record(hits.iter().any(|h| h.id == want), || format!("{}: {w:?}", p.key));
        }
    }
    c
}

fn c10_source_untouched(
    before: &BTreeMap<PathBuf, (u64, Option<SystemTime>)>,
    after: &BTreeMap<PathBuf, (u64, Option<SystemTime>)>,
) -> Check {
    let mut c = Check::new("C10", "the source graph is unchanged", Some(1.0));
    let keys: BTreeSet<&PathBuf> = before.keys().chain(after.keys()).collect();
    for k in keys {
        c.record(before.get(k) == after.get(k), || k.display().to_string());
    }
    c
}

fn c11_second_import_adds_nothing(second: &Value) -> Check {
    let mut c = Check::new("C11", "importing again adds no notes and no attachments", Some(1.0));
    for field in ["notes", "attachments"] {
        let n = second.get(field).and_then(Value::as_u64).unwrap_or(u64::MAX);
        c.record(n == 0, || format!("the second import added {n} {field}"));
    }
    c
}

/// Three imports of one graph leave one asset note per stored file, not three.
fn c11b_one_asset_note_per_file(notes: &[Object]) -> Check {
    let mut c = Check::new("C11b", "three imports leave one asset note per file", Some(1.0));
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for o in notes.iter().filter(|o| o.kind == fm_model::Kind::Asset) {
        for a in &o.assets {
            *seen.entry(a.as_str()).or_default() += 1;
        }
    }
    for (a, n) in seen {
        c.record(n == 1, || format!("{a}: {n} asset notes"));
    }
    c
}

/// A replace over an import nobody edited rewrites every note and keeps none aside.
fn c14_replace_rewrites_what_was_not_edited(third: &Value, inv: &Inventory) -> Check {
    let mut c = Check::new("C14", "replacing an unedited import rewrites every note", Some(1.0));
    let n = |k: &str| third.get(k).and_then(Value::as_u64).unwrap_or(u64::MAX);
    let want = (inv.pages.len() + inv.drawings.len()) as u64;
    c.record(n("replaced") == want, || format!("replaced {} of {want}", n("replaced")));
    c.record(n("keptEdited") == 0, || format!("kept {} as edited", n("keptEdited")));
    c.record(n("notes") == 0, || format!("added {} new notes", n("notes")));
    c
}

/// The preview of a second import names the vault that already holds the graph.
fn c15_an_earlier_import_is_recognised(check: &Value) -> Check {
    let mut c = Check::new("C15", "a second import is told where the first one is", Some(1.0));
    let found = check
        .get("previous")
        .and_then(Value::as_array)
        .is_some_and(|p| p.iter().any(|v| v.get("vault") == Some(&json!("imported"))));
    c.record(found, || format!("check_import answered {check}"));
    c
}

/// `draws/*.excalidraw` becomes a whiteboard note.
fn c13_drawings(inv: &Inventory, notes: &[Object]) -> Check {
    let mut c = Check::new("C13", "each Excalidraw drawing is a whiteboard note", Some(1.0));
    let by_key = by_source_key(notes);
    for d in &inv.drawings {
        let board = by_key.get(d.as_str()).is_some_and(|o| {
            matches!(o.extra.get("view"), Some(fm_model::PropertyValue::Text(v)) if v == "board")
        });
        c.record(board, || d.clone());
    }
    c
}
