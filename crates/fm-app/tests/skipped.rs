//! The unreadable-note surface: what the app can say about a note it could not parse,
//! and the one thing it can do about it.
//!
//! A note whose frontmatter does not parse is absent from every view. That is deliberate
//! — see `docs/context/decisions.md` — but "absent" is only survivable if the app can
//! still *name* the note and hand it to something that can fix it. These tests cover the
//! naming and the handing over, including the part that matters most: `open_skipped`
//! resolves the path from the indexer's own current skipped set, so the set is an
//! allowlist and a caller cannot name a path of their own.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fm_app::{App, Host, Output};
use fm_core::MultiStore;
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};

/// A `Host` that records instead of shelling out — the seam exists precisely so this arm
/// is testable without launching a text editor on the machine running CI.
struct Recording(Mutex<Vec<PathBuf>>);

impl Host for Recording {
    fn open_external(&self, path: &Path) -> Result<(), String> {
        self.0.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

/// Frontmatter with conflict markers in it — what a divergent `status` field actually
/// leaves on disk (see `fm-cli/tests/merge.rs`). This is the shape that does not parse.
const BROKEN: &str = "---\nid: 01JQ0000000000000000000000\ntype: task\ntitle: broken\n\
    created: 2026-07-17T10:00:00Z\nupdated: 2026-07-17T10:00:00Z\n\
    <<<<<<< ours\nstatus: doing\n=======\nstatus: done\n>>>>>>> theirs\n---\n\nBody.\n";

/// Deliberately complete: an incomplete one is skipped too, for a *different* reason, and
/// then the test proves nothing about the markers it meant to be about.
const GOOD: &str = "---\nid: 01JQ0000000000000000000001\ntype: note\ntitle: fine\n\
    created: 2026-07-17T10:00:00Z\nupdated: 2026-07-17T10:00:00Z\n---\n\nBody.\n";

fn vault_with_one_broken_note() -> (TempDir, App) {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("broken.md"), BROKEN).unwrap();
    std::fs::write(notes.join("fine.md"), GOOD).unwrap();

    let store = MultiStore::open(&[("home".to_string(), dir.path())]).expect("open vault");
    (dir, App::new(store, vec![], None, false))
}

fn call(app: &App, host: &dyn Host, cmd: &str, args: Value) -> Result<Value, String> {
    match fm_app::dispatch(cmd, &args, &[], app, host)? {
        Output::Json(b) if b.is_empty() => Ok(Value::Null),
        Output::Json(b) => Ok(serde_json::from_slice(&b).expect("valid json")),
        Output::Bytes(_) => panic!("{cmd} answered with bytes"),
    }
}

/// The vault opens, the readable note is served, and the unreadable one is *named* —
/// with its vault, so a list can group by audience rather than parsing a string apart.
#[test]
fn an_unreadable_note_is_named_on_the_heartbeat_and_does_not_stop_the_vault() {
    let (_dir, app) = vault_with_one_broken_note();
    let host = Recording(Mutex::new(Vec::new()));

    let ping = call(&app, &host, "ping", json!({})).expect("ping");
    let skipped = ping["skipped"].as_array().expect("skipped is a list");

    assert_eq!(skipped.len(), 1, "exactly the broken one: {skipped:?}");
    assert_eq!(skipped[0]["name"], "broken.md");
    assert_eq!(skipped[0]["vault"], "home", "labelled by audience");
    assert!(
        skipped[0]["reason"].as_str().is_some_and(|r| !r.is_empty()),
        "and says why, in the parser's words: {skipped:?}"
    );
    assert!(
        skipped[0].get("path").is_none(),
        "the absolute path is not handed to the browser: {skipped:?}"
    );

    // The other note is unaffected — one bad file must never take the vault down.
    let search = call(&app, &host, "search", json!({ "query": "fine" })).expect("search");
    assert!(!search.as_array().unwrap().is_empty(), "the readable note still serves");
}

/// The one action available, and the reason it is safe: the path comes from the skipped
/// set, never from the caller.
#[test]
fn open_skipped_hands_over_the_real_path() {
    let (dir, app) = vault_with_one_broken_note();
    let host = Recording(Mutex::new(Vec::new()));

    call(&app, &host, "ping", json!({})).expect("ping");
    call(&app, &host, "open_skipped", json!({ "vault": "home", "name": "broken.md" }))
        .expect("opens");

    let opened = host.0.lock().unwrap().clone();
    assert_eq!(opened, vec![dir.path().join("notes").join("broken.md")]);
}

/// **The guard.** Every name that is not currently broken is refused — a readable note, a
/// file that does not exist, a traversal attempt, and the right name in the wrong vault.
/// Nothing is opened for any of them.
#[test]
fn open_skipped_refuses_anything_not_currently_unreadable() {
    let (_dir, app) = vault_with_one_broken_note();
    let host = Recording(Mutex::new(Vec::new()));
    call(&app, &host, "ping", json!({})).expect("ping");

    for (vault, name) in [
        ("home", "fine.md"),                    // parses, so not ours to open
        ("home", "nope.md"),                    // does not exist
        ("home", "../../../etc/passwd"),        // traversal
        ("home", "/etc/passwd"),                // absolute
        ("other", "broken.md"),                 // right file, wrong audience
        ("", ""),                               // empty
    ] {
        let r = call(&app, &host, "open_skipped", json!({ "vault": vault, "name": name }));
        assert!(r.is_err(), "{vault}/{name} must be refused, got {r:?}");
    }

    assert!(
        host.0.lock().unwrap().is_empty(),
        "and nothing reached the OS: {:?}",
        host.0.lock().unwrap()
    );
}
