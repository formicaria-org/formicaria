//! **The usage diary** (`decisions.md` 2026-11-20; the owner: *"we need to collect usage data from me
//! on long run, to improve the app efficiency"*).
//!
//! What formicaria did on this device, kept **on this device only** and never sent anywhere — a phone's
//! diary travels to the person's computer with their notes' own sync, nowhere else. On by default, a
//! switch in Settings, and *Delete it* removes every line. It holds:
//!
//! - **efficiency**: each part's memory and processor every few minutes (`sample`), the time each
//!   command took, slow ones on their own line (`slow`);
//! - **starts and stops, and how each ended** (`start`, `stop`, `ended_unsaid`: the last run left
//!   without saying, so killed, crashed or switched off), the browser's own (`browser`);
//! - **which features are used**: commands counted by name (`use`), pages by kind (`page`), errors
//!   with what could name a person's things taken out (`error`).
//!
//! **Never** a note's text or title, a web address, a search, a mail: commands are counted by their
//! fixed names, a page by its kind, and what the pages tell the diary (`note`) is held to a short list
//! of fields, each cleaned (`clean`).
//!
//! One JSON object per line, a file a day (`2026-11-20.jsonl`), kept 90 days and at most 100 MB.
//! `pixi run usage-report` reads it.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Days a diary file is kept.
pub const KEEP_DAYS: i64 = 90;
/// The most the diary may take on disk; the oldest days go first.
pub const MAX_BYTES: u64 = 100 * 1024 * 1024;
/// A command that took this long gets a line of its own.
pub const SLOW_MS: u64 = 1000;
/// How often a running program writes its sample and its counts: five minutes, or
/// `FM_DIARY_EVERY` seconds (the release checks, which cannot wait five minutes to see one).
pub fn every() -> Duration {
    std::env::var("FM_DIARY_EVERY")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s >= 1)
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(300))
}

#[derive(Default, Clone, Copy)]
struct Tally {
    n: u64,
    ms: u64,
    max_ms: u64,
    errors: u64,
}

pub struct Diary {
    dir: PathBuf,
    device: &'static str,
    on: AtomicBool,
    tallies: Mutex<BTreeMap<String, Tally>>,
    write: Mutex<()>,
}

static DIARY: OnceLock<Diary> = OnceLock::new();

/// Where this device keeps its diary.
pub fn dir() -> Option<PathBuf> {
    crate::vaults::config_dir().map(|d| d.join("formicaria").join("diary"))
}

