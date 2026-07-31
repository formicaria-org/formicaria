//! **Reproducing "146 notes not in history" on a laptop, with no phone involved.**
//!
//! The phone showed 146 and nobody could say what they were — app-private storage is unreadable on a
//! release build, MIUI eats our logcat tag, and the owner works only through the UI. But the
//! *mechanism* needs no device at all, and this is it:
//!
//! `commit_all` stages exactly the paths `FileStore::put`/`delete` recorded — deliberately, so a vault
//! that is also a project repo never has its owner's staged work swept into an `auto:` commit. That
//! record is **per-process memory**. Android kills backgrounded apps constantly, so on a phone almost
//! every relaunch orphaned whatever the previous run had written: not lagging, *permanently
//! unstageable*, and nothing said so.
//!
//! Dropping and re-opening the `App` is exactly what a restart does to that record, which makes the
//! whole failure reproducible in milliseconds. What is pinned here is the shape of the bug **and** the
//! shape of the fix: `commit_all` alone still records nothing (that precision is not a bug and must not
//! be "fixed"), while `unrecorded` finds them and `record_unrecorded` commits them.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap()
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(serde_json::Value::Null))
}

/// An app over one vault — built fresh each time, which is the point: a new `App` has no memory of
/// what an earlier one wrote, exactly like a relaunched phone.
fn open_app(home: &TempDir, vault: &Path) -> App {
    let store = MultiStore::open(&[("v".to_string(), vault.to_path_buf())]).unwrap();
    App::new(
        store,
        vec![VaultConfig { name: "v".into(), path: vault.to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    )
}

#[test]
fn notes_written_before_a_restart_are_found_and_recordable() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // ── Session zero: one note, recorded. ──
    //
    // Needed so the vault has history *and* so `ensure_repo`'s own files (`.gitattributes`,
    // `.gitignore`) are already committed. `commit_all` stages those unconditionally — they must
    // travel or a collaborator's clone loses the merge driver — so on a brand-new repo it has
    // something of its own to commit even when no note is its own. Skipping this step is what made
    // the first draft of this test assert something false about the code.
    {
        let app = open_app(&home, &vault);
        call(&app, "capture", serde_json::json!({ "body": "already recorded", "vault": "v" }))
            .unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    }

    // ── Session one: write three notes through the real command, then let the process end. ──
    {
        let app = open_app(&home, &vault);
        // Through the command door, the same one the phone uses — `App::lock` is private, which is the
        // seam doing its job.
        for body in ["first", "second", "third"] {
            call(&app, "capture", serde_json::json!({ "body": body, "vault": "v" })).unwrap();
        }
    }

    // ── Session two: a fresh App, with no memory of those writes. ──
    let app = open_app(&home, &vault);

    // The commit path records **nothing**, because it stages only what *this* process wrote. That is
    // the precision that protects a project vault from an `auto:` commit sweeping the user's index —
    // it is not the bug, and a future change that makes this line commit is a regression.
    let committed = call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }))
        .expect("commit must not error, only decline");
    assert_eq!(committed["committed"], false, "nothing of *this* process's changed: {committed}");
    assert_eq!(
        String::from_utf8_lossy(&git(&vault, &["log", "--oneline"]).stdout).lines().count(),
        1,
        "history still holds only session zero's commit — the three notes are nowhere"
    );

    // The reconciliation surface finds them, and says how they are out of history.
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(un[0]["vault"], "v");
    assert_eq!(un[0]["count"], 3, "{un}");
    // **All three are `new`.** This is the distinction the phone could not report: three notes that
    // exist nowhere else, not three notes something was rewriting.
    assert_eq!(un[0]["new"], 3, "{un}");
    assert_eq!(un[0]["modified"], 0);
    assert_eq!(un[0]["deleted"], 0);
    // And enough detail to recognise them without a shell: a title or first line, a size, a time.
    let first = &un[0]["notes"][0];
    assert!(first["id"].as_str().is_some_and(|s| !s.is_empty()), "{first}");
    assert_eq!(first["kind"], "new");
    assert!(first["bytes"].as_u64().is_some_and(|b| b > 0), "{first}");
    assert!(first["modified"].as_str().is_some_and(|s| s.contains('T')), "{first}");

    // Recording them is the one-click fix, and it works from a process that never wrote them.
    let rec = call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    assert_eq!(rec["committed"], true, "{rec}");
    assert_eq!(rec["notes"], 3, "{rec}");
    let after = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(after, serde_json::json!([]), "nothing left unrecorded: {after}");
}

