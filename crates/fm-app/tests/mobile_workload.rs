//! **The phone's day, at the one door every frontend goes through.**
//!
//! `ui/src/App.phone.test.ts` drives the same journeys from the page and counts commands; this
//! drives them at `dispatch` and asks the questions only the backend can answer — does a
//! multi-megabyte photo actually become a blob, does a 2 MB body survive a round trip through the
//! command surface, and (the one nothing had ever checked) **does a slow command block every other
//! command while it runs.**
//!
//! That last one is the load-bearing test in this file. `dispatch.rs` documents a lock discipline
//! — five arms deliberately drop the vault lock before slow I/O — and it was a *comment*. On the
//! desktop `fm-serve` is thread-per-connection, so a regression there would show up as one stalled
//! browser tab; on the phone every command is serialised through one bridge, so a held lock is the
//! whole app stopping. A discipline nothing checks lasts exactly one refactor.

use fm_app::{dispatch, App, Host};
use fm_core::MultiStore;
use fm_app::vaults::VaultConfig;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

/// A host whose `open_external` parks until it is told to finish, announcing when it has been
/// entered. The instrument for the lock-discipline test below: it turns "is the guard still held
/// while the platform is being called?" into a question with a yes/no answer instead of a
/// stopwatch reading.
struct BlockingHost {
    entered: std::sync::mpsc::Sender<()>,
    release: std::sync::Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}
impl Host for BlockingHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        self.entered.send(()).ok();
        if let Some(rx) = self.release.lock().unwrap().take() {
            rx.recv().ok();
        }
        Ok(())
    }
}

/// An app with an empty vault list in its own tempdir, so nothing touches the real config.
/// Same harness as `acquire.rs`; kept identical on purpose so the two read alike.
fn app() -> (TempDir, App) {
    let home = tempdir().unwrap();
    let config = home.path().join("vaults.json");
    let app = App::new(
        MultiStore::open(&[] as &[(String, PathBuf)]).unwrap(),
        Vec::<VaultConfig>::new(),
        Some(config),
        true,
    );
    (home, app)
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<String, String> {
    dispatch(cmd, &args, &[], app, &NoHost).map(|o| String::from_utf8(o.into_bytes()).unwrap())
}

fn with_bytes(app: &App, cmd: &str, args: serde_json::Value, body: &[u8]) -> Result<String, String> {
    dispatch(cmd, &args, body, app, &NoHost).map(|o| String::from_utf8(o.into_bytes()).unwrap())
}

/// For arms that answer with raw bytes. `call` runs the reply through `String::from_utf8`, which
/// is right for every JSON arm and panics on `resolve_asset` — a JPEG is not UTF-8.
fn call_raw(app: &App, cmd: &str, args: serde_json::Value) -> Result<Vec<u8>, String> {
    dispatch(cmd, &args, &[], app, &NoHost).map(|o| o.into_bytes())
}

/// A vault registered through the command surface, as the app itself would.
fn vault_in(home: &TempDir, app: &App, name: &str) -> PathBuf {
    let path = home.path().join(name);
    call(app, "create_vault", json!({ "name": name, "path": path.to_string_lossy() })).unwrap();
    path
}

/// A JPEG-shaped payload of `n` bytes: real magic so MIME sniffing has something to work with,
/// then non-repeating filler so a truncation cannot hide.
fn photo(n: usize) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00];
    v.extend((0..n - 11).map(|i| (i.wrapping_mul(31) >> 3) as u8));
    v
}

/// **Adding a photo the size a phone takes.**
///
/// The only existing byte-body test anywhere passes a 67-byte PNG (`acquire.rs`). Nothing had ever
/// pushed a real photo through `ingest`, so nothing could have noticed the blob write, the hash,
/// or the note round trip behaving differently at size.
#[test]
fn a_multi_megabyte_photo_becomes_a_blob_and_a_note() {
    let (home, app) = app();
    let vault = vault_in(&home, &app, "v");
    let bytes = photo(4 * 1024 * 1024);

    let out = with_bytes(&app, "ingest", json!({ "name": "IMG_2201.jpg", "vault": "v" }), &bytes)
        .expect("a 4 MB photo must ingest");
    assert!(out.contains("\"type\":\"asset\""), "ingest should answer with an asset note: {out}");
    assert!(out.contains("image/jpeg"), "the MIME must be sniffed from the bytes: {out}");

    // Exactly one blob, and it holds exactly what was sent. Through `BlobStore` rather than a
    // hand-rolled walk: the store sharded the path (`blobs/sha256/ab/cd/<hash>`) and a `read_dir`
    // of `blobs/` finds only the shard directory, which is how the first draft of this test
    // "found" zero blobs beside a working ingest.
    let store = fm_core::BlobStore::new(&vault);
    let blobs = store.blob_paths();
    assert_eq!(blobs.len(), 1, "one photo in, one blob out");
    assert_eq!(
        std::fs::read(&blobs[0]).unwrap(),
        bytes,
        "the stored blob differs from the bytes that were ingested",
    );

    // The same photo again must dedupe to the same blob — content addressing, at size.
    with_bytes(&app, "ingest", json!({ "name": "copy.jpg", "vault": "v" }), &bytes).unwrap();
    assert_eq!(
        fm_core::BlobStore::new(&vault).blob_paths().len(),
        1,
        "identical bytes must not produce a second blob",
    );
}

