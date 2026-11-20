//! **Fitness → Coach** through `dispatch` (`decisions.md` 2026-11-20, *Fitness gets a Coach tab*;
//! the coach plan's Spec K and S.5). Invented entries in temporary vaults; nothing real is read.
//!
//! - the plan in force and the Coach's own calendar: training, the Menu's planned meals and
//!   meetings, each on its day; a stopped plan is not in force;
//! - progress: planned and done sessions and minutes per sport and week, skipped counted apart;
//! - **T16** matching: one planned run that day → one proposal; accepting keeps `planned_minutes`,
//!   folds the imported note in (its `source_key` and body kept) and records one step; two
//!   candidates → no proposal, listed; a different kind of session → nothing.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use tempfile::{tempdir, TempDir};

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

struct World {
    _home: TempDir,
    vault: TempDir,
    app: App,
}

fn world() -> World {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    assert!(vault.path().starts_with(std::env::temp_dir()));
    let v = vault.path();
    std::fs::create_dir_all(v.join("notes")).unwrap();
    if have_git() {
        git(v, &["init", "-q", "-b", "main"]);
        git(v, &["config", "user.name", "Tester"]);
        git(v, &["config", "user.email", "tester@example.com"]);
        std::fs::write(v.join("README.md"), "test vault\n").unwrap();
        git(v, &["add", "-A"]);
        git(v, &["commit", "-q", "-m", "first"]);
    }
    let store = MultiStore::open(&[("notes".to_string(), v.to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: v.to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    World { _home: home, vault, app }
}

fn call(app: &App, cmd: &str, args: Value) -> Result<Value, String> {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
}

/// Notes written as the Coach and the import write them; answers their ids.
fn make(w: &World, notes: Value) -> Vec<String> {
    let made = call(&w.app, "create_notes", json!({ "vault": "notes", "notes": notes })).unwrap();
    made.as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap().to_string()).collect()
}

fn get(w: &World, id: &str) -> Value {
    call(&w.app, "get", json!({ "id": id })).unwrap()
}

/// A planned run of a followed plan, as "Put this week on my calendar" writes it.
fn planned_run(day: &str, program: &str, minutes: i64) -> Value {
    json!({ "title": "Easy run", "start": day, "body": "- Easy running",
        "props": { "log": "workout", "planned": 1, "activity": "Running", "sport": "run",
            "counts_as": "aerobic", "type": "easy", "plan": program, "plan_week": 1,
            "session": "easy", "minutes": minutes } })
}

/// An activity as the watch import writes one (its key invented, its body a table).
fn imported_run(day: &str, minutes: i64) -> Value {
    json!({ "title": "Running", "start": format!("{day}T07:05"),
        "body": "| Lap | Time |\n|---|---|\n| 1 | 10:00 |",
        "props": { "log": "workout", "activity": "Running", "sport": "run", "counts_as": "aerobic",
            "minutes": minutes, "distance_m": 6100, "day": day,
            "source_key": format!("fit:invented-{day}-{minutes}"),
            "source_fit": "sha256:0000000000000000000000000000000000000000000000000000000000000000" } })
}

#[test]
fn the_plan_in_force_and_the_coach_s_own_calendar() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world();
    let ids = make(
        &w,
        json!([
            { "title": "10 km to a half marathon", "start": "2026-11-02",
              "props": { "program": "coach", "plan": "ten-k-half", "question": "goal",
                         "goal": "A half marathon", "week": 3, "event_date": "2027-01-10" } },
            { "title": "An old plan", "start": "2026-10-01", "due": "2026-10-31",
              "props": { "program": "coach", "plan": "hybrid-week", "question": "goal", "week": 1 } },
        ]),
    );
    // Stopped: a last day before the day asked about.
    call(&w.app, "set_properties", json!({ "id": ids[1], "props": { "due": "2026-10-31" } }))
        .unwrap();
    make(
        &w,
        json!([
            planned_run("2026-11-17", &ids[0], 40),
            { "title": "Oats", "start": "2026-11-17", "props": { "log": "food", "planned": 1, "slot": "breakfast" } },
            { "title": "Toast", "start": "2026-11-17", "props": { "log": "food", "slot": "breakfast" } },
            { "title": "Team meeting", "start": "2026-11-18T10:00", "tags": ["meeting"], "props": {} },
            { "title": "A note", "start": "2026-11-18", "props": {} },
        ]),
    );
    let a = call(
        &w.app,
        "coach",
        json!({ "day": "2026-11-20", "from": "2026-11-16", "to": "2026-11-22" }),
    )
    .unwrap();
    let followed = a["followed"].as_array().unwrap();
    assert_eq!(followed.len(), 1, "a stopped plan is not in force: {followed:?}");
    assert_eq!(followed[0]["week"], 3);
    assert_eq!(followed[0]["plan"]["id"], "ten-k-half");
    assert_eq!(followed[0]["event_date"], "2027-01-10");
    let days = a["days"].as_array().unwrap();
    assert_eq!(days.len(), 7);
    assert_eq!(days[1]["day"], "2026-11-17");
    assert_eq!(days[1]["training"].as_array().unwrap().len(), 1);
    let meals = days[1]["meals"].as_array().unwrap();
    assert_eq!(meals.len(), 1, "only the Menu's planned meals: {meals:?}");
    assert_eq!(meals[0]["title"], "Oats");
    let meetings = days[2]["meetings"].as_array().unwrap();
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0]["title"], "Team meeting");
    // A span longer than a month view is refused.
    assert!(call(
        &w.app,
        "coach",
        json!({ "day": "2026-11-20", "from": "2026-01-01", "to": "2026-12-31" })
    )
    .is_err());
}

