//! **A watch's activity files, read into the training log** (`decisions.md` 2026-11-20; the coach
//! plan's Spec S, phase S1) — through `dispatch`, the way the panel reaches it.
//!
//! Every file here is synthetic: written by `fm-core/src/fit_synth.rs` with invented values and
//! invented serial numbers, into a temporary folder. No real person's file is read.
//!
//! What is pinned (the plan's test numbers):
//! - **T7** importing twice adds nothing and makes no second commit; the same identity with other
//!   bytes is reported and nothing is overwritten; a near-duplicate is a proposal; a damaged file is
//!   refused and named with what to do; the source folder is untouched.
//! - **T8** `day` is the watch's own date — over midnight, across a daylight-saving change, on a
//!   half-hour offset — and the same vault under two app clocks gets the same days.
//! - **T10** no serial number in clear in any note.
//! - **T11** every vault is a temporary one, asserted before anything is written.
//! - **T13** a pool swim: intervals, `mixed_stroke`, heart rate only as the watch's.
//! - **T14** strength: one set note per active set, rests ignored, an unmapped exercise kept as text.
//! - **T15** an edited set wins: a re-import leaves it, and makes no commit.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::zone::Zone;
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::{tempdir, TempDir};

#[path = "../../fm-core/src/fit_synth.rs"]
mod synth;
use synth::*;

/// One test at a time: the vaults are temporary, but `git` and the clock are process-wide habits
/// worth not interleaving (T11).
static LOCK: Mutex<()> = Mutex::new(());

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn commits(vault: &Path) -> usize {
    git(vault, &["rev-list", "--count", "HEAD"]).trim().parse().unwrap_or(0)
}

struct World {
    _home: TempDir,
    vault: TempDir,
    app: App,
}