#[test]
fn the_kinds_tell_the_two_stories_apart() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // A committed starting point, written and recorded by one session.
    let id = {
        let app = open_app(&home, &vault);
        let meta =
            call(&app, "capture", serde_json::json!({ "body": "committed", "vault": "v" })).unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
        meta["id"].as_str().unwrap().to_string()
    };

    // Then, from a fresh process: one note edited, one new note, one committed note deleted.
    std::fs::write(
        vault.join(format!("notes/{id}.md")),
        std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap() + "\nan edit\n",
    )
    .unwrap();
    std::fs::write(
        vault.join("notes/01NEWNEWNEWNEWNEWNEWNEWNEW.md"),
        "---\nschema: 1\nid: 01NEWNEWNEWNEWNEWNEWNEWNEW\ntype: note\ntitle: Fresh\ncreated: 2026-07-31T00:00:00Z\nupdated: 2026-07-31T00:00:00Z\n---\nbrand new\n",
    )
    .unwrap();

    let app = open_app(&home, &vault);
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    // **The whole point of this pass**: one number split into two meanings. A pile of `modified` says
    // something is rewriting notes; a pile of `new` says notes exist in one place only.
    assert_eq!(un[0]["new"], 1, "{un}");
    assert_eq!(un[0]["modified"], 1, "{un}");
    // **An untitled note must never be labelled with its frontmatter.** The first version of
    // `title_of` scanned the raw file, so every untitled note came back titled `schema: 1` — seen on
    // the owner's phone within the hour, where it read as "142 malformed notes" and nearly sent the
    // diagnosis down a wrong path.
    let all: Vec<String> = un[0]["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["title"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        !all.iter().any(|t| t.starts_with("schema:") || t.starts_with("id:")),
        "a title must come from the body, never from the frontmatter: {all:?}"
    );

    // The titled note is nameable in the list, not just a ULID.
    let titles: Vec<&str> =
        un[0]["notes"].as_array().unwrap().iter().filter_map(|n| n["title"].as_str()).collect();
    assert!(titles.contains(&"Fresh"), "a note is named by its title: {titles:?}");
}

/// **The root-cause fix: a relaunch adopts what the previous session wrote.**
///
/// The chip and the panel make the orphaned notes *visible*; they do not stop them happening. `App`
/// now rebuilds the write-record from the filesystem at open, so the next ordinary commit records
/// them — no click, and no new staging semantics. This drives `App::load`, which is the only path
/// that reconciles, through `FM_VAULTS`/`FM_VAULT` (the same override `fm-serve` documents).
#[test]
fn a_relaunch_adopts_the_previous_session_s_notes_and_the_next_commit_records_them() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // Session zero: one note, recorded, so `.gitattributes`/`.gitignore` are already in history.
    {
        let app = open_app(&home, &vault);
        call(&app, "capture", serde_json::json!({ "body": "recorded", "vault": "v" })).unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    }
    // Session one writes two notes and dies without committing.
    {
        let app = open_app(&home, &vault);
        for body in ["orphan one", "orphan two"] {
            call(&app, "capture", serde_json::json!({ "body": body, "vault": "v" })).unwrap();
        }
    }

    // Session two comes up through `App::load`, which is what a relaunch really does.
    let app = {
        // Safety: single-threaded test setup, before the app reads either variable.
        unsafe {
            std::env::set_var("FM_VAULTS", home.path().join("vaults.json"));
            std::env::set_var("FM_VAULT", &vault);
        }
        let (app, _skipped) = App::load().expect("load");
        app
    };

    // **The ordinary commit records them.** Before this change it answered "nothing of mine changed"
    // and those two notes were unstageable for the life of the vault.
    let out = call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }))
        .expect("commit");
    assert_eq!(out["committed"], true, "the adopted notes were committed: {out}");
    let after = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(after, serde_json::json!([]), "nothing left outstanding: {after}");
    // And they are really in history, by name.
    let log = String::from_utf8_lossy(&git(&vault, &["log", "--stat", "-1"]).stdout).to_string();
    assert_eq!(log.matches(".md").count(), 2, "both orphans in the commit: {log}");
}

