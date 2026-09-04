//! **A slow command must not freeze every other one.**
//!
//! `dispatch` takes one global vault lock, and several arms used to hold it across work that spawns
//! subprocesses. The cost is not theoretical: ingest shells out twice per file (`pdftotext` for the
//! searchable text, `vipsthumbnail` for the preview), so a bulk import froze every tab, every pane
//! and the phone's whole UI for as long as it ran — 25–40 minutes for 5,000 PDFs, on the record.
//! `papers-plan.md` B5 named it; `ingest_finish`, added for chunked ingest on 2026-09-04, inherited
//! it, which is the worse half because that path exists for large files.
//!
//! **These tests are a rendezvous, not a stopwatch.** A stub `vipsthumbnail` on `PATH` blocks until
//! this file releases it, so the failing case waits forever and the passing case returns in
//! microseconds. That is a 5-second-versus-infinity split rather than a timing margin, which is why
//! it can live in `pixi run ci` without being flaky.
//!
//! They live in their own file rather than in `dispatch.rs`'s `mod tests` because they manipulate
//! `PATH`, which is process-global — the same reason `secrets.rs` and `fm-core/tests/backup.rs`
//! serialise on a mutex.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};

/// `PATH` is process-global; these tests must not run concurrently with each other.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn app_with_vault() -> (TempDir, TempDir, std::sync::Arc<App>) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let store = MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: vault.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, std::sync::Arc::new(app))
}

/// A `vipsthumbnail` that announces itself and then waits for permission to finish.
///
/// Chosen over a stub `git` deliberately: thumbnailing runs on **every** ingest, and no other test
/// in this suite needs `vipsthumbnail`, so the rendezvous cannot be tripped by unrelated work.
fn blocking_vipsthumbnail(dir: &Path) -> (PathBuf, PathBuf) {
    let started = dir.join("started");
    let go = dir.join("go");
    let script = dir.join("vipsthumbnail");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch '{}'\nwhile [ ! -f '{}' ]; do sleep 0.05; done\nexit 0\n",
            started.display(),
            go.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (started, go)
}

/// A 1x1 PNG — real magic bytes, so `infer` sniffs it as an image and thumbnailing is attempted.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[cfg(unix)]
#[test]
fn an_ingest_that_is_shelling_out_does_not_block_every_other_command() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempdir().unwrap();
    let (started, go) = blocking_vipsthumbnail(bin.path());
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", bin.path().display()));

    let (_home, _vault, app) = app_with_vault();

    // A: an ingest, which will park inside the stub.
    let ingesting = {
        let app = std::sync::Arc::clone(&app);
        std::thread::spawn(move || {
            dispatch("ingest", &json!({ "name": "photo.png", "vault": "notes" }), PNG, &app, &NoHost)
        })
    };

    // Wait for it to actually be inside the subprocess — not merely started.
    let mut waited = 0;
    while !started.exists() && waited < 200 {
        std::thread::sleep(Duration::from_millis(50));
        waited += 1;
    }
    assert!(started.exists(), "the stub vipsthumbnail never ran — the rendezvous is not armed");

    // B: any other command at all. **This is the assertion.** With the guard held across the
    // subprocess it waits for the stub; without it, it returns immediately.
    let (tx, rx) = mpsc::channel();
    {
        let app = std::sync::Arc::clone(&app);
        std::thread::spawn(move || {
            let out = dispatch("list_vaults", &json!({}), &[], &app, &NoHost);
            let _ = tx.send(out.is_ok());
        });
    }
    let answered = rx.recv_timeout(Duration::from_secs(5));

    // Release the stub before asserting, so a failure does not also hang the suite.
    std::fs::write(&go, b"").unwrap();
    let _ = ingesting.join().unwrap();
    std::env::set_var("PATH", old_path);

    assert_eq!(
        answered,
        Ok(true),
        "another command was blocked while ingest was inside a subprocess — the vault lock is \
         being held across the shell-out"
    );
}

/// **The activity revwalk must not block the app either.**
///
/// `activity` is among the first things the UI's first `refresh()` fires, and it used to hold the
/// vault lock across one `git log` per vault — so on a cold start with a year of history, every
/// other command queued behind it. Same rendezvous as above, with a stub `git`.
///
/// **`#[cfg(not(feature = "native-git"))]` is load-bearing, not tidiness.** Under
/// `pixi run test-native-git`, `vcs::` routes to libgit2 and the stub is never invoked — so the
/// `started` marker would never appear and this test would hang rather than fail.
#[cfg(all(unix, not(feature = "native-git")))]
#[test]
fn an_activity_revwalk_does_not_block_every_other_command() {
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: no git");
        return;
    }
    let (_home, vault, app) = app_with_vault();

    // A real repository first — built with the real git, before the stub is on PATH.
    let g = |args: &[&str]| {
        std::process::Command::new("git").arg("-C").arg(vault.path()).args(args).output().unwrap()
    };
    g(&["init", "-q"]);
    g(&["config", "user.name", "Ada"]);
    g(&["config", "user.email", "ada@example.org"]);
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();
    std::fs::write(vault.path().join("notes/a.md"), "---\nid: 01AAAAAAAAAAAAAAAAAAAAAAAA\ntype: note\ntitle: a\ncreated: 2026-09-01T00:00:00Z\nupdated: 2026-09-01T00:00:00Z\n---\n\nx\n").unwrap();
    g(&["add", "-A"]);
    g(&["commit", "-qm", "one"]);

    // Now shadow git with a blocking stub that defers to the real one once released.
    let bin = tempdir().unwrap();
    let started = bin.path().join("started");
    let go = bin.path().join("go");
    let real = String::from_utf8_lossy(
        &std::process::Command::new("sh").arg("-c").arg("command -v git").output().unwrap().stdout,
    )
    .trim()
    .to_string();
    let script = bin.path().join("git");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ntouch '{}'\nwhile [ ! -f '{}' ]; do sleep 0.05; done\nexec '{}' \"$@\"\n",
            started.display(),
            go.display(),
            real
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old_path}", bin.path().display()));

    let walking = {
        let app = std::sync::Arc::clone(&app);
        std::thread::spawn(move || {
            dispatch("activity", &json!({ "since": "1 year ago" }), &[], &app, &NoHost)
        })
    };

    let mut waited = 0;
    while !started.exists() && waited < 200 {
        std::thread::sleep(Duration::from_millis(50));
        waited += 1;
    }
    let armed = started.exists();

    // **The probe must not itself touch git**, or it blocks on the stub rather than on the lock
    // and the test measures the wrong thing. `list_vaults` looked like the obvious choice and is
    // wrong: `infos` shells out per vault for the remote label and the identity. `recent` is a
    // pure store query.
    let (tx, rx) = mpsc::channel();
    {
        let app = std::sync::Arc::clone(&app);
        std::thread::spawn(move || {
            let _ = tx.send(dispatch("recent", &json!({}), &[], &app, &NoHost).is_ok());
        });
    }
    let answered = rx.recv_timeout(Duration::from_secs(5));

    std::fs::write(&go, b"").unwrap();
    let _ = walking.join().unwrap();
    std::env::set_var("PATH", old_path);

    assert!(armed, "the stub git never ran — the rendezvous is not armed");
    assert_eq!(
        answered,
        Ok(true),
        "another command was blocked while activity was inside a git revwalk"
    );
}
