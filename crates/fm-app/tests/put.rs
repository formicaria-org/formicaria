//! **Put: the one way content moves into a note** (`actions.rs`; `decisions.md` 2026-11-17, *one
//! server, many views*). Pinned here, through the command surface every caller uses:
//!
//!  1. A piece goes into a note **whether or not it is open anywhere**: at its end, or at the end
//!     of a heading's section.
//!  2. **The version guard holds**: a `base` that is not the note's version writes nothing.
//!  3. A piece can start **a new note**, on a shelf.
//!  4. **A put wakes whoever waits for a change** (the server's change stream), at once.
//!  5. The actions table lists every action once, each **off** for the assistant.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn app() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let store = MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: vault.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

fn call(app: &App, cmd: &str, args: Value) -> Result<Value, String> {
    dispatch(cmd, &args, b"", app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
}

fn note(app: &App, title: &str, body: &str) -> String {
    let made =
        call(app, "create_notes", json!({ "notes": [{ "title": title, "body": body }] })).unwrap();
    made[0]["id"].as_str().unwrap().to_string()
}

fn body(app: &App, id: &str) -> String {
    call(app, "get", json!({ "id": id })).unwrap()["body"].as_str().unwrap().to_string()
}

#[test]
fn a_piece_goes_into_a_note_that_is_open_nowhere() {
    let (_h, _v, app) = app();
    let id = note(&app, "Ants", "# Ants\n\n## Reading\n\nOne.\n\n## Questions\n\nWhy?\n");
    call(&app, "put", json!({ "to": { "note": id }, "md": "> Leaves are carried home.\n>\n> — [A walk](https://example.org/a)" }))
        .unwrap();
    assert!(body(&app, &id)
        .ends_with("Why?\n\n> Leaves are carried home.\n>\n> — [A walk](https://example.org/a)\n"));
    call(&app, "put", json!({ "to": { "note": id, "at": "after:Reading" }, "md": "> Fungus." }))
        .unwrap();
    assert!(body(&app, &id).contains("One.\n\n> Fungus.\n\n## Questions"), "{}", body(&app, &id));
}

#[test]
fn a_stale_base_writes_nothing() {
    let (_h, _v, app) = app();
    let id = note(&app, "Ants", "First.\n");
    let read = call(&app, "get", json!({ "id": id })).unwrap();
    let base = read["version"].as_str().unwrap().to_string();
    call(&app, "update_body", json!({ "id": id, "body": "Changed meanwhile.\n", "base": base }))
        .unwrap();
    let r = call(&app, "put", json!({ "to": { "note": id }, "md": "> Late.", "base": base }));
    assert!(r.is_err(), "a put over a newer version is refused");
    assert_eq!(body(&app, &id), "Changed meanwhile.\n");
}

#[test]
fn a_piece_can_start_a_new_note_on_a_shelf() {
    let (_h, _v, app) = app();
    let made = call(&app, "put", json!({ "to": { "new": { "title": "From the web", "shelf": "Ants" } }, "md": "> A passage." }))
        .unwrap();
    let id = made["id"].as_str().unwrap();
    let got = call(&app, "get", json!({ "id": id })).unwrap();
    assert_eq!(got["title"], "From the web");
    assert!(got["tags"].as_array().unwrap().iter().any(|t| t == "shelf:Ants"), "{got}");
    assert_eq!(got["body"].as_str().unwrap().trim(), "> A passage.");
    assert!(call(&app, "put", json!({ "to": {}, "md": "x" })).is_err(), "nowhere is refused");
    assert!(
        call(&app, "put", json!({ "to": { "note": id }, "md": "  " })).is_err(),
        "nothing is refused"
    );
}

#[test]
fn a_put_wakes_whoever_waits_for_a_change() {
    let (_h, _v, app) = app();
    let id = note(&app, "Ants", "First.\n");
    let app = Arc::new(app);
    let since = app.generation();
    let waiter = {
        let app = Arc::clone(&app);
        std::thread::spawn(move || {
            let t = Instant::now();
            (app.wait_changed(since, Duration::from_secs(20)), t.elapsed())
        })
    };
    std::thread::sleep(Duration::from_millis(100));
    call(&app, "put", json!({ "to": { "note": id }, "md": "> Now." })).unwrap();
    let (g, took) = waiter.join().unwrap();
    assert!(g > since);
    assert!(took < Duration::from_secs(5), "woken at once, not at the timeout: {took:?}");
    // Nothing changing: the wait ends at its timeout with the same generation.
    let now = app.generation();
    assert_eq!(app.wait_changed(now, Duration::from_millis(50)), now);
}

#[test]
fn the_actions_table_lists_each_action_once_off_for_the_assistant() {
    let (_h, _v, app) = app();
    let all = call(&app, "actions", json!({})).unwrap();
    let all = all.as_array().unwrap();
    assert!(all.iter().any(|a| a["key"] == "put" && a["runs"] == "server"));
    assert!(all.iter().all(|a| a["agent"] == "off"));
}

#[test]
fn what_a_put_wrote_can_be_swapped_or_taken_back() {
    let (_h, _v, app) = app();
    let id = note(&app, "Ants", "Mine.\n");
    call(&app, "put", json!({ "to": { "note": id }, "md": "[Image](https://example.org/a.png)" }))
        .unwrap();
    // An alternative: the same piece in another form.
    call(&app, "put", json!({ "to": { "note": id }, "md": "![](asset:x)", "replace": "[Image](https://example.org/a.png)" }))
        .unwrap();
    assert_eq!(body(&app, &id), "Mine.\n\n![](asset:x)\n");
    // Undo: taken back, no gap left.
    call(&app, "put", json!({ "to": { "note": id }, "md": "", "replace": "![](asset:x)" }))
        .unwrap();
    assert_eq!(body(&app, &id), "Mine.\n");
    // Gone already: nothing written.
    call(&app, "put", json!({ "to": { "note": id }, "md": "", "replace": "not there" })).unwrap();
    assert_eq!(body(&app, &id), "Mine.\n");
}