/// Opened once per program, for `device` ("computer", "phone").
pub fn init(device: &'static str) -> Option<&'static Diary> {
    let d = dir()?;
    Some(DIARY.get_or_init(|| Diary::open(d, device)))
}

/// The diary, if this program opened one.
pub fn get() -> Option<&'static Diary> {
    DIARY.get()
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn day_of(ms: u64) -> String {
    let t = time::OffsetDateTime::from_unix_timestamp((ms / 1000) as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let d = t.date();
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

impl Diary {
    /// The diary in `dir`, on unless its settings say off.
    pub fn open(dir: PathBuf, device: &'static str) -> Diary {
        let _ = std::fs::create_dir_all(&dir);
        let on = std::fs::read(dir.join("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v.get("on").and_then(Value::as_bool))
            .unwrap_or(true);
        Diary {
            dir,
            device,
            on: AtomicBool::new(on),
            tallies: Mutex::new(BTreeMap::new()),
            write: Mutex::new(()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    /// The switch. Off: nothing more is written (what is there stays until *Delete it*).
    pub fn set_on(&self, on: bool) {
        self.on.store(on, Ordering::Relaxed);
        let _ = std::fs::write(self.dir.join("settings.json"), json!({ "on": on }).to_string());
        if !on {
            if let Ok(mut t) = self.tallies.lock() {
                t.clear();
            }
        }
    }

    /// One line: `kind`, this device, the time, and `fields` (an object).
    pub fn record(&self, kind: &str, fields: Value) {
        if !self.is_on() {
            return;
        }
        let t = now_ms();
        let mut line = Map::new();
        line.insert("t".into(), json!(t));
        line.insert("dev".into(), json!(self.device));
        line.insert("kind".into(), json!(kind));
        if let Value::Object(f) = fields {
            for (k, v) in f {
                line.insert(k, v);
            }
        }
        let _one = self.write.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(format!("{}.jsonl", day_of(t))))
        {
            let _ = writeln!(f, "{}", Value::Object(line));
        }
    }

    /// A command answered: counted by its name, timed; a slow one on its own line.
    pub fn command(&self, cmd: &str, took: Duration, ok: bool) {
        if !self.is_on() {
            return;
        }
        let ms = took.as_millis() as u64;
        if let Ok(mut t) = self.tallies.lock() {
            let e = t.entry(cmd.to_string()).or_default();
            e.n += 1;
            e.ms += ms;
            e.max_ms = e.max_ms.max(ms);
            e.errors += u64::from(!ok);
        }
        if ms >= SLOW_MS {
            self.record("slow", json!({ "cmd": cmd, "ms": ms, "ok": ok }));
        }
    }

    /// The counts since the last time, as one `use` line.
    pub fn flush(&self) {
        let taken = match self.tallies.lock() {
            Ok(mut t) => std::mem::take(&mut *t),
            Err(_) => return,
        };
        if taken.is_empty() {
            return;
        }
        let commands: Map<String, Value> = taken
            .into_iter()
            .map(|(k, t)| {
                (k, json!({ "n": t.n, "ms": t.ms, "max_ms": t.max_ms, "errors": t.errors }))
            })
            .collect();
        self.record("use", json!({ "commands": commands }));
    }

    /// This program started: said, and **how the last one ended** if it left without saying.
    pub fn started(&self, version: &str, fields: Value) {
        let marker = self.dir.join("running");
        if let Some(prev) =
            std::fs::read(&marker).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        {
            self.record(
                "ended_unsaid",
                json!({ "since": prev.get("t"), "version": prev.get("version") }),
            );
        }
        let _ = std::fs::write(&marker, json!({ "t": now_ms(), "version": version }).to_string());
        let mut f = match fields {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        f.insert("version".into(), json!(version));
        self.record("start", Value::Object(f));
        self.prune();
    }

    /// This program is leaving, and why.
    pub fn stopped(&self, reason: &str) {
        self.flush();
        self.record("stop", json!({ "reason": reason }));
        let _ = std::fs::remove_file(self.dir.join("running"));
    }

    fn days(&self) -> Vec<(String, PathBuf, u64)> {
        let mut out: Vec<(String, PathBuf, u64)> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let day = name.strip_suffix(".jsonl")?.to_string();
                (day.len() == 10)
                    .then(|| (day, e.path(), e.metadata().map(|m| m.len()).unwrap_or(0)))
            })
            .collect();
        out.sort();
        out
    }

    /// Days older than `KEEP_DAYS` go, then the oldest until it fits `MAX_BYTES`.
    pub fn prune(&self) {
        self.prune_at(now_ms(), MAX_BYTES);
    }

    fn prune_at(&self, now: u64, max: u64) {
        let oldest = day_of(now.saturating_sub(KEEP_DAYS as u64 * 86_400_000));
        let mut days = self.days();
        days.retain(|(d, p, _)| {
            let keep = d.as_str() >= oldest.as_str();
            if !keep {
                let _ = std::fs::remove_file(p);
            }
            keep
        });
        let mut total: u64 = days.iter().map(|(_, _, n)| n).sum();
        for (_, p, n) in &days {
            if total <= max {
                break;
            }
            let _ = std::fs::remove_file(p);
            total -= n;
        }
    }

    /// *Delete it*: every line, and the browser's own output. The switch stays as it is.
    pub fn forget(&self) {
        let _one = self.write.lock().unwrap_or_else(|e| e.into_inner());
        for (_, p, _) in self.days() {
            let _ = std::fs::remove_file(p);
        }
        for f in ["browser.log", "browser.log.1", "exported"] {
            let _ = std::fs::remove_file(self.dir.join(f));
        }
        if let Ok(mut t) = self.tallies.lock() {
            t.clear();
        }
    }

    /// For Settings: on or off, how much is kept, since when, where.
    pub fn status(&self) -> Value {
        let days = self.days();
        json!({
            "on": self.is_on(),
            "bytes": days.iter().map(|(_, _, n)| n).sum::<u64>(),
            "days": days.len(),
            "since": days.first().map(|(d, _, _)| d.clone()),
            "dir": self.dir.display().to_string(),
            "keep_days": KEEP_DAYS,
        })
    }

    /// **A phone's diary, carried with the notes** to the person's computer: the lines written since
    /// the last time, added to `<vault>/.formicaria/usage/<device>-<month>.jsonl`. The path written,
    /// for the store to commit with the notes (`seed_written`), or `None` if there was nothing new.
    pub fn export_to(&self, vault: &Path) -> Option<PathBuf> {
        let cursor = self.dir.join("exported");
        let since: u64 =
            std::fs::read_to_string(&cursor).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        let mut lines = Vec::new();
        let mut last = since;
        for (_, p, _) in self.days() {
            for l in std::fs::read_to_string(&p).unwrap_or_default().lines() {
                let t = serde_json::from_str::<Value>(l)
                    .ok()
                    .and_then(|v| v.get("t").and_then(Value::as_u64))
                    .unwrap_or(0);
                if t > since {
                    lines.push(l.to_string());
                    last = last.max(t);
                }
            }
        }
        if lines.is_empty() {
            return None;
        }
        let dir = vault.join(".formicaria").join("usage");
        std::fs::create_dir_all(&dir).ok()?;
        let path = dir.join(format!("{}-{}.jsonl", self.device, &day_of(last)[..7]));
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()?;
        for l in &lines {
            writeln!(f, "{l}").ok()?;
        }
        std::fs::write(&cursor, last.to_string()).ok()?;
        Some(path)
    }
}

// ── What the pages tell the diary ──────────────────────────────────────────────────────────────────

/// Text that might name a person's things, made safe: anything quoted, an address, a path, an email,
/// a long number, taken out; short.
pub fn clean(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if matches!(c, '"' | '\'' | '`' | '“' | '‘' | '«') => {
                quote = Some(match c {
                    '“' => '”',
                    '‘' => '’',
                    '«' => '»',
                    other => other,
                });
                out.push('…');
            }
            None => out.push(c),
        }
    }
    let words: Vec<String> = out
        .split_whitespace()
        .map(|w| {
            let digits = w.chars().filter(char::is_ascii_digit).count();
            if w.contains("://")
                || w.starts_with('/')
                || w.contains('\\')
                || w.contains('@')
                || w.contains("www.")
            {
                "…".to_string()
            } else if digits >= 4 {
                "#".to_string()
            } else {
                w.to_string()
            }
        })
        .collect();
    words.join(" ").chars().take(max).collect()
}

fn name(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    (!s.is_empty()
        && s.len() <= 40
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '/' | '.')))
    .then(|| s.to_string())
}

fn number(v: Option<&Value>) -> Option<Value> {
    let n = v?.as_f64()?;
    (n.is_finite() && (0.0..1e9).contains(&n)).then(|| json!((n * 10.0).round() / 10.0))
}

/// **What a page may tell the diary**: a short list of kinds, each with its own fields, names held
/// to plain words and errors cleaned. Anything else is dropped. The lines kept, as `(kind, fields)`.
pub fn from_pages(events: &Value) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    for e in events.as_array().into_iter().flatten().take(200) {
        let kind = e.get("kind").and_then(Value::as_str).unwrap_or("");
        let mut f = Map::new();
        let kind: &'static str = match kind {
            // A formicaria page opened: its kind ("Food", "note", "settings"), how long it took.
            "page" => {
                let Some(p) = name(e.get("page")) else { continue };
                f.insert("page".into(), json!(p));
                if let Some(ms) = number(e.get("ms")) {
                    f.insert("ms".into(), ms);
                }
                "page"
            }
            // Something went wrong in a page.
            "error" => {
                let Some(m) = e.get("message").and_then(Value::as_str) else { continue };
                f.insert("message".into(), json!(clean(m, 160)));
                if let Some(w) = name(e.get("where")) {
                    f.insert("where".into(), json!(w));
                }
                "error"
            }
            // formicaria's browser: started (how long its session took), quitting (why), its
            // windows and tabs.
            "browser" => {
                let Some(what) = name(e.get("what")) else { continue };
                f.insert("what".into(), json!(what));
                for k in ["ms", "tabs", "windows"] {
                    if let Some(n) = number(e.get(k)) {
                        f.insert(k.into(), n);
                    }
                }
                "browser"
            }
            // A gesture of the browser's add-on or a page's, by name ("bring", "snip", "save-media").
            "act" => {
                let Some(what) = name(e.get("what")) else { continue };
                f.insert("what".into(), json!(what));
                if let Some(ms) = number(e.get("ms")) {
                    f.insert("ms".into(), ms);
                }
                "act"
            }
            _ => continue,
        };
        out.push((kind, Value::Object(f)));
    }
    out
}