#[test]
fn progress_counts_planned_done_and_skipped_per_sport() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world();
    let ids = make(
        &w,
        json!([
            planned_run("2026-11-16", "P", 30),
            planned_run("2026-11-18", "P", 45),
            planned_run("2026-11-20", "P", 50),
            { "title": "Squat", "start": "2026-11-19", "props": { "log": "workout", "exercise": "Squat", "reps": 5, "load_g": 60000 } },
            { "title": "Squat", "start": "2026-11-19", "props": { "log": "workout", "exercise": "Squat", "reps": 5, "load_g": 60000 } },
        ]),
    );
    // Done as planned, but longer: what was planned is kept (C3), the done minutes written.
    call(
        &w.app,
        "set_properties",
        json!({ "id": ids[0], "props": { "planned": "", "planned_minutes": 30, "minutes": 35, "distance_m": 5000 } }),
    )
    .unwrap();
    call(&w.app, "set_properties", json!({ "id": ids[1], "props": { "skipped": 1 } })).unwrap();
    let p = call(&w.app, "coach_progress", json!({ "day": "2026-11-20", "weeks": 2 })).unwrap();
    let weeks = p.as_array().unwrap();
    assert_eq!(weeks.len(), 2);
    assert_eq!(weeks[1]["from"], "2026-11-16");
    let sport = |name: &str| {
        weeks[1]["sports"].as_array().unwrap().iter().find(|s| s["sport"] == name).cloned().unwrap()
    };
    let run = sport("run");
    assert_eq!(run["planned"], 3);
    assert_eq!(run["done"], 1);
    assert_eq!(run["skipped"], 1);
    assert_eq!(run["planned_minutes"], 30 + 45 + 50);
    assert_eq!(run["done_minutes"], 35);
    assert_eq!(run["distance_m"], 5000);
    let strength = sport("strength");
    assert_eq!(strength["done"], 1, "a day of sets is one session");
    assert!(weeks[0]["sports"].as_array().unwrap().is_empty());
}

