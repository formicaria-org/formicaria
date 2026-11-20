//! **A watch's activity files, read into the training log** (`decisions.md` 2026-11-20, *Watch data
//! enters fm as files another app wrote*; the coach plan's Spec S, phase S1).
//!
//! A folder the person names — copied from the watch over USB, or the folder an app such as
//! Gadgetbridge exports to — is read, never written. Each activity file becomes one `log: workout`
//! entry with the existing activity shape (`activity`, `minutes`, `effort`, `counts_as`) plus the
//! measured facts the file holds, a table fm writes once into its body, and, for a strength session,
//! one set note per set the watch counted. `fm_core::fit` decodes; this module decides what notes
//! that makes, and the `fit_preview` / `import_fit` arms in `dispatch.rs` hold the lock and write.
//!
//! **Idempotent, never destructive** (Spec S.0):
//! - a file is known by `source_key` — `fit:` and a salted HMAC of its `file_id` (type,
//!   manufacturer, product, serial, time created). The salt is random, made once per vault and kept
//!   at `.formicaria/fit-salt`, so it travels with the notes; the serial number never appears in a
//!   note in clear (a 32-bit field, so a plain hash could be reversed by trying them all);
//! - a known key is skipped. The same key with **other bytes** is reported ("two versions") and
//!   nothing is overwritten — an edited note (`import_hash` no longer matches) is said to be edited;
//! - an entry of the same sport starting within a minute with the same timer time is a **merge
//!   proposal** in the report, never merged and never written twice;
//! - a damaged file is refused and named, with what to do about it.
//!
//! **Time** (the plan's P1.3): every FIT time is UTC; `start` is written on the app's clock, as the
//! calendar reader writes a meeting; `day` is the **watch's own local date** of the start, from the
//! activity's `local_timestamp` minus its `timestamp`, fixed at import and the same on every device.

use fm_core::fit::{self, FitFile, Session};
use fm_core::import::{content_hash, IMPORT_HASH, SOURCE_KEY};
use fm_core::zone::Zone;
use fm_core::{Store, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue, Stamp};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// The training log's kind.
pub const LOG_KIND: &str = "workout";
/// Where an imported entry's file is: `sha256:<hex>`, a blob in the vault (kept like an audio
/// file's cover — a blob a property names, with no asset note of its own).
pub const SOURCE_FIT: &str = "source_fit";
/// The vault's own random salt for `source_key`, relative to the vault's root.
pub const SALT_FILE: &str = ".formicaria/fit-salt";

/// More files than this in one folder is a mistyped path, not a watch.
const MAX_FILES: usize = 5000;
/// How deep a folder is walked: a watch's own tree is two or three levels.
const MAX_DEPTH: usize = 4;
/// Names listed back in a report, at most.
const MAX_LISTED: usize = 200;

/// The sentence every refusal ends with (the plan's P1.2): a damaged export is never retried by the
/// app that wrote it, so the person is told how to get a good copy.
pub const REEXPORT: &str = "Delete this file in the folder and export the activity again, or \
                            copy it from the watch over USB.";

// ── the shipped tables ───────────────────────────────────────────────────────────────────────────

pub const SPORTS: &str = include_str!("shipped/fit-sports.json");
pub const EXERCISES: &str = include_str!("shipped/fit-exercises.json");

#[derive(Deserialize, Debug, Clone)]
pub struct SportRow {
    pub fit_sport: String,
    #[serde(default)]
    pub fit_sub_sport: Option<String>,
    pub activity: String,
    pub counts_as: String,
    pub sport: String,
    #[serde(default)]
    pub indoor: bool,
}

#[derive(Deserialize)]
struct SportTable {
    sports: Vec<SportRow>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExerciseRow {
    pub category: String,
    pub name: String,
    pub card: String,
}

#[derive(Deserialize)]
struct ExerciseTable {
    map: Vec<ExerciseRow>,
}

pub fn sport_rows() -> Vec<SportRow> {
    serde_json::from_str::<SportTable>(SPORTS).map(|t| t.sports).unwrap_or_default()
}

pub fn exercise_rows() -> Vec<ExerciseRow> {
    serde_json::from_str::<ExerciseTable>(EXERCISES).map(|t| t.map).unwrap_or_default()
}

/// The table's row for a sport and sub-sport: the sub-sport's own row, else the sport's.
fn sport_row<'a>(rows: &'a [SportRow], sport: &str, sub: Option<&str>) -> Option<&'a SportRow> {
    rows.iter()
        .find(|r| {
            r.fit_sport == sport && r.fit_sub_sport.is_some() && r.fit_sub_sport.as_deref() == sub
        })
        .or_else(|| rows.iter().find(|r| r.fit_sport == sport && r.fit_sub_sport.is_none()))
}

/// A card the app carries: its name and, when it is one of the four, the region it trains.
#[derive(Debug, Clone)]
struct Card {
    name: String,
    region: Option<String>,
}