// ── Memory and processor, from the system ──────────────────────────────────────────────────────────

/// A process, as the system tells it (Linux and Android).
#[derive(Debug, Clone)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub exe: Option<PathBuf>,
    /// Processor time used so far, in clock ticks (100 a second on Linux and Android).
    pub ticks: u64,
}

/// Every process this program may read.
pub fn processes() -> Vec<Proc> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/proc") else { return out };
    for e in rd.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else { continue };
        // `pid (name) state ppid … utime stime`: fields after the name's closing parenthesis.
        let Some(rest) = stat.rsplit_once(')').map(|(_, r)| r) else { continue };
        let f: Vec<&str> = rest.split_whitespace().collect();
        let num = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        out.push(Proc {
            pid,
            ppid: num(1) as u32,
            exe: std::fs::read_link(format!("/proc/{pid}/exe")).ok(),
            ticks: num(11) + num(12),
        });
    }
    out
}

/// A process's proportional memory (its share of what it holds with others), in kB.
pub fn pss_kb(pid: u32) -> Option<u64> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    s.lines()
        .find_map(|l| l.strip_prefix("Pss:"))
        .and_then(|v| v.split_whitespace().next()?.parse().ok())
}

/// `root` and every process started under it.
pub fn descendants(all: &[Proc], root: u32) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in all {
        kids.entry(p.ppid).or_default().push(p.pid);
    }
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        if let Some(k) = kids.get(&out[i]) {
            out.extend(k.iter().copied().filter(|p| *p != root));
        }
        i += 1;
    }
    out
}