/// A git vault in a temporary folder, on a chosen app clock.
fn world(zone: &str) -> World {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    // T11: the vault is a temporary folder, never a real one.
    assert!(vault.path().starts_with(std::env::temp_dir()), "{}", vault.path().display());
    let v = vault.path();
    std::fs::create_dir_all(v.join("notes")).unwrap();
    git(v, &["init", "-q", "-b", "main"]);
    git(v, &["config", "user.name", "Tester"]);
    git(v, &["config", "user.email", "tester@example.com"]);
    std::fs::write(v.join("README.md"), "test vault\n").unwrap();
    git(v, &["add", "-A"]);
    git(v, &["commit", "-q", "-m", "first"]);
    let store = MultiStore::open(&[("notes".to_string(), v.to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: v.to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    app.clock().set(Some(Zone::parse(zone).unwrap()));
    World { _home: home, vault, app }
}

fn call(app: &App, cmd: &str, args: Value) -> Result<Value, String> {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
}

fn preview(w: &World, folder: &Path) -> Value {
    call(&w.app, "fit_preview", json!({ "path": folder, "vault": "notes" })).unwrap()
}

fn import(w: &World, folder: &Path) -> Value {
    call(&w.app, "import_fit", json!({ "path": folder, "vault": "notes" })).unwrap()
}

/// A folder of watch files: `(name, spec)` written as `.fit`.
fn folder(files: &[(&str, Vec<u8>)]) -> TempDir {
    let d = tempdir().unwrap();
    for (name, bytes) in files {
        std::fs::write(d.path().join(name), bytes).unwrap();
    }
    d
}

/// Every note in the vault, as it is on disk.
fn notes(vault: &Path) -> Vec<fm_model::Object> {
    let mut out: Vec<_> = std::fs::read_dir(vault.join("notes"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .map(|e| {
            fm_core::frontmatter::from_file(&std::fs::read_to_string(e.path()).unwrap()).unwrap()
        })
        .collect();
    out.sort_by_key(|o| o.id.to_string());
    out
}

fn prop(o: &fm_model::Object, k: &str) -> String {
    o.get(k).display()
}

fn activities(vault: &Path) -> Vec<fm_model::Object> {
    notes(vault).into_iter().filter(|o| !o.get("activity").is_null()).collect()
}

fn sets(vault: &Path) -> Vec<fm_model::Object> {
    notes(vault).into_iter().filter(|o| !o.get("exercise").is_null()).collect()
}

/// Names, sizes and contents of everything under a folder.
fn fingerprint(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        out.insert(e.path(), std::fs::read(e.path()).unwrap());
    }
    out
}

fn damaged() -> Vec<u8> {
    let mut b = steady_run().encode();
    let n = b.len();
    b[n - 1] ^= 0x55;
    b
}

#[test]
fn t7_importing_twice_adds_nothing_and_the_folder_is_only_read() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let src = folder(&[
        ("run.fit", steady_run().encode()),
        ("intervals.fit", intervals().encode()),
        ("swim.fit", pool_swim().encode()),
        ("strength.fit", strength().encode()),
        ("broken.fit", damaged()),
        ("readme.txt", b"not a watch file".to_vec()),
    ]);
    let before = fingerprint(src.path());
    let c0 = commits(w.vault.path());

    let p = preview(&w, src.path());
    assert_eq!(p["files"], 5, "{p}");
    assert_eq!(p["activities"], 4, "{p}");
    assert_eq!(p["sets"], 3, "{p}");
    assert_eq!(p["ok"], true);
    // The preview wrote nothing: no note, no salt, no commit.
    assert!(notes(w.vault.path()).is_empty());
    assert!(!w.vault.path().join(".formicaria/fit-salt").exists());
    assert_eq!(commits(w.vault.path()), c0);

    let r = import(&w, src.path());
    assert_eq!(r["written"], 7, "{r}");
    assert_eq!(r["recorded"], true, "{r}");
    assert_eq!(commits(w.vault.path()), c0 + 1, "one commit for the whole import");
    // Refused, named, and told what to do.
    let refused = r["refused"].as_array().unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0]["file"], "broken.fit");
    let why = refused[0]["reason"].as_str().unwrap();
    assert!(why.contains("checksum") && why.contains("export the activity again"), "{why}");
    // The salt went into the same commit, so it travels with the notes.
    assert!(git(w.vault.path(), &["ls-files"]).contains(".formicaria/fit-salt"));
    assert_eq!(activities(w.vault.path()).len(), 4);
    assert_eq!(sets(w.vault.path()).len(), 3);

    // Again: nothing written, no commit.
    let r2 = import(&w, src.path());
    assert_eq!(r2["written"], 0, "{r2}");
    assert_eq!(r2["known"], 4, "{r2}");
    assert_eq!(commits(w.vault.path()), c0 + 1);
    assert_eq!(notes(w.vault.path()).len(), 7);

    // I4: the folder is exactly as it was.
    assert_eq!(fingerprint(src.path()), before);
}

#[test]
fn t7_the_same_file_with_other_bytes_is_reported_never_overwritten() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let first = folder(&[("run.fit", steady_run().encode())]);
    import(&w, first.path());
    let before = notes(w.vault.path());
    let c = commits(w.vault.path());

    // Same `file_id` (serial, time created), different contents.
    let mut other = steady_run();
    other.distance_m = Some(3100);
    let second = folder(&[("run-again.fit", other.encode())]);
    let p = preview(&w, second.path());
    assert_eq!(p["otherVersions"].as_array().unwrap().len(), 1, "{p}");
    assert_eq!(p["otherVersions"][0]["edited"], false);
    assert_eq!(p["activities"], 0);
    let r = import(&w, second.path());
    assert_eq!(r["written"], 0, "{r}");
    assert_eq!(notes(w.vault.path()), before, "nothing overwritten");
    assert_eq!(commits(w.vault.path()), c);
}

