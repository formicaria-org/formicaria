//! **Two clients, one vault** — how a second client ever learns that the first one wrote.
//!
//! `ping` is the local poll: every view is served from SQLite, so nothing a client did not do
//! itself is visible until the poll says so. It used to answer "did *this* reindex find drift?",
//! which is a fact that can only be told once and, worse, a fact that misses most of what
//! happens.
//!
//! Both halves are asserted here, because they failed for different reasons:
//!
//! - **A write through `dispatch` produced no drift at all.** `FileStore::put` indexes the file
//!   it just wrote, mtime included, so afterwards the index and the disk agree and an
//!   incremental reindex finds nothing. The poll only ever saw *out-of-band* edits — a `git
//!   pull`, the merge driver, Vim — which was enough for exactly as long as there was one
//!   client. A note captured on a tablet was invisible on the desktop indefinitely.
//! - **Drift could only be reported once.** The reindex that detects it also writes the fresh
//!   mtimes back, so whichever client asked first consumed the news and the others were told
//!   "nothing changed" forever.
//!
//! The fix is a generation counter the clients compare against, so the answer is a *comparison*
//! rather than a *report* — and a comparison can be made by any number of clients, each at its
//! own pace.

use fm_app::{dispatch, vaults::VaultConfig, App, Host, Output};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn call(app: &App, cmd: &str, args: Value) -> Value {
    match dispatch(cmd, &args, &[], app, &NoHost) {
        Ok(Output::Json(b)) if b.is_empty() => Value::Null,
        Ok(o) => serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null),
        Err(e) => panic!("{cmd} failed: {e}"),
    }
}

/// One client's beat: send the cursor it holds, get back `(changed, generation)`.
fn beat(app: &App, since: u64) -> (bool, u64) {
    let r = call(app, "ping", json!({ "since": since }));
    (r["changed"].as_bool().unwrap(), r["generation"].as_u64().unwrap())
}