/// **One sample**: for each named part (its processes), memory and the share of one processor used
/// since the previous sample. `prev` carries each process's ticks from one sample to the next.
pub fn sample(
    parts: &[(&str, Vec<u32>)],
    all: &[Proc],
    prev: &mut HashMap<u32, (u64, u64)>,
) -> Value {
    let now = now_ms();
    let ticks: HashMap<u32, u64> = all.iter().map(|p| (p.pid, p.ticks)).collect();
    let mut out = Map::new();
    let mut next = HashMap::new();
    for (part, pids) in parts {
        if pids.is_empty() {
            continue;
        }
        let mut kb = 0u64;
        let mut used = 0f64;
        for pid in pids {
            kb += pss_kb(*pid).unwrap_or(0);
            let t = ticks.get(pid).copied().unwrap_or(0);
            if let Some((t0, at)) = prev.get(pid) {
                let secs = (now.saturating_sub(*at)) as f64 / 1000.0;
                if secs > 0.0 {
                    used += (t.saturating_sub(*t0)) as f64 / 100.0 / secs;
                }
            }
            next.insert(*pid, (t, now));
        }
        out.insert(
            (*part).to_string(),
            json!({ "mb": kb / 1024, "cpu_pct": (used * 1000.0).round() / 10.0, "procs": pids.len() }),
        );
    }
    *prev = next;
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fm-diary-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn lines(d: &Diary) -> Vec<Value> {
        d.days()
            .iter()
            .flat_map(|(_, p, _)| {
                std::fs::read_to_string(p)
                    .unwrap_or_default()
                    .lines()
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect::<Vec<Value>>()
            })
            .collect()
    }

    #[test]
    fn commands_are_counted_by_name_and_timed_and_a_slow_one_has_its_own_line() {
        let d = Diary::open(temp("count"), "computer");
        d.command("list_notes", Duration::from_millis(20), true);
        d.command("list_notes", Duration::from_millis(40), true);
        d.command("search", Duration::from_millis(1500), false);
        d.flush();
        let l = lines(&d);
        let slow = l.iter().find(|v| v["kind"] == "slow").expect("a slow line");
        assert_eq!(slow["cmd"], "search");
        let used = l.iter().find(|v| v["kind"] == "use").expect("a use line");
        assert_eq!(used["commands"]["list_notes"]["n"], 2);
        assert_eq!(used["commands"]["list_notes"]["ms"], 60);
        assert_eq!(used["commands"]["list_notes"]["max_ms"], 40);
        assert_eq!(used["commands"]["search"]["errors"], 1);
        assert_eq!(used["dev"], "computer");
        d.flush();
        assert_eq!(lines(&d).iter().filter(|v| v["kind"] == "use").count(), 1, "counted once");
    }

    #[test]
    fn off_writes_nothing_and_stays_off_and_delete_removes_every_line() {
        let dir = temp("off");
        let d = Diary::open(dir.clone(), "computer");
        d.record("start", json!({}));
        assert_eq!(lines(&d).len(), 1);
        d.set_on(false);
        d.record("start", json!({}));
        d.command("x", Duration::from_secs(3), true);
        assert_eq!(lines(&d).len(), 1, "nothing written while off");
        assert!(!Diary::open(dir, "computer").is_on(), "off is remembered");
        d.forget();
        assert!(lines(&d).is_empty());
        assert_eq!(d.status()["bytes"], 0);
    }

    #[test]
    fn a_run_that_left_without_saying_is_noticed_at_the_next_start() {
        let dir = temp("unsaid");
        let d = Diary::open(dir.clone(), "computer");
        d.started("v1", json!({}));
        // No `stopped`: killed, crashed, the power gone.
        let again = Diary::open(dir, "computer");
        again.started("v2", json!({}));
        again.stopped("closed");
        let l = lines(&again);
        assert_eq!(l.iter().filter(|v| v["kind"] == "ended_unsaid").count(), 1);
        assert_eq!(l.iter().find(|v| v["kind"] == "ended_unsaid").unwrap()["version"], "v1");
        let third = Diary::open(again.dir().to_path_buf(), "computer");
        third.started("v2", json!({}));
        assert_eq!(
            lines(&third).iter().filter(|v| v["kind"] == "ended_unsaid").count(),
            1,
            "a said stop is not"
        );
    }

    #[test]
    fn old_days_and_the_oldest_beyond_the_size_go() {
        let d = Diary::open(temp("prune"), "computer");
        let now = 100 * 86_400_000u64 + 5;
        for back in [0u64, 1, 2, 95] {
            let day = day_of(now - back * 86_400_000);
            std::fs::write(d.dir().join(format!("{day}.jsonl")), "x".repeat(100)).unwrap();
        }
        d.prune_at(now, 250);
        let kept: Vec<String> = d.days().into_iter().map(|(n, _, _)| n).collect();
        assert_eq!(
            kept,
            vec![day_of(now - 86_400_000), day_of(now)],
            "95 days old, then the oldest"
        );
    }

    #[test]
    fn what_could_name_a_persons_things_is_taken_out() {
        let m = clean("could not open \"Diary of May\" at https://example.com/a?b=c in /home/x/notes for me@example.com id 12345678", 200);
        assert!(!m.contains("Diary of May") && !m.contains("example.com") && !m.contains("/home"));
        assert!(!m.contains("12345678") && !m.contains('@'));
        assert!(m.starts_with("could not open"));
        assert!(clean(&"word ".repeat(100), 30).chars().count() <= 30);
    }

    #[test]
    fn pages_may_tell_only_a_few_kinds_of_thing() {
        let kept = from_pages(&json!([
            { "kind": "page", "page": "Food", "ms": 312.44 },
            { "kind": "page", "page": "https://bank.example/account" },
            { "kind": "page", "page": "Food", "title": "My secret note" },
            { "kind": "error", "message": "TypeError: x is \"Mum's birthday\"", "where": "ui" },
            { "kind": "browser", "what": "quit", "windows": 0, "tabs": 1, "url": "https://x.example" },
            { "kind": "note", "body": "anything" },
        ]));
        let kinds: Vec<&str> = kept.iter().map(|(k, _)| *k).collect();
        assert_eq!(kinds, vec!["page", "page", "error", "browser"]);
        assert_eq!(kept[0].1, json!({ "page": "Food", "ms": 312.4 }));
        assert_eq!(kept[1].1, json!({ "page": "Food" }), "only the fields it may have");
        assert!(!kept[2].1["message"].as_str().unwrap().contains("Mum"));
        assert!(kept[3].1.get("url").is_none());
    }

    #[test]
    fn a_phones_new_lines_go_with_the_notes_once() {
        let d = Diary::open(temp("export"), "phone");
        let vault = temp("export-vault");
        d.record("start", json!({}));
        d.record("sample", json!({}));
        let p = d.export_to(&vault).expect("written");
        assert!(p.starts_with(vault.join(".formicaria").join("usage")));
        assert!(p.file_name().unwrap().to_string_lossy().starts_with("phone-"));
        assert_eq!(std::fs::read_to_string(&p).unwrap().lines().count(), 2);
        assert!(d.export_to(&vault).is_none(), "nothing new");
        std::thread::sleep(Duration::from_millis(3));
        d.record("stop", json!({}));
        d.export_to(&vault).expect("the new line");
        assert_eq!(std::fs::read_to_string(&p).unwrap().lines().count(), 3);
    }

    #[test]
    fn a_sample_reads_this_process() {
        if !Path::new("/proc/self/stat").exists() {
            return;
        }
        let all = processes();
        let me = std::process::id();
        assert!(all.iter().any(|p| p.pid == me));
        let mut prev = HashMap::new();
        let s = sample(&[("server", vec![me])], &all, &mut prev);
        assert!(s["server"]["mb"].as_u64().unwrap() > 0);
        assert_eq!(s["server"]["procs"], 1);
        assert!(descendants(&all, me).contains(&me));
    }
}