#[test]
fn t7_a_near_duplicate_is_proposed_never_merged() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    // A copy another app regenerated: other identity, no `file_creator`, same session.
    let twin = || Spec { serial: 3_000_000_099, manufacturer: 255, creator: false, ..steady_run() };

    // In one folder: the watch's own copy is taken, the twin proposed.
    let src =
        folder(&[("a-regenerated.fit", twin().encode()), ("b-watch.fit", steady_run().encode())]);
    let r = import(&w, src.path());
    assert_eq!(r["activities"], 1, "{r}");
    let merges = r["merges"].as_array().unwrap();
    assert_eq!(merges.len(), 1, "{r}");
    assert_eq!(merges[0]["file"], "a-regenerated.fit");
    assert_eq!(activities(w.vault.path()).len(), 1);

    // Later, against what is already in the vault: a proposal that names the entry.
    let c = commits(w.vault.path());
    let later = folder(&[("twin-again.fit", Spec { serial: 3_000_000_098, ..twin() }.encode())]);
    let r = import(&w, later.path());
    assert_eq!(r["written"], 0, "{r}");
    assert_eq!(r["merges"][0]["with"], "Running", "{r}");
    assert!(r["merges"][0]["id"].is_string());
    assert_eq!(commits(w.vault.path()), c);

    // A run a person logged by hand at the same minute, for the same time, is proposed too.
    let w2 = world("+01:00");
    call(
        &w2.app,
        "create_notes",
        json!({ "vault": "notes", "notes": [{
            "title": "Running",
            "start": "2026-03-14T08:00",
            "props": { "log": "workout", "activity": "Running", "minutes": 16, "counts_as": "aerobic" }
        }]}),
    )
    .unwrap();
    let p = preview(&w2, folder(&[("run.fit", steady_run().encode())]).path());
    assert_eq!(p["merges"].as_array().unwrap().len(), 1, "{p}");
    assert_eq!(p["activities"], 0);
}

#[test]
fn t7_a_folder_with_nothing_to_import_says_so() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let empty = tempdir().unwrap();
    let p = preview(&w, empty.path());
    assert_eq!(p["ok"], false);
    assert!(p["problem"].as_str().unwrap().contains("no .fit files"), "{p}");
    let missing = empty.path().join("nope");
    let e = call(&w.app, "import_fit", json!({ "path": missing, "vault": "notes" })).unwrap_err();
    assert!(e.contains("There is no folder"), "{e}");
}

/// The run, imported on a given app clock, as `(start as the app shows it, day)`.
fn run_on(zone: &str, spec: Spec) -> (String, String) {
    let w = world(zone);
    import(&w, folder(&[("run.fit", spec.encode())]).path());
    let a = activities(w.vault.path());
    assert_eq!(a.len(), 1);
    let got = call(&w.app, "get", json!({ "id": a[0].id.to_string() })).unwrap();
    (got["start"].as_str().unwrap_or_default().to_string(), prop(&a[0], "day"))
}

#[test]
fn t8_the_day_is_the_watch_own_date() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 23:50 on the watch, on three offsets including a half hour: the watch's day, on any clock.
    for off in [3600, -5 * 3600, 5 * 3600 + 1800] {
        for clock in ["+09:00", "-07:00", "+05:30"] {
            let (_, day) = run_on(clock, over_midnight(off));
            assert_eq!(day, "2026-06-20", "watch offset {off}, app clock {clock}");
        }
    }
    // On a half-hour clock matching the watch, `start` is the watch's 23:50.
    let (start, _) = run_on("+05:30", over_midnight(5 * 3600 + 1800));
    assert_eq!(start, "2026-06-20T23:50");
}

#[test]
fn t8_daylight_saving_days() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The 23-hour day: a run at 00:30 UTC on the night the clocks go forward (01:30 there, then
    // 03:00); the watch ends it after the change, on +02:00.
    let spring = Spec {
        start: fit_time(2026, 3, 29, 0, 30, 0),
        offset_s: Some(7200),
        laps: vec![LapSpec {
            timer_s: 3600,
            distance_m: 10_000,
            hr_avg: 140,
            hr_max: 150,
            ..Default::default()
        }],
        ..Default::default()
    };
    let (start, day) = run_on("Europe/Berlin", spring.clone());
    assert_eq!((start.as_str(), day.as_str()), ("2026-03-29T01:30", "2026-03-29"));
    // The 25-hour day: 00:30 UTC is 02:30 summer time there; the watch is on +01:00 by the end.
    let autumn =
        Spec { start: fit_time(2026, 10, 25, 0, 30, 0), offset_s: Some(3600), ..spring.clone() };
    let (start, day) = run_on("Europe/Berlin", autumn);
    assert_eq!((start.as_str(), day.as_str()), ("2026-10-25T02:30", "2026-10-25"));
    // 23:50 on the evening before the 25-hour day, summer time: still that evening's day.
    let eve = Spec { start: fit_time(2026, 10, 24, 21, 50, 0), offset_s: Some(7200), ..spring };
    let (start, day) = run_on("Europe/Berlin", eve);
    assert_eq!((start.as_str(), day.as_str()), ("2026-10-24T23:50", "2026-10-24"));
}

