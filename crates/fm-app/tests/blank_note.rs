//! **A blank note is not kept** (`decisions.md#data`, 2026-10-27). Owner: *"I see no point in
//! allowing a completely empty note to be saved in any device … if one opens a note and does nothing
//! with it, we do not keep it."*
//!
//! Three ways a blank note could survive, one test each: the editor leaves it (`discard_if_blank`),
//! a commit sweeps it into history (`to_record`), or the process ends with it open — on a phone, Back
//! ends the process — and the next start finds it (`App::load`).
//!
//! Booted through **`App::load`**, the path `mobile/src-tauri`'s `boot()` takes, by pointing
//! `FM_VAULTS` at a temp config. `FM_VAULTS` is process-global, so the tests share one lock.

use fm_app::{dispatch, App, Host};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tempfile::{tempdir, TempDir};

static ENV: Mutex<()> = Mutex::new(());

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

fn call(app: &App, cmd: &str, args: serde_json::Value) -> serde_json::Value {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(serde_json::Value::Null))
        .unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

/// A note file exactly as `FileStore` writes one — what a process killed with it open leaves.
fn write_note(notes: &Path, body: &str) -> PathBuf {
    let o = fm_model::Object::new(fm_model::Kind::Note, body.to_string());
    let path = notes.join(format!("{}.md", o.id));
    std::fs::write(&path, fm_core::frontmatter::to_file(&o).unwrap()).unwrap();
    path
}

/// A git vault with its tutorials committed, so the app adds nothing of its own at boot.
fn vault() -> (TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("v");
    let notes = vault.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "Tester"]);
    git(&vault, &["config", "user.email", "tester@example.com"]);
    for (name, text) in fm_app::tutorials::TUTORIALS {
        std::fs::write(notes.join(name), text).unwrap();
    }
    git(&vault, &["add", "-A"]);
    git(&vault, &["commit", "-q", "-m", "first"]);
    (dir, vault)
}

fn boot(home: &TempDir, vault: &Path) -> App {
    #[cfg(feature = "native-git")]
    fm_core::vcs::force_native(true);
    let cfg = home.path().join("vaults.json");
    std::fs::write(
        &cfg,
        serde_json::json!({ "vaults": [{ "name": "v", "path": vault }] }).to_string(),
    )
    .unwrap();
    // SAFETY: guarded by `ENV`; these tests are the only writers.
    unsafe { std::env::set_var("FM_VAULTS", &cfg) };
    App::load().expect("the app must open").0
}

fn file_of(vault: &Path, id: &str) -> PathBuf {
    vault.join("notes").join(format!("{id}.md"))
}

fn new_note(app: &App, body: &str) -> String {
    call(app, "capture", serde_json::json!({ "body": body, "vault": "v" }))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn discard(app: &App, id: &str) -> bool {
    call(app, "discard_if_blank", serde_json::json!({ "id": id }))["discarded"].as_bool().unwrap()
}

#[test]
fn a_note_left_as_it_came_is_discarded_and_one_with_anything_in_it_is_not() {
    let _env = ENV.lock().unwrap_or_else(|p| p.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (home, vault) = vault();
    let app = boot(&home, &vault);

    let blank = new_note(&app, "");
    assert!(discard(&app, &blank), "a note nobody wrote in is not kept");
    assert!(!file_of(&vault, &blank).exists());
    assert!(!discard(&app, &blank), "a note already gone answers false, not an error");

    let spaces = new_note(&app, "  \n\n ");
    assert!(discard(&app, &spaces), "whitespace is nothing");

    // Anything a person put there keeps it: text, a title, a tag, a date, any property.
    let text = new_note(&app, "a thought");
    let titled = new_note(&app, "");
    call(
        &app,
        "set_property",
        serde_json::json!({ "id": titled, "key": "title", "value": "Plan" }),
    );
    let tagged = new_note(&app, "");
    call(&app, "set_property", serde_json::json!({ "id": tagged, "key": "tags", "value": "idea" }));
    let dated = new_note(&app, "");
    call(
        &app,
        "set_property",
        serde_json::json!({ "id": dated, "key": "due", "value": "2026-11-02" }),
    );
    let custom = new_note(&app, "");
    call(&app, "set_property", serde_json::json!({ "id": custom, "key": "place", "value": "lab" }));
    for (what, id) in [
        ("text", &text),
        ("a title", &titled),
        ("a tag", &tagged),
        ("a date", &dated),
        ("a property", &custom),
    ] {
        assert!(!discard(&app, id), "a note with {what} is kept");
        assert!(file_of(&vault, id).exists(), "a note with {what} is still on disk");
    }
}

#[test]
fn a_blank_note_is_never_recorded_nor_reported_as_unrecorded() {
    let _env = ENV.lock().unwrap_or_else(|p| p.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (home, vault) = vault();
    let app = boot(&home, &vault);

    let blank = new_note(&app, "");
    let written = new_note(&app, "words");
    let r = call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }));
    assert_eq!(r["committed"], true, "the note with words is recorded: {r}");

    let tracked = git(&vault, &["ls-files", "notes"]);
    assert!(tracked.contains(&format!("{written}.md")), "the written note is in history");
    assert!(!tracked.contains(&format!("{blank}.md")), "the blank note never reaches history");
    // Still on disk — it may be open in an editor right now — but not "work at risk".
    assert!(file_of(&vault, &blank).exists());
    let outstanding = call(&app, "unrecorded", serde_json::json!({}));
    assert_eq!(outstanding, serde_json::json!([]), "a blank note is not reported as unrecorded");

    // Once written in, it is an ordinary note and the next commit records it.
    call(
        &app,
        "update_body",
        serde_json::json!({ "id": blank, "body": "now it has words", "base": "" }),
    );
    call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }));
    assert!(git(&vault, &["ls-files", "notes"]).contains(&format!("{blank}.md")));
}

#[test]
fn a_blank_note_left_by_an_ended_process_is_removed_at_the_next_start() {
    let _env = ENV.lock().unwrap_or_else(|p| p.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (home, vault) = vault();
    // What a phone left when Back ended it: one note opened and left blank, one written in. Neither
    // was committed.
    let blank = write_note(&vault.join("notes"), "");
    let written = write_note(&vault.join("notes"), "written, never committed");

    let app = boot(&home, &vault);
    assert!(!blank.exists(), "the blank note is removed at open");
    assert!(written.exists(), "a note with anything in it is never removed");
    call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }));
    let tracked = git(&vault, &["ls-files", "notes"]);
    let name = |p: &Path| p.file_name().unwrap().to_string_lossy().into_owned();
    assert!(tracked.contains(&name(&written)), "the written one is adopted and recorded");
    assert!(!tracked.contains(&name(&blank)));
}

#[test]
fn a_blank_note_already_in_history_is_left_alone_at_start() {
    let _env = ENV.lock().unwrap_or_else(|p| p.into_inner());
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (home, vault) = vault();
    // Committed before this rule, or by another device: other clones have it, so start-up does not
    // delete it behind anyone's back. Leaving it in an editor is what discards it.
    let old = write_note(&vault.join("notes"), "");
    git(&vault, &["add", "-A"]);
    git(&vault, &["commit", "-q", "-m", "an old blank note"]);

    let _app = boot(&home, &vault);
    assert!(old.exists());
}