/// **T16.**
#[test]
fn one_planned_run_that_day_is_one_proposal_and_accepting_keeps_what_was_planned() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world();
    let ids = make(&w, json!([planned_run("2026-11-17", "P", 40), imported_run("2026-11-17", 43)]));
    let (planned, imported) = (&ids[0], &ids[1]);
    let before = git(w.vault.path(), &["rev-list", "--count", "HEAD"]);

    let m = call(&w.app, "match_planned", json!({})).unwrap();
    let proposals = m["proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 1, "{m}");
    assert_eq!(proposals[0]["planned"]["id"], planned.as_str());
    assert_eq!(proposals[0]["imported"]["id"], imported.as_str());
    // Nothing is done until the person says so.
    assert_eq!(get(&w, planned)["props"]["planned"], 1);

    let f =
        call(&w.app, "accept_match", json!({ "planned": planned, "imported": imported })).unwrap();
    let n = get(&w, planned);
    assert_eq!(n["props"]["planned_minutes"], 40, "what was planned is kept");
    assert_eq!(n["props"]["minutes"], 43, "the done value is the watch's");
    assert!(n["props"]["planned"].is_null(), "no longer planned: {n}");
    assert_eq!(n["props"]["plan"], "P", "still linked to its plan");
    assert_eq!(n["props"]["session"], "easy");
    assert!(n["props"]["source_key"].as_str().unwrap().starts_with("fit:"));
    assert!(n["props"]["source_fit"].as_str().unwrap().starts_with("sha256:"));
    assert!(n["body"].as_str().unwrap().contains("| Lap |"), "the table moved across");
    assert!(n["body"].as_str().unwrap().contains("Easy running"), "the steps stay");
    assert!(call(&w.app, "get", json!({ "id": imported })).map_or(true, |v| v.is_null()));
    // What undo needs: the values before, and the imported note as it was.
    assert_eq!(f["previous"]["planned"], "1");
    assert!(f["previous"]["planned_minutes"].is_null());
    assert!(f["removed"]["file"].as_str().unwrap().contains("source_key"));
    if have_git() {
        assert_eq!(f["recorded"], true);
        let after = git(w.vault.path(), &["rev-list", "--count", "HEAD"]);
        assert_eq!(
            after.trim().parse::<i64>().unwrap(),
            before.trim().parse::<i64>().unwrap() + 1,
            "one step"
        );
    }
    // Matched once: nothing more to propose, and a second accept is refused.
    let again = call(&w.app, "match_planned", json!({})).unwrap();
    assert!(again["proposals"].as_array().unwrap().is_empty());
    assert!(
        call(&w.app, "accept_match", json!({ "planned": planned, "imported": imported })).is_err()
    );
}

/// **T16**, the other half: two candidates → no proposal; another kind → nothing.
#[test]
fn two_candidates_are_listed_and_nothing_is_proposed() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let w = world();
    make(
        &w,
        json!([
            planned_run("2026-11-17", "P", 40),
            planned_run("2026-11-17", "P", 20),
            imported_run("2026-11-17", 41),
            { "title": "Strength", "start": "2026-11-18", "props": { "log": "workout", "planned": 1,
              "activity": "Strength training", "sport": "strength", "counts_as": "strength", "plan": "P" } },
            imported_run("2026-11-18", 30),
        ]),
    );
    let m = call(&w.app, "match_planned", json!({})).unwrap();
    assert!(m["proposals"].as_array().unwrap().is_empty(), "{m}");
    let listed = m["unmatched"].as_array().unwrap();
    assert_eq!(listed.len(), 1, "two candidates are listed; none is not, unless a day is asked");
    assert_eq!(listed[0]["candidates"].as_array().unwrap().len(), 2);
    let day = call(&w.app, "match_planned", json!({ "day": "2026-11-18" })).unwrap();
    assert!(day["proposals"].as_array().unwrap().is_empty(), "a run is not the planned strength");
    assert_eq!(day["unmatched"][0]["candidates"].as_array().unwrap().len(), 0);
}