#[test]
fn t8_one_folder_two_clocks_the_same_days() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let files = [
        ("run.fit", steady_run().encode()),
        ("intervals.fit", intervals().encode()),
        ("late.fit", over_midnight(-4 * 3600).encode()),
        ("swim.fit", pool_swim().encode()),
        ("strength.fit", strength().encode()),
    ];
    let days = |zone: &str| {
        let w = world(zone);
        import(&w, folder(&files).path());
        let mut d: Vec<(String, String)> = notes(w.vault.path())
            .iter()
            .map(|o| (o.title.clone().unwrap(), prop(o, "day")))
            .collect();
        d.sort();
        d
    };
    let east = days("+10:00");
    let west = days("-09:30");
    assert_eq!(east.len(), 8);
    assert_eq!(east, west);
}

#[test]
fn t10_no_serial_number_in_clear() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let specs = [steady_run(), pool_swim(), strength()];
    let files: Vec<(String, Vec<u8>)> =
        specs.iter().enumerate().map(|(i, s)| (format!("{i}.fit"), s.encode())).collect();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    import(&w, folder(&refs).path());
    for e in std::fs::read_dir(w.vault.path().join("notes")).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap();
        for s in &specs {
            assert!(!text.contains(&s.serial.to_string()), "a serial in clear in {text}");
            assert!(!text.contains(&format!("{:x}", s.serial)), "a serial in hex in {text}");
        }
    }
    // And the key is not a plain hash a search could reverse: another vault, another salt.
    let w2 = world("+01:00");
    import(&w2, folder(&refs).path());
    let k1 = prop(&activities(w.vault.path())[0], "source_key");
    let k2: Vec<String> =
        activities(w2.vault.path()).iter().map(|o| prop(o, "source_key")).collect();
    assert!(k1.starts_with("fit:"));
    assert!(!k2.contains(&k1), "two vaults keyed the same file alike");
}

#[test]
fn t13_a_pool_swim() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    import(&w, folder(&[("swim.fit", pool_swim().encode())]).path());
    let a = activities(w.vault.path());
    let s = &a[0];
    assert_eq!(prop(s, "activity"), "Swimming");
    assert_eq!(prop(s, "sport"), "swim");
    assert_eq!(s.get("distance_m"), fm_model::PropertyValue::Int(250));
    // Heart rate only as the watch's.
    assert!(s.get("hr_avg_bpm").is_null() && s.get("hr_max_bpm").is_null());
    assert_eq!(s.get("watch_hr_avg_bpm"), fm_model::PropertyValue::Int(128));
    let q = prop(s, "quality");
    for word in ["mixed_stroke", "drill_typed", "hr_wrist_water"] {
        assert!(q.contains(word), "{word} missing from {q}");
    }
    // Three intervals between the rests; the medley one is mixed.
    let rows: Vec<&str> =
        s.body.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| Interval")).collect();
    assert_eq!(rows.len(), 3, "{}", s.body);
    assert!(rows[0].contains("| 4 | 100 m |") && rows[0].contains("freestyle"), "{}", rows[0]);
    assert!(rows[1].contains("mixed"), "{}", rows[1]);
    assert!(rows[2].contains("drill"), "{}", rows[2]);
    assert!(s.body.contains("watch's"), "the in-water warning: {}", s.body);
}