/// A long note, written and read back through the command surface rather than the store.
#[test]
fn a_two_megabyte_body_round_trips_through_dispatch() {
    let (home, app) = app();
    vault_in(&home, &app, "v");

    let created = call(&app, "capture", json!({ "body": "seed", "vault": "v" })).unwrap();
    let id = created
        .split("\"id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("capture returns an id")
        .to_string();

    let mut body = String::new();
    let mut i = 0;
    while body.len() < 2_000_000 {
        body.push_str(&format!("{i}. the advantage estimate leaks across the boundary\n"));
        i += 1;
    }
    body.push_str("\nthe quokka theorem resolves the marsupial conjecture\n");

    call(&app, "update_body", json!({ "id": id, "body": body, "base": "" })).unwrap();
    let back = call(&app, "get", json!({ "id": id })).unwrap();
    assert!(
        back.contains("quokka theorem"),
        "a needle 2 MB into the body did not survive the round trip",
    );

    // …and it is searchable, which is the half that goes through FTS tokenisation.
    let hits = call(&app, "search", json!({ "query": "quokka" })).unwrap();
    assert!(hits.contains(&id), "a 2 MB note must still be findable by a word inside it");
}

/// **A slow command must not stop every other command — asserted, not timed.**
///
/// `dispatch.rs`'s module doc says five arms deliberately drop the vault lock before slow I/O, and
/// nothing checked it. On the desktop a regression there stalls one browser tab; on the phone every
/// command is serialised through one bridge, so a held guard is the whole app stopping.
///
/// **Why this is not a stopwatch.** Two timing versions were written and both were useless, which
/// is worth recording because the third attempt is the honest one:
///  - an absolute ceiling (`ping < 1s`) could not fail — `ping` costs about a millisecond here, so
///    even fully serialised it stayed orders of magnitude inside the budget;
///  - a ceiling relative to an idle ping *did* fire, but on the wrong thing: hammering `recent`
///    alongside it measured `recent` legitimately holding the lock across a `load_all()` of the
///    corpus (~120 ms against ~1.3 ms idle at only 300 notes — real, expected, and not the
///    discipline under test). Swapping the load to `resolve_asset` removed the false alarm and with
///    it all the signal: a 2 MB blob read comes from page cache in well under a millisecond, so
///    *holding* the guard across it costs nothing a clock can see. Verified by deliberately holding
///    it: the test still passed.
///
/// So the property is asserted directly instead. `open_external` resolves the path under the guard
/// and then calls the platform with it **released** — a documented, load-bearing choice, because
/// handing a file to the OS can block on anything. A host that parks inside that call therefore
/// holds the arm open indefinitely while owning no lock. If `ping` answers, the guard was dropped;
/// if it does not, it was not. No timing, no flakiness, and it fails hard on the regression rather
/// than drifting.
#[test]
fn an_arm_that_drops_the_lock_does_not_block_other_commands() {
    use std::sync::mpsc;

    let (home, app) = app();
    let vault = vault_in(&home, &app, "v");

    let out = with_bytes(&app, "ingest", json!({ "name": "big.jpg", "vault": "v" }), &photo(64 * 1024))
        .unwrap();
    let reference = out
        .split("\"assets\":[\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("the asset note names its blob")
        .to_string();
    assert!(vault.join("blobs").exists());

    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let host = BlockingHost { entered: entered_tx, release: std::sync::Mutex::new(Some(release_rx)) };

    let app = Arc::new(app);
    let holder = {
        let app = Arc::clone(&app);
        std::thread::spawn(move || {
            // Parks inside the host call, with the vault guard released — if the discipline holds.
            dispatch("open_external", &json!({ "reference": reference }), &[], &app, &host)
        })
    };

    // The arm is now inside the platform call and will stay there until released.
    entered_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("open_external never reached the host — the test cannot conclude anything");

    // Can anything else get in? Answered on another thread so a held lock shows up as a timeout
    // rather than deadlocking the test run.
    let probe = {
        let app = Arc::clone(&app);
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let r = dispatch("ping", &json!({ "since": 0 }), &[], &app, &NoHost);
            done_tx.send(r.is_ok()).ok();
        });
        done_rx
    };
    let answered = probe.recv_timeout(Duration::from_secs(5));

    release_tx.send(()).ok();
    let _ = holder.join();

    assert_eq!(
        answered.ok(),
        Some(true),
        "`ping` could not be answered while `open_external` was inside the platform call. That arm \
         is supposed to resolve the path under the vault guard and then release it before calling \
         the host, precisely because handing a file to the OS can block on anything — see the lock \
         discipline in dispatch.rs. On the phone every command is serialised through one bridge, so \
         a guard held here is the entire app frozen for as long as the OS takes.",
    );
}