fn cards() -> HashMap<(String, String), Card> {
    let lib: serde_json::Value = serde_json::from_str(crate::log::EXERCISES).unwrap_or_default();
    let by_id: HashMap<&str, &serde_json::Value> = lib["exercises"]
        .as_array()
        .map(|a| a.iter().filter_map(|e| Some((e["id"].as_str()?, e))).collect())
        .unwrap_or_default();
    exercise_rows()
        .into_iter()
        .filter_map(|r| {
            let e = by_id.get(r.card.as_str())?;
            let region = e["area"]
                .as_str()
                .filter(|a| ["upper-push", "upper-pull", "lower-push", "lower-pull"].contains(a))
                .map(str::to_string);
            Some(((r.category, r.name), Card { name: e["name"].as_str()?.to_string(), region }))
        })
        .collect()
}

/// `goblet_squat` → `Goblet squat`: the watch's own name, as text.
fn humane(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut c = words.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

// ── the salt ─────────────────────────────────────────────────────────────────────────────────────

/// The vault's salt, when it has one.
pub fn read_salt(vault: &Path) -> Option<Vec<u8>> {
    let text = std::fs::read_to_string(vault.join(SALT_FILE)).ok()?;
    let hex = text.trim();
    (hex.len() == 64)
        .then(|| (0..32).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect())
        .flatten()
}

/// A fresh random salt, from the system's randomness.
pub fn new_salt() -> Result<Vec<u8>, String> {
    let mut b = vec![0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(b)
}

/// Write the salt, once. Returns the file's path, for the commit.
pub fn write_salt(vault: &Path, salt: &[u8]) -> Result<PathBuf, String> {
    let path = vault.join(SALT_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let hex: String = salt.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(&path, hex + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// `source_key` for a file: its `file_id`, keyed. A file with no identity is keyed by its bytes.
pub fn source_key(salt: &[u8], file: &FitFile, bytes: &[u8]) -> String {
    let what = file
        .file_id
        .as_ref()
        .and_then(|f| f.identity())
        .unwrap_or_else(|| format!("bytes|{}", fm_core::blob::sha256_hex(bytes)));
    format!("fit:{}", &fm_core::blob::hmac_sha256_hex(salt, what.as_bytes())[..32])
}

// ── the folder ───────────────────────────────────────────────────────────────────────────────────

/// The `.fit` files under a path the person typed (or the one file it names), only ever read.
#[derive(Debug, Default)]
pub struct Source {
    pub files: Vec<PathBuf>,
    /// One sentence when the path cannot be imported from.
    pub problem: Option<String>,
}

fn is_fit(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("fit"))
}

pub fn scan(typed: &str) -> Source {
    let typed = typed.trim();
    if typed.is_empty() {
        return Source {
            problem: Some("Type the folder the watch files are in.".into()),
            ..Default::default()
        };
    }
    let root = PathBuf::from(crate::vaults::expand_home(typed));
    let meta = match std::fs::metadata(&root) {
        Ok(m) => m,
        Err(_) => {
            return Source {
                problem: Some(format!("There is no folder or file at {}.", root.display())),
                ..Default::default()
            }
        }
    };
    if meta.is_file() {
        return if is_fit(&root) {
            Source { files: vec![root], problem: None }
        } else {
            Source { problem: Some("That file is not a .fit file.".into()), ..Default::default() }
        };
    }
    let mut files = Vec::new();
    let mut stack = vec![(root.clone(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            // Never follow a link: a loop, or a path out of the folder the person named.
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() && depth < MAX_DEPTH {
                stack.push((p, depth + 1));
            } else if ft.is_file() && is_fit(&p) {
                files.push(p);
                if files.len() > MAX_FILES {
                    return Source {
                        files: Vec::new(),
                        problem: Some(format!(
                            "More than {MAX_FILES} .fit files are under {} — name the watch's own folder.",
                            root.display()
                        )),
                    };
                }
            }
        }
    }
    files.sort();
    let problem =
        files.is_empty().then(|| format!("There are no .fit files in {}.", root.display()));
    Source { files, problem }
}

// ── what the vault already has ───────────────────────────────────────────────────────────────────

/// A note an earlier import wrote.
#[derive(Debug, Clone)]
pub struct Known {
    pub id: Id,
    pub title: String,
    pub source_fit: String,
    /// Its fields are no longer what the import wrote.
    pub edited: bool,
}

/// An entry of the training log a new one could be a second copy of.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: Id,
    pub title: String,
    pub sport: String,
    /// Unix seconds, from its `start` on the app's clock.
    pub at: i64,
    pub timer_s: Option<i64>,
    pub minutes: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Existing {
    pub by_key: HashMap<String, Known>,
    pub entries: Vec<Candidate>,
}

fn text(o: &Object, k: &str) -> String {
    match o.get(k) {
        PropertyValue::Text(s) => s,
        _ => String::new(),
    }
}

fn int(o: &Object, k: &str) -> Option<i64> {
    match o.get(k) {
        PropertyValue::Int(n) => Some(n),
        PropertyValue::Text(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// The training log of one vault, in one pass: earlier imports by key, and every activity entry
/// with a time, for the near-duplicate check.
pub fn existing(store: &dyn Store, vault: &str, zone: &Zone) -> Result<Existing, StoreError> {
    let q = fm_query::Query { filter: crate::thread::log_base(LOG_KIND), ..Default::default() };
    let mut out = Existing::default();
    for o in store.query(&q)?.rows.iter().filter(|o| o.vault == vault) {
        let key = text(o, SOURCE_KEY);
        if key.starts_with("fit:") {
            let edited = match o.extra.get(IMPORT_HASH) {
                Some(PropertyValue::Text(h)) => content_hash(o) != *h,
                _ => true,
            };
            out.by_key.insert(
                key,
                Known {
                    id: o.id,
                    title: o.title.clone().unwrap_or_default(),
                    source_fit: text(o, SOURCE_FIT),
                    edited,
                },
            );
        }
        let sport = match text(o, "fit_sport") {
            s if !s.is_empty() => s,
            _ => text(o, "activity").to_lowercase(),
        };
        if sport.is_empty() || !text(o, "exercise").is_empty() {
            continue;
        }
        let Some(start) = o.start else { continue };
        let Some(at) = zone.instant(start.date, start.time.unwrap_or(time::Time::MIDNIGHT)) else {
            continue;
        };
        out.entries.push(Candidate {
            id: o.id,
            title: o.title.clone().unwrap_or_default(),
            sport,
            at,
            timer_s: int(o, "timer_s"),
            minutes: int(o, "minutes"),
        });
    }
    Ok(out)
}

// ── the report ───────────────────────────────────────────────────────────────────────────────────

/// One activity that would be (or was) written.
#[derive(Serialize, Debug, Clone)]
pub struct Item {
    pub file: String,
    pub title: String,
    pub day: String,
    pub minutes: i64,
    #[serde(rename = "distanceM", skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<i64>,
    pub sets: usize,
}

/// A file refused, and why.
#[derive(Serialize, Debug, Clone)]
pub struct Refusal {
    pub file: String,
    pub reason: String,
}

/// The same file identity as an earlier import, with other bytes.
#[derive(Serialize, Debug, Clone)]
pub struct OtherVersion {
    pub file: String,
    pub note: String,
    pub id: String,
    /// The note was changed since it was imported.
    pub edited: bool,
}

/// A file that looks like an entry already there: proposed, never merged.
#[derive(Serialize, Debug, Clone)]
pub struct Merge {
    pub file: String,
    /// The entry it looks like: its title and id, or the other file in this folder.
    pub with: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub day: String,
}

/// What a preview would write, or what an import did — the same shape for both, so the panel says
/// the same thing before and after.
#[derive(Serialize, Debug, Default)]
pub struct FitReport {
    /// `.fit` files found.
    pub files: usize,
    /// Activities and sets new to this vault.
    pub activities: usize,
    pub sets: usize,
    /// Files imported before, left as they are.
    pub known: usize,
    #[serde(rename = "otherVersions")]
    pub other_versions: Vec<OtherVersion>,
    pub merges: Vec<Merge>,
    pub refused: Vec<Refusal>,
    /// Files that are not activities (settings, a workout, a day's monitoring): left out.
    #[serde(rename = "notActivities")]
    pub not_activities: Vec<String>,
    pub items: Vec<Item>,
    /// Why nothing can be imported from this path, if so.
    pub problem: Option<String>,
    /// Something new to write and nothing in the way.
    pub ok: bool,
    /// After an import: how many notes were written, and whether they went into history as one.
    pub written: usize,
    pub recorded: bool,
    pub warnings: Vec<String>,
    pub vault: String,
}

/// A file read and turned into notes, waiting to be written.
pub struct Planned {
    pub notes: Vec<Object>,
    /// The file's bytes; the caller keeps them as a blob.
    pub blob: Vec<u8>,
}

/// Everything an import would do.
pub struct Plan {
    pub report: FitReport,
    pub planned: Vec<Planned>,
}

fn name_of(p: &Path) -> String {
    p.file_name().and_then(|n| n.to_str()).unwrap_or("a file").to_string()
}

fn day_string(d: time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

/// One watch file to read, from wherever its bytes came: a folder on this computer (`plan`), or the
/// phone's share menu and granted folder (`plan_files`, through a chunked upload). The two routes
/// share everything after this point, so a file makes the same notes whichever way it arrived.
pub struct InFile {
    /// The file's own name, as the report names it.
    pub name: String,
    /// The order files are taken in (the path, for a folder): a pair's watch copy is preferred,
    /// then this.
    pub order: String,
    /// The bytes, or why they could not be read.
    pub bytes: Result<Vec<u8>, String>,
}

/// Read every file under a folder the person typed and decide what it becomes. Reads the files and
/// nothing else; writes nothing.
pub fn plan(source: &Source, salt: &[u8], zone: &Zone, existing: &Existing) -> Plan {
    let files = source
        .files
        .iter()
        .map(|p| InFile {
            name: name_of(p),
            order: p.display().to_string(),
            bytes: std::fs::read(p).map_err(|e| e.to_string()),
        })
        .collect();
    plan_files(files, source.problem.clone(), salt, zone, existing)
}

/// Decide what files already in memory become — the core both routes share. Writes nothing.
pub fn plan_files(
    files: Vec<InFile>,
    problem: Option<String>,
    salt: &[u8],
    zone: &Zone,
    existing: &Existing,
) -> Plan {
    let rows = sport_rows();
    let cards = cards();
    let mut report = FitReport { files: files.len(), problem, ..Default::default() };
    let mut planned: Vec<Planned> = Vec::new();
    // Keys and near-duplicates within this run, too: a folder can hold the same activity twice.
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut ours: Vec<(Candidate, String)> = Vec::new();

    // Read them all first, then prefer the watch's own copy of a pair (the one carrying a
    // `file_creator`) by taking those first.
    let mut read: Vec<(String, String, Vec<u8>, FitFile)> = Vec::new();
    for f in files {
        let InFile { name, order, bytes } = f;
        let bytes = match bytes {
            Ok(b) => b,
            Err(e) => {
                report
                    .refused
                    .push(Refusal { file: name, reason: format!("it could not be read ({e}).") });
                continue;
            }
        };
        match fit::decode(&bytes) {
            Ok(f) => read.push((order, name, bytes, f)),
            Err(e) => {
                report.refused.push(Refusal { file: name, reason: format!("{e}. {REEXPORT}") })
            }
        }
    }
    read.sort_by(|(a, _, _, f), (b, _, _, g)| (!f.has_creator, a).cmp(&(!g.has_creator, b)));

    for (_, name, bytes, file) in read {
        let kind = file.file_id.as_ref().and_then(|f| f.file_type.clone());
        if kind.as_deref().is_some_and(|k| k != "activity") || file.sessions.is_empty() {
            report.not_activities.push(name);
            continue;
        }
        let base = source_key(salt, &file, &bytes);
        let blob_ref = format!("sha256:{}", fm_core::blob::sha256_hex(&bytes));
        if let Some(k) = existing.by_key.get(&base) {
            if k.source_fit == blob_ref || k.source_fit.is_empty() {
                report.known += 1;
            } else {
                report.other_versions.push(OtherVersion {
                    file: name,
                    note: k.title.clone(),
                    id: k.id.to_string(),
                    edited: k.edited,
                });
            }
            continue;
        }
        if !seen_keys.insert(base.clone()) {
            report.merges.push(Merge {
                file: name,
                with: "another file in this folder with the same identity".into(),
                id: None,
                day: String::new(),
            });
            continue;
        }
        let mut notes = Vec::new();
        let mut held = false;
        for (i, session) in file.sessions.iter().enumerate() {
            let key = if i == 0 { base.clone() } else { format!("{base}:{}", i + 1) };
            let Some(built) = build(&file, session, &key, &blob_ref, zone, &rows, &cards) else {
                continue;
            };
            let this = &built.candidate;
            let near = |c: &Candidate| {
                c.sport == this.sport
                    && (c.at - this.at).abs() <= 60
                    && match (c.timer_s, this.timer_s) {
                        (Some(a), Some(b)) => (a - b).abs() <= 1,
                        _ => c.minutes.is_some() && c.minutes == this.minutes,
                    }
            };
            if let Some(c) = existing.entries.iter().find(|c| near(c)) {
                report.merges.push(Merge {
                    file: name.clone(),
                    with: c.title.clone(),
                    id: Some(c.id.to_string()),
                    day: built.day.clone(),
                });
                held = true;
                break;
            }
            if let Some((_, other)) = ours.iter().find(|(c, _)| near(c)) {
                report.merges.push(Merge {
                    file: name.clone(),
                    with: other.clone(),
                    id: None,
                    day: built.day.clone(),
                });
                held = true;
                break;
            }
            ours.push((built.candidate.clone(), name.clone()));
            report.activities += 1;
            report.sets += built.notes.len() - 1;
            if report.items.len() < MAX_LISTED {
                report.items.push(Item {
                    file: name.clone(),
                    title: built.notes[0].title.clone().unwrap_or_default(),
                    day: built.day.clone(),
                    minutes: int(&built.notes[0], "minutes").unwrap_or(0),
                    distance_m: int(&built.notes[0], "distance_m"),
                    sets: built.notes.len() - 1,
                });
            }
            notes.extend(built.notes);
        }
        if held {
            // A pair is all or nothing: half a multisport file is not written beside its twin.
            continue;
        }
        if !notes.is_empty() {
            planned.push(Planned { notes, blob: bytes });
        }
    }
    report.refused.truncate(MAX_LISTED);
    report.not_activities.truncate(MAX_LISTED);
    report.ok = report.problem.is_none() && !planned.is_empty();
    Plan { report, planned }
}

// ── one session → notes ──────────────────────────────────────────────────────────────────────────

struct Built {
    notes: Vec<Object>,
    day: String,
    candidate: Candidate,
}

fn whole(x: f64) -> i64 {
    x.round() as i64
}

/// `m:ss`, or `h:mm:ss`.
fn clock(secs: f64) -> String {
    let s = whole(secs).max(0);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn km(m: f64) -> String {
    format!("{:.2} km", m / 1000.0)
}

fn in_window(t: Option<i64>, from: i64, to: i64) -> bool {
    t.is_some_and(|t| t >= from && t < to)
}

#[allow(clippy::too_many_arguments)]
fn build(
    file: &FitFile,
    s: &Session,
    key: &str,
    blob_ref: &str,
    zone: &Zone,
    rows: &[SportRow],
    cards: &HashMap<(String, String), Card>,
) -> Option<Built> {
    let start = s.start.or(file.activity_end.map(|e| e - whole(s.elapsed_s.unwrap_or(0.0))))?;
    let at = OffsetDateTime::from_unix_timestamp(start).ok()?;
    let here = at.to_offset(zone.offset_at(start));
    let stamp = Stamp::at(here.date(), here.time());
    let mut quality: Vec<String> = Vec::new();
    // **The watch's own day** (P1.3): its local date of the start, never this computer's.
    let day = match file.local_offset_s {
        Some(off) => OffsetDateTime::from_unix_timestamp(start + off).ok()?.date(),
        None => {
            quality.push("day_by_this_clock".into());
            here.date()
        }
    };
    let end = start + whole(s.elapsed_s.or(s.timer_s).unwrap_or(0.0)).max(1);

    let sport = s.sport.clone().unwrap_or_else(|| "generic".into());
    let row = sport_row(rows, &sport, s.sub_sport.as_deref());
    let activity = row.map_or_else(|| "other".to_string(), |r| r.activity.clone());
    let fm_sport = row.map_or_else(|| "other".to_string(), |r| r.sport.clone());
    let swim = fm_sport == "swim";

    let mut o = Object::new(Kind::Note, String::new());
    o.title = Some(activity.clone());
    o.start = Some(stamp);
    let mut put = |k: &str, v: PropertyValue| {
        o.extra.insert(k.to_string(), v);
    };
    let txt = |s: &str| PropertyValue::Text(s.to_string());
    put(crate::thread::LOG, txt(LOG_KIND));
    put("activity", txt(&activity));
    if let Some(r) = row {
        put("counts_as", txt(&r.counts_as));
        if r.indoor {
            put("indoor", PropertyValue::Bool(true));
        }
    }
    put("sport", txt(&fm_sport));
    put("fit_sport", txt(&sport));
    if let Some(sub) = &s.sub_sport {
        put("fit_sub_sport", txt(sub));
    }
    let timer = s.timer_s.unwrap_or(0.0);
    put("minutes", PropertyValue::Int(whole(timer / 60.0)));
    put("timer_s", PropertyValue::Int(whole(timer)));
    if let Some(e) = s.elapsed_s {
        put("elapsed_s", PropertyValue::Int(whole(e)));
    }

    // Pool lengths: distance from the active ones × the pool, intervals between the rests.
    let lengths: Vec<&fit::Length> = file
        .lengths
        .iter()
        .filter(|l| file.sessions.len() == 1 || in_window(l.start, start, end))
        .collect();
    let pool = s.pool_length_m.filter(|p| *p > 0.0);
    let distance = match (swim, pool) {
        (true, Some(p)) if !lengths.is_empty() => {
            Some(lengths.iter().filter(|l| l.active).count() as f64 * p)
        }
        _ => s.distance_m,
    };
    if let Some(d) = distance.filter(|d| *d > 0.0) {
        put("distance_m", PropertyValue::Int(whole(d)));
    }
    if let Some(a) = s.ascent_m {
        put("ascent_m", PropertyValue::Int(whole(a)));
    }
    if let Some(d) = s.descent_m {
        put("descent_m", PropertyValue::Int(whole(d)));
    }
    // **Swim heart rate is the watch's, and nothing counts it** (decision 2026-11-20, 4).
    let hr_prefix = if swim { "watch_" } else { "" };
    if let Some(h) = s.hr_avg {
        put(&format!("{hr_prefix}hr_avg_bpm"), PropertyValue::Int(whole(h)));
    }
    if let Some(h) = s.hr_max {
        put(&format!("{hr_prefix}hr_max_bpm"), PropertyValue::Int(whole(h)));
    }
    if swim && (s.hr_avg.is_some() || s.hr_max.is_some()) {
        quality.push("hr_wrist_water".into());
    }
    if let Some(c) = s.cadence_avg.filter(|c| *c > 0.0 && !swim) {
        if matches!(fm_sport.as_str(), "run" | "walk" | "hike") {
            // A run's cadence is written in strides a minute; a person counts steps.
            let steps = 2.0 * (c + s.fractional_cadence.unwrap_or(0.0));
            put("cadence_avg_spm", PropertyValue::Int(whole(steps)));
        } else {
            put("cadence_avg_rpm", PropertyValue::Int(whole(c)));
        }
    }
    if let Some(r) = s.workout_rpe.filter(|r| *r > 0.0 && *r <= 100.0) {
        // "multiplied 10x" (the profile's comment): the person's own 0–10, typed on the watch. It is
        // the same scale as `effort`, so it fills that too — never heart rate (D10).
        let rpe = whole(r / 10.0);
        put("rpe", PropertyValue::Int(rpe));
        put("effort", PropertyValue::Int(rpe));
    }
    if let Some(t) = s.training_effect.filter(|t| *t > 0.0) {
        put("watch_training_effect_x10", PropertyValue::Int(whole(t * 10.0)));
    }
    if let Some(t) = s.anaerobic_effect.filter(|t| *t > 0.0) {
        put("watch_anaerobic_effect_x10", PropertyValue::Int(whole(t * 10.0)));
    }
    if let Some(k) = s.calories.filter(|k| *k > 0.0) {
        put("watch_kcal", PropertyValue::Int(whole(k)));
    }
    let st = &file.stream;
    if st.with_position > 0 && st.with_position * 2 < st.records {
        quality.push("gps_short".into());
    }

    // ── the body: a table written once ──
    let mut body = String::new();
    let laps: Vec<&fit::Lap> = file
        .laps
        .iter()
        .filter(|l| file.sessions.len() == 1 || in_window(l.start, start, end))
        .collect();
    if swim && !lengths.is_empty() {
        let (table, mixed, drill) = intervals(&lengths, pool);
        body.push_str(&table);
        if mixed {
            quality.push("mixed_stroke".into());
        }
        if drill {
            quality.push("drill_typed".into());
        }
    } else if !laps.is_empty() && !matches!(fm_sport.as_str(), "strength") {
        body.push_str(&lap_table(&laps, &fm_sport, swim));
    }
    let steps = steps_table(&file.steps, &laps);
    if !steps.is_empty() {
        body.push_str("\n\n");
        body.push_str(&steps);
    }
    let mut lines: Vec<&str> = Vec::new();
    if swim && quality.iter().any(|q| q == "hr_wrist_water") {
        lines.push(
            "Heart rate in water is the watch's reading from the wrist, often wrong there; it is \
             kept as the watch's and nothing counts it.",
        );
    }
    if s.sub_sport.as_deref() == Some("open_water") {
        lines.push("Distance in open water is from GPS and is not validated for this device.");
    }
    if s.ascent_m.is_none()
        && matches!(fm_sport.as_str(), "run" | "ride" | "walk" | "hike")
        && row.is_some_and(|r| !r.indoor)
    {
        lines.push("No climb was recorded.");
    }
    if !lines.is_empty() {
        body.push_str("\n\n");
        body.push_str(&lines.join("\n\n"));
    }
    o.body = body.trim().to_string();

    let day_value = PropertyValue::Stamp(Stamp::day(day));
    let candidate = Candidate {
        id: o.id,
        title: activity.clone(),
        sport: sport.clone(),
        at: zone.instant(stamp.date, stamp.time.unwrap_or(time::Time::MIDNIGHT)).unwrap_or(start),
        timer_s: Some(whole(timer)),
        minutes: Some(whole(timer / 60.0)),
    };

    // ── strength: one set note per active set ──
    let mut set_notes = Vec::new();
    if fm_sport == "strength" || file.sets.iter().any(|x| x.active && x.reps.is_some()) {
        let mut counted: HashMap<String, i64> = HashMap::new();
        for (i, x) in file
            .sets
            .iter()
            .filter(|x| file.sessions.len() == 1 || in_window(x.start, start, end))
            .filter(|x| x.active)
            .enumerate()
        {
            let card = x
                .category
                .as_ref()
                .zip(x.exercise.as_ref())
                .and_then(|(c, n)| cards.get(&(c.clone(), n.clone())));
            let exercise = match (card, &x.exercise, &x.category) {
                (Some(c), _, _) => c.name.clone(),
                (None, Some(n), _) => humane(n),
                (None, None, Some(c)) => humane(c),
                _ => "Strength exercise".to_string(),
            };
            let n = counted.entry(exercise.clone()).or_insert(0);
            *n += 1;
            let mut so = Object::new(Kind::Note, String::new());
            so.title = Some(exercise.clone());
            so.start = Some(stamp);
            so.extra.insert(crate::thread::LOG.into(), txt(LOG_KIND));
            so.extra.insert("exercise".into(), txt(&exercise));
            so.extra.insert("set".into(), PropertyValue::Int(*n));
            if let Some(r) = x.reps {
                so.extra.insert("reps".into(), PropertyValue::Int(i64::from(r)));
            }
            if let Some(w) = x.weight_kg {
                let load = whole(w * 1000.0);
                so.extra.insert("load_g".into(), PropertyValue::Int(load));
                if let Some(r) = x.reps {
                    so.extra.insert("work_g".into(), PropertyValue::Int(load * i64::from(r)));
                }
            }
            if let Some(region) = card.and_then(|c| c.region.clone()) {
                so.extra.insert("region".into(), txt(&region));
            }
            so.extra.insert(
                "quality".into(),
                PropertyValue::List(vec![PropertyValue::Text("reps_watch".into())]),
            );
            so.extra.insert("day".into(), day_value.clone());
            so.extra.insert(SOURCE_KEY.into(), txt(&format!("{key}:set:{}", i + 1)));
            let h = content_hash(&so);
            so.extra.insert(IMPORT_HASH.into(), PropertyValue::Text(h));
            set_notes.push(so);
        }
    }

    if !quality.is_empty() {
        o.extra.insert(
            "quality".into(),
            PropertyValue::List(quality.into_iter().map(PropertyValue::Text).collect()),
        );
    }
    o.extra.insert("day".into(), day_value);
    o.extra.insert(SOURCE_KEY.into(), txt(key));
    o.extra.insert(SOURCE_FIT.into(), txt(blob_ref));
    o.assets.push(blob_ref.to_string());
    let h = content_hash(&o);
    o.extra.insert(IMPORT_HASH.into(), PropertyValue::Text(h));

    let mut notes = vec![o];
    notes.extend(set_notes);
    Some(Built { notes, day: day_string(day), candidate })
}

fn lap_table(laps: &[&fit::Lap], sport: &str, swim: bool) -> String {
    let ride = sport == "ride";
    let mut t = format!(
        "| Lap | Time | Distance |{} {} |\n|---|---|---|{}---|\n",
        if swim { "" } else { " Avg HR |" },
        if ride { "Speed" } else { "Pace" },
        if swim { "" } else { "---|" },
    );
    for (i, l) in laps.iter().enumerate() {
        let time = l.timer_s.unwrap_or(0.0);
        let dist = l.distance_m.unwrap_or(0.0);
        let speed =
            l.speed_avg.filter(|v| *v > 0.0).or((time > 0.0 && dist > 0.0).then(|| dist / time));
        let pace = match speed {
            Some(v) if ride => format!("{:.1} km/h", v * 3.6),
            Some(v) => format!("{} /km", clock(1000.0 / v)),
            None => "—".into(),
        };
        let hr = l.hr_avg.map(|h| whole(h).to_string()).unwrap_or_else(|| "—".into());
        if swim {
            t.push_str(&format!("| {} | {} | {} | {} |\n", i + 1, clock(time), km(dist), pace));
        } else {
            t.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                i + 1,
                clock(time),
                km(dist),
                hr,
                pace
            ));
        }
    }
    t.trim_end().to_string()
}

/// Lengths grouped into intervals: runs of active lengths between rests. Returns the table, and
/// whether any interval changed stroke, and whether any length was drill.
fn intervals(lengths: &[&fit::Length], pool: Option<f64>) -> (String, bool, bool) {
    let mut groups: Vec<Vec<&fit::Length>> = vec![Vec::new()];
    for l in lengths {
        if l.active {
            groups.last_mut().expect("one group").push(l);
        } else if !groups.last().expect("one group").is_empty() {
            groups.push(Vec::new());
        }
    }
    groups.retain(|g| !g.is_empty());
    let mut mixed = false;
    let mut drill = false;
    let pool = pool.unwrap_or(0.0);
    let mut t = String::from(
        "| Interval | Lengths | Distance | Time | Stroke | Strokes | Pace /100 m | SWOLF |\n\
         |---|---|---|---|---|---|---|---|\n",
    );
    for (i, g) in groups.iter().enumerate() {
        let strokes_kinds: HashSet<&str> = g.iter().filter_map(|l| l.stroke.as_deref()).collect();
        let changes = strokes_kinds.len() > 1
            || strokes_kinds.iter().any(|s| matches!(*s, "mixed" | "im" | "im_by_round" | "rimo"));
        mixed |= changes;
        drill |= strokes_kinds.contains("drill");
        let stroke = if changes {
            "mixed".to_string()
        } else {
            strokes_kinds.iter().next().map(|s| s.to_string()).unwrap_or_else(|| "—".into())
        };
        let time: f64 = g.iter().filter_map(|l| l.timer_s).sum();
        let strokes: u32 = g.iter().filter_map(|l| l.strokes).sum();
        let dist = g.len() as f64 * pool;
        let pace = if dist > 0.0 { clock(time / dist * 100.0) } else { "—".into() };
        // SWOLF: time plus strokes, per length of this pool — meaningful only within one pool.
        let swolf = if g.is_empty() || strokes == 0 {
            "—".to_string()
        } else {
            whole((time + f64::from(strokes)) / g.len() as f64).to_string()
        };
        t.push_str(&format!(
            "| {} | {} | {} m | {} | {} | {} | {} | {} |\n",
            i + 1,
            g.len(),
            whole(dist),
            clock(time),
            stroke,
            strokes,
            pace,
            swolf
        ));
    }
    (t.trim_end().to_string(), mixed, drill)
}

/// Steps of a followed workout beside the laps that did them.
fn steps_table(steps: &[fit::WorkoutStep], laps: &[&fit::Lap]) -> String {
    if steps.is_empty() || !laps.iter().any(|l| l.wkt_step_index.is_some()) {
        return String::new();
    }
    let mut t = String::from("| Step | Planned | Done |\n|---|---|---|\n");
    for (i, s) in steps.iter().enumerate() {
        let idx = s.index.unwrap_or(i as u32);
        let planned = match (s.duration_s, s.distance_m) {
            (Some(d), _) => clock(d),
            (None, Some(m)) => km(m),
            _ => "—".into(),
        };
        let done: Vec<String> = laps
            .iter()
            .filter(|l| l.wkt_step_index == Some(idx))
            .map(|l| {
                format!("{} · {}", clock(l.timer_s.unwrap_or(0.0)), km(l.distance_m.unwrap_or(0.0)))
            })
            .collect();
        let name = s
            .name
            .clone()
            .or_else(|| s.intensity.clone())
            .unwrap_or_else(|| format!("Step {}", i + 1));
        let done = if done.is_empty() { "—".to_string() } else { done.join(", ") };
        t.push_str(&format!("| {name} | {planned} | {done} |\n"));
    }
    t.trim_end().to_string()
}

// ── writing ──────────────────────────────────────────────────────────────────────────────────────

/// Write the planned notes into `vault`. Each note is stamped with the vault first —
/// `MultiStore::put` routes on it (see `commands::write_import`). One unwritable note does not
/// abandon the rest; it is reported.
pub fn write(store: &mut dyn Store, vault: &str, planned: Vec<Planned>, report: &mut FitReport) {
    for p in planned {
        for mut note in p.notes {
            note.vault = vault.to_string();
            match store.put(&note) {
                Ok(()) => report.written += 1,
                Err(e) => report.warnings.push(format!(
                    "{} could not be written ({e})",
                    note.title.as_deref().unwrap_or("an entry")
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapped_card_is_a_card_the_app_carries() {
        let lib: serde_json::Value = serde_json::from_str(crate::log::EXERCISES).unwrap();
        let ids: HashSet<&str> =
            lib["exercises"].as_array().unwrap().iter().filter_map(|e| e["id"].as_str()).collect();
        for r in exercise_rows() {
            assert!(ids.contains(r.card.as_str()), "{} is not a card", r.card);
        }
        assert_eq!(cards().len(), exercise_rows().len());
    }

    #[test]
    fn every_mapped_exercise_is_a_name_the_profile_knows() {
        for r in exercise_rows() {
            let found =
                (0..2000).any(|n| fit::exercise_name(&r.category, n).as_deref() == Some(&r.name));
            assert!(found, "{} / {} is not in the profile", r.category, r.name);
        }
    }

    #[test]
    fn the_tables_parse_and_say_when() {
        assert!(sport_rows().len() > 10);
        for t in [SPORTS, EXERCISES] {
            let v: serde_json::Value = serde_json::from_str(t).unwrap();
            assert!(v["as_at"].as_str().is_some_and(|d| d.len() == 10));
        }
        let rows = sport_rows();
        assert_eq!(sport_row(&rows, "running", Some("treadmill")).map(|r| r.indoor), Some(true));
        assert_eq!(sport_row(&rows, "running", Some("street")).map(|r| r.indoor), Some(false));
        assert!(sport_row(&rows, "golf", None).is_none());
    }

    #[test]
    fn the_salt_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_salt(dir.path()).is_none());
        let s = new_salt().unwrap();
        write_salt(dir.path(), &s).unwrap();
        assert_eq!(read_salt(dir.path()), Some(s));
    }

    #[test]
    fn a_name_from_the_watch_reads_as_words() {
        assert_eq!(humane("goblet_squat"), "Goblet squat");
    }
}