#[test]
fn t14_strength_sets_one_note_each() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    import(&w, folder(&[("strength.fit", strength().encode())]).path());
    let a = activities(w.vault.path());
    assert_eq!(a.len(), 1);
    assert_eq!(prop(&a[0], "counts_as"), "strength");
    let mut s = sets(w.vault.path());
    s.sort_by_key(|o| (prop(o, "exercise"), prop(o, "set")));
    assert_eq!(s.len(), 3, "rests are not sets");
    // Mapped to the card the app carries, with its region.
    assert_eq!(prop(&s[0], "exercise"), "Bench Press");
    assert_eq!(prop(&s[0], "set"), "1");
    assert_eq!(prop(&s[0], "reps"), "8");
    assert_eq!(prop(&s[0], "load_g"), "60000");
    assert_eq!(prop(&s[0], "region"), "upper-push");
    assert_eq!(prop(&s[1], "set"), "2");
    // Unmapped: the watch's own name, as text.
    assert_eq!(prop(&s[2], "exercise"), "Goblet squat");
    assert!(s[2].get("region").is_null());
    for x in &s {
        assert_eq!(prop(x, "log"), "workout");
        assert!(prop(x, "quality").contains("reps_watch"));
        assert_eq!(x.start, a[0].start, "a set shares its session's start");
        assert_eq!(prop(x, "day"), prop(&a[0], "day"));
    }
}

#[test]
fn t15_an_edited_set_wins() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let src = folder(&[("strength.fit", strength().encode())]);
    import(&w, src.path());
    let set = sets(w.vault.path()).into_iter().find(|o| prop(o, "reps") == "8").unwrap();
    call(&w.app, "set_property", json!({ "id": set.id.to_string(), "key": "reps", "value": "9" }))
        .unwrap();
    call(&w.app, "commit", json!({ "vault": "notes" })).ok();
    let c = commits(w.vault.path());
    let r = import(&w, src.path());
    assert_eq!(r["written"], 0, "{r}");
    assert_eq!(commits(w.vault.path()), c, "no commit");
    let after = notes(w.vault.path()).into_iter().find(|o| o.id == set.id).unwrap();
    assert_eq!(prop(&after, "reps"), "9", "the person's edit stands");
}

#[test]
fn a_run_counts_on_the_existing_cards() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    import(&w, folder(&[("run.fit", steady_run().encode())]).path());
    let a = &activities(w.vault.path())[0];
    // The existing activity shape: what `plan.rs` reads.
    assert_eq!(prop(a, "log"), "workout");
    assert_eq!(prop(a, "activity"), "Running");
    assert_eq!(prop(a, "counts_as"), "aerobic");
    assert_eq!(prop(a, "minutes"), "16");
    assert_eq!(prop(a, "effort"), "5", "the watch's typed effort, ÷10");
    assert_eq!(prop(a, "rpe"), "5");
    assert_eq!(prop(a, "distance_m"), "3000");
    assert_eq!(prop(a, "hr_avg_bpm"), "145");
    assert_eq!(prop(a, "cadence_avg_spm"), "168");
    assert_eq!(prop(a, "watch_training_effect_x10"), "32");
    assert_eq!(prop(a, "day"), "2026-03-14");
    assert!(prop(a, "source_fit").starts_with("sha256:"));
    assert!(!prop(a, "import_hash").is_empty());
    assert!(a.body.contains("| Lap |"), "{}", a.body);
    // The file is kept beside it, as a blob the note names.
    let hex = prop(a, "source_fit").trim_start_matches("sha256:").to_string();
    let blob = fm_core::blob::BlobStore::new(w.vault.path()).path_for(&hex);
    assert_eq!(std::fs::read(blob).unwrap(), steady_run().encode());
}

// ── the phone's route (`decisions.md` 2026-11-20, *The phone takes watch files*) ────────────────

/// Upload each file as a chunked session — what the page does with a shared file or one from the
/// granted folder — then import them all in one call.
fn import_bytes(w: &World, files: &[(&str, Vec<u8>)], tag: &str) -> Value {
    let mut sent = Vec::new();
    for (i, (name, bytes)) in files.iter().enumerate() {
        let session = format!("u{tag}-{i}");
        for (seq, chunk) in bytes.chunks(1000).enumerate() {
            let args = json!({ "session": session, "seq": seq, "vault": "notes" });
            dispatch("ingest_chunk", &args, chunk, &w.app, &NoHost).unwrap();
        }
        sent.push(json!({ "session": session, "name": name }));
    }
    call(&w.app, "import_fit_files", json!({ "files": sent, "vault": "notes" })).unwrap()
}

fn assets(vault: &Path) -> usize {
    notes(vault).iter().filter(|o| o.kind == fm_model::Kind::Asset).count()
}