fn app_with_a_vault() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();
    let owned: Vec<(String, PathBuf)> = vec![("v".into(), vault.path().to_path_buf())];
    let configs =
        vec![VaultConfig { name: "v".into(), path: vault.path().to_path_buf(), restic: None }];
    let app = App::new(
        MultiStore::open(&owned).unwrap(),
        configs,
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

/// **The bug that made the tablet pointless.** One client captures a note; the other must be
/// told. Before the generation counter this was simply impossible — the write left no drift for
/// the poll to find, so `changed` was `false` and stayed `false`.
#[test]
fn a_note_written_by_one_client_is_announced_to_the_other() {
    let (_home, _vault, app) = app_with_a_vault();

    // Two clients, both caught up. (The desktop and the tablet; nothing distinguishes them.)
    let (_, mut desktop) = beat(&app, 0);
    let (_, mut tablet) = beat(&app, 0);
    assert_eq!(desktop, tablet, "two clients starting together hold the same cursor");

    // The tablet captures a note. Its own next beat sees the move, which is harmless — it is
    // about to re-query anyway, and it may have missed something else in the meantime.
    call(&app, "capture", json!({ "body": "written on the tablet\n", "vault": "v" }));

    // **The desktop is told, and this is the assertion that used to fail.**
    let changed;
    (changed, desktop) = beat(&app, desktop);
    assert!(changed, "a note captured by another client must be announced by the poll");

    // And once it has caught up it is quiet again — a beat is not a standing refresh order.
    let (changed, desktop_after) = beat(&app, desktop);
    assert!(!changed, "nothing has happened since; the client must not re-query on every beat");
    assert_eq!(desktop_after, desktop, "a quiet beat does not move the cursor");

    // The tablet, which never beat in between, catches up in one step rather than being told
    // "nothing changed" because the desktop already consumed the news.
    let (changed, tablet_after) = beat(&app, tablet);
    assert!(changed, "the second client to ask must still be told");
    assert_eq!(tablet_after, desktop, "and both converge on the same cursor");
    tablet = tablet_after;

    // A third client arriving now sends `since: 0`, because it has no cursor yet, and is told
    // `changed` — one redundant refresh on its very first beat.
    //
    // **Deliberate, and the safe direction.** A fresh client has just run its initial queries,
    // but "just" is not "atomically": another client can write between those queries and this
    // first beat, and a `since: 0` that meant "I am current" would swallow exactly that write
    // until something else happened to move the counter. Erring the other way costs one extra
    // SQLite read per page load. The costs are not symmetric, so this is biased toward the
    // recoverable one.
    let (changed, fresh) = beat(&app, 0);
    assert!(changed, "a client with no cursor is assumed to be behind, not assumed current");
    assert_eq!(fresh, tablet, "and it picks up the current cursor, so its next beat is quiet");
    assert!(!beat(&app, fresh).0, "quiet from the second beat on");
}

/// Every writing command must announce itself, not just `capture` — a status dragged on the
/// board and a body saved in the editor are exactly as invisible to the other screen.
#[test]
fn edits_and_property_changes_are_announced_too() {
    let (_home, _vault, app) = app_with_a_vault();
    let note = call(&app, "capture", json!({ "body": "one\n", "vault": "v" }));
    let id = note["id"].as_str().unwrap().to_string();
    let (_, mut other) = beat(&app, 0);

    let version = call(&app, "get", json!({ "id": &id }))["version"].as_str().unwrap().to_string();
    call(&app, "update_body", json!({ "id": &id, "body": "two\n", "base": version }));
    let changed;
    (changed, other) = beat(&app, other);
    assert!(changed, "an edited body must reach the other client");

    call(&app, "set_property", json!({ "id": &id, "key": "status", "value": "doing" }));
    let (changed, other_after) = beat(&app, other);
    assert!(changed, "a board drag must reach the other client");

    call(&app, "delete", json!({ "id": &id }));
    let (changed, _) = beat(&app, other_after);
    assert!(changed, "a deletion must reach the other client");
}

/// **A refused write must stay silent.** `update_body` with a stale `base` changes nothing, so
/// announcing it would send every other client off to re-run every open query for a change that
/// never landed — on a debounced editor, once per keystroke.
#[test]
fn a_rejected_write_announces_nothing() {
    let (_home, _vault, app) = app_with_a_vault();
    let note = call(&app, "capture", json!({ "body": "one\n", "vault": "v" }));
    let id = note["id"].as_str().unwrap().to_string();
    let (_, other) = beat(&app, 0);

    let stale = dispatch(
        "update_body",
        &json!({ "id": &id, "body": "two\n", "base": "not-the-version" }),
        &[],
        &app,
        &NoHost,
    );
    assert!(stale.is_err(), "the stale-base guard still refuses");

    let (changed, after) = beat(&app, other);
    assert!(!changed, "a write that was refused must not be announced");
    assert_eq!(after, other);
}

/// Reads must never move the cursor. `ping` is the sharp case — it is itself a command, so a
/// poll that bumped would tell every client something changed on every beat, forever, which is
/// a permanent full re-query of every open view on a 15-second timer.
#[test]
fn reading_moves_nothing() {
    let (_home, _vault, app) = app_with_a_vault();
    call(&app, "capture", json!({ "body": "one\n", "vault": "v" }));
    let (_, base) = beat(&app, 0);

    for cmd in ["recent", "list_vaults", "config", "backup_status"] {
        call(&app, cmd, json!({}));
    }
    call(&app, "search", json!({ "query": "one" }));
    call(&app, "board", json!({ "groupBy": "status" }));

    // Several beats, to prove the poll itself is not the thing moving it.
    for _ in 0..3 {
        let (changed, now) = beat(&app, base);
        assert!(!changed, "a read must not look like a change");
        assert_eq!(now, base, "and must not move the cursor");
    }
}

/// The out-of-band half, which is what the poll was originally built for: a note rewritten
/// behind the app's back — Vim, a `git pull`, the merge driver — still has to be noticed, and
/// now has to be noticed by **every** client rather than only the one that asked first.
#[test]
fn an_edit_behind_the_apps_back_reaches_every_client() {
    let (_home, vault, app) = app_with_a_vault();
    call(&app, "capture", json!({ "body": "one\n", "vault": "v" }));
    let (_, desktop) = beat(&app, 0);
    let (_, tablet) = beat(&app, 0);

    // Somebody edits the file directly. Sleep first so the mtime is distinguishable even on a
    // coarse-granularity filesystem — without a different mtime the drift check's own
    // precondition is unmet and this test would prove nothing.
    let path = std::fs::read_dir(vault.path().join("notes"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "md"))
        .expect("the note file");
    let text = std::fs::read_to_string(&path).unwrap();
    let (front, _) = text.rsplit_once("---\n").expect("frontmatter fence");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&path, format!("{front}---\n\nedited in vim\n")).unwrap();

    // The first client to ask finds the drift...
    let (changed, desktop_after) = beat(&app, desktop);
    assert!(changed, "an out-of-band edit must be noticed");

    // ...and the second is told too. **This is the half that used to fail**: that reindex wrote
    // the fresh mtimes back, so there was no drift left for anyone else to find.
    let (changed, tablet_after) = beat(&app, tablet);
    assert!(changed, "the drift was consumed by the first client — every client must be told");
    assert_eq!(tablet_after, desktop_after);
}