/// **A duplicate count is what turns a number into a cause.**
///
/// The real incident (2026-07-31): 147 notes outstanding on the phone, of which one prompt to the
/// agent appeared ~138 times, every copy timestamped the same minute. "147 unrecorded" reads as a
/// backlog; "one body written 138 times at machine pace" names a loop. The count must key on the
/// **body**, because copies differ in `id` and `created` — hashing the file would report every
/// duplicate as unique and hide the very thing worth seeing.
#[test]
fn duplicates_are_counted_by_body_and_the_role_names_the_code_path() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // Three copies of one message: same body, different ids and timestamps, exactly as a loop leaves
    // them. Plus one ordinary note, so the counts stay honest.
    // **Valid Crockford base32 — no I, L, O or U.** `01ROOT…` looks like a ULID and is not one, so
    // `parse_note_ref` rejects it and `is_message` silently answers false; the first version of this
    // test used it and blamed the code. `mock.ts` carries the same warning about `M0CK` vs `MOCK`.
    let root = "01RTRTRTRTRTRTRTRTRTRTRTRT";
    for (i, id) in ["01CCCCCCCCCCCCCCCCCCCCCC01", "01CCCCCCCCCCCCCCCCCCCCCC02", "01CCCCCCCCCCCCCCCCCCCCCC03"]
        .iter()
        .enumerate()
    {
        std::fs::write(
            vault.join(format!("notes/{id}.md")),
            format!(
                "---\nschema: 1\nid: {id}\ntype: note\ncreated: 2026-07-31T07:44:0{i}Z\nupdated: 2026-07-31T07:44:0{i}Z\nthread_of: note:{root}\n---\n@lfm2.5-230m does mRNA change DNA? /search\n"
            ),
        )
        .unwrap();
    }
    std::fs::write(
        vault.join("notes/01PZPZPZPZPZPZPZPZPZPZPZPZ.md"),
        "---\nschema: 1\nid: 01PZPZPZPZPZPZPZPZPZPZPZPZ\ntype: note\ntitle: An ordinary note\ncreated: 2026-07-31T09:00:00Z\nupdated: 2026-07-31T09:00:00Z\n---\nsomething else entirely\n",
    )
    .unwrap();

    let app = open_app(&home, &vault);
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    let rows = un[0]["notes"].as_array().unwrap();
    assert_eq!(un[0]["count"], 4, "{un}");

    // The three copies each report the whole family, so any one row tells the story.
    let dupes: Vec<u64> = rows
        .iter()
        .filter(|n| n["role"] == "message")
        .map(|n| n["copies"].as_u64().unwrap())
        .collect();
    assert_eq!(dupes, vec![3, 3, 3], "each copy names the size of its family: {rows:?}");
    // **The role names the code path** — a message came from the reply path, not from capture. That
    // distinction is what a title could never carry.
    assert_eq!(rows.iter().filter(|n| n["role"] == "message").count(), 3);

    let plain = rows.iter().find(|n| n["role"] == "note").expect("the ordinary note");
    assert_eq!(plain["copies"], 1, "a unique note is not a duplicate: {plain}");
    assert_eq!(plain["title"], "An ordinary note");
}