#[test]
fn i1_the_phone_route_imports_the_same_notes_once_and_makes_no_asset_note() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let files = [
        ("run.fit", steady_run().encode()),
        ("strength.fit", strength().encode()),
        ("broken.fit", damaged()),
    ];
    // The computer's route, for comparison.
    let desk = world("+01:00");
    import(&desk, folder(&files).path());

    let w = world("+01:00");
    let c0 = commits(w.vault.path());
    let r = import_bytes(&w, &files, "a");
    assert_eq!(r["activities"], 2, "{r}");
    assert_eq!(r["recorded"], true, "{r}");
    assert_eq!(r["refused"][0]["file"], "broken.fit", "{r}");
    assert_eq!(commits(w.vault.path()), c0 + 1, "one commit");
    assert_eq!(
        assets(w.vault.path()),
        0,
        "no asset note: the file is only the blob its entry names"
    );
    // The same notes, field for field, as the computer's import (ids, times and the salted key
    // aside: each vault has its own salt).
    let fields = |v: &Path| {
        let mut out: Vec<_> = notes(v)
            .iter()
            .map(|o| {
                let mut e: Vec<_> = o
                    .extra
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "source_key" | "import_hash"))
                    .map(|(k, v)| format!("{k}={}", v.display()))
                    .collect();
                e.sort();
                (o.title.clone(), e.join(" "), o.body.clone())
            })
            .collect();
        out.sort();
        out
    };
    assert_eq!(fields(w.vault.path()), fields(desk.vault.path()));
    // The uploads are gone once imported.
    let sessions = fm_core::chunked::sessions_dir(w.vault.path());
    assert!(std::fs::read_dir(&sessions).map(|d| d.count() == 0).unwrap_or(true));

    // Again: nothing written, no commit, still no asset note.
    let before = notes(w.vault.path()).len();
    let r2 = import_bytes(&w, &files, "b");
    assert_eq!(r2["written"], 0, "{r2}");
    assert_eq!(r2["known"], 2, "{r2}");
    assert_eq!(commits(w.vault.path()), c0 + 1);
    assert_eq!(notes(w.vault.path()).len(), before);
    assert_eq!(assets(w.vault.path()), 0);
}

#[test]
fn a_file_that_is_not_a_watch_file_is_refused_on_the_phone_route_too() {
    if !have_git() {
        return;
    }
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let r = import_bytes(&w, &[("holiday.fit", b"a picture, renamed".to_vec())], "c");
    assert_eq!(r["written"], 0, "{r}");
    assert_eq!(r["refused"][0]["file"], "holiday.fit", "{r}");
    assert!(notes(w.vault.path()).is_empty());
    // A session that was never sent is named, not imported.
    let r = call(
        &w.app,
        "import_fit_files",
        json!({ "files": [{ "session": "unever", "name": "x.fit" }], "vault": "notes" }),
    )
    .unwrap();
    assert_eq!(r["refused"].as_array().unwrap().len(), 1, "{r}");
}

#[test]
fn a_planned_session_comes_back_as_a_workout_file() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world("+01:00");
    let made = call(
        &w.app,
        "create_notes",
        json!({ "vault": "notes", "notes": [{
            "title": "Intervals · Run",
            "start": "2031-03-14",
            "body": "- Warm-up: 10 min easy\n- Work: 5 x 3 min at 150-160 bpm\n- Recovery: 2 min\n",
            "props": { "log": "workout", "planned": 1, "sport": "run", "activity": "Running" }
        }]}),
    )
    .unwrap();
    let id = made[0]["id"].as_str().unwrap().to_string();
    let f = call(&w.app, "workout_fit", json!({ "id": id })).unwrap();
    assert_eq!(f["name"], "2031-03-14-intervals-run.fit", "{f}");
    assert_eq!(f["steps"], 4, "{f}");
    let bytes = fm_app::wire::b64_decode(f["data"].as_str().unwrap()).unwrap();
    let back = fm_core::fit::decode(&bytes).unwrap();
    assert_eq!(back.steps[1].target_low, Some(250), "150 bpm + the protocol's 100");
    // Nothing was written.
    assert_eq!(notes(w.vault.path()).len(), 1);
}
