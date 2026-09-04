//! **A file's size stops being a memory limit.**
//!
//! `MAX_INGEST` (16 MB, `ui/src/lib/ipc.ts`) is not a policy about how large an attachment should
//! be — it is the point at which base64-in-a-JSON-argument, copied several times between the page
//! and Rust, stops fitting on a phone. `outstanding.md` §1.3 called it *"a memory limit wearing a
//! size limit's clothes"*, and it is what refuses video on Android.
//!
//! The chunked path sends bounded slices into a session file and stores the blob from that file,
//! streaming. What is pinned here is the three properties that make it safe to prefer:
//!
//!  1. **A file well over the ceiling arrives whole and hashes correctly.** The test that proves
//!     the refusal is lifted.
//!  2. **The content address is the same either way.** Chunks are transport; a file ingested in
//!     one piece and the same file ingested in forty must produce one blob, not two.
//!  3. **A lost or repeated chunk is refused at the moment it happens**, not assembled into a
//!     file that hashes fine and is quietly wrong.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::json;
use std::path::{Path, PathBuf};
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
        vec![VaultConfig {
            name: "notes".into(),
            path: vault.path().to_path_buf(),
            restic: None,
        }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

fn call(app: &App, cmd: &str, args: serde_json::Value, body: &[u8]) -> Result<String, String> {
    dispatch(cmd, &args, body, app, &NoHost).map(|o| String::from_utf8_lossy(&o.into_bytes()).into_owned())
}

/// Deterministic, non-compressible, and not a repeating byte — a file of zeros would hash the
/// same however badly it were assembled out of order.
fn payload(len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    let mut x: u32 = 0x9E37_79B9;
    while v.len() < len {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.extend_from_slice(&x.to_le_bytes());
    }
    v.truncate(len);
    v
}

/// The `sha256:…` reference an ingest wrote onto its asset note.
///
/// Read out of the command's own answer rather than recomputed here: hashing the bytes a second
/// time in the test would only prove that this file's `sha2` agrees with `fm-core`'s. What is
/// worth asserting is that the blob **stored under that name** contains the bytes that were sent.
fn hash_from(meta: &str) -> String {
    let i = meta.find("sha256:").expect("the asset note carries a blob reference");
    meta[i + 7..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect()
}

/// **The one that proves the refusal is lifted.** 24 MB is comfortably over the 16 MB ceiling the
/// single-shot path has to keep, and it arrives in 3 MB slices — so the transient peak is a slice,
/// not the file. Every byte is checked, on the blob, by re-hashing it.
#[test]
fn a_file_over_the_single_shot_ceiling_arrives_whole() {
    let (_home, vault, app) = app();
    let bytes = payload(24 * 1024 * 1024);

    let chunk = 3 * 1024 * 1024;
    for (seq, slice) in bytes.chunks(chunk).enumerate() {
        let out = call(
            &app,
            "ingest_chunk",
            json!({ "session": "s1", "seq": seq, "vault": "notes" }),
            slice,
        )
        .unwrap_or_else(|e| panic!("chunk {seq}: {e}"));
        assert!(out.contains("\"received\""), "{out}");
    }

    let out = call(
        &app,
        "ingest_finish",
        json!({ "session": "s1", "name": "clip.mp4", "vault": "notes" }),
        &[],
    )
    .expect("a 24 MB file must ingest through the chunked path");
    assert!(out.contains("clip.mp4"), "the asset note is named after the file: {out}");

    // **The assertion that matters.** A file assembled wrongly still produces *a* blob and *a*
    // note; what it cannot produce is a blob whose bytes equal the ones that were sent. Compared
    // byte for byte rather than by re-hashing, so a hash function that agreed with itself about
    // the wrong bytes could not make this pass.
    let want = hash_from(&out);
    let blob = find_blob(vault.path(), &want).expect("the blob is stored under its own hash");
    assert_eq!(
        std::fs::read(&blob).unwrap(),
        bytes,
        "the stored blob is not the file that was sent"
    );

    // The session is gone: a finished upload leaves nothing behind.
    assert!(
        !fm_core::chunked::sessions_dir(vault.path()).join("s1").exists(),
        "a finished session is discarded"
    );
}

/// **Chunks are transport, so the identity is unchanged.** The same bytes sent in one piece and
/// in many must be one blob. If this ever fails, content addressing has been broken by the
/// transport — which would mean the same file stored twice under two names, and dedup silently
/// off for everything that arrived by phone.
#[test]
fn the_same_file_has_the_same_address_whichever_way_it_arrives() {
    let (_home, vault, app) = app();
    let bytes = payload(700_000);

    let single = call(&app, "ingest", json!({ "name": "a.bin", "vault": "notes" }), &bytes).unwrap();
    let one = blob_count(vault.path());

    for (seq, slice) in bytes.chunks(64 * 1024).enumerate() {
        call(&app, "ingest_chunk", json!({ "session": "s2", "seq": seq, "vault": "notes" }), slice)
            .unwrap();
    }
    let chunked =
        call(&app, "ingest_finish", json!({ "session": "s2", "name": "b.bin", "vault": "notes" }), &[])
            .unwrap();

    assert_eq!(
        hash_from(&single),
        hash_from(&chunked),
        "the same bytes must have the same content address whichever way they arrived"
    );
    assert_eq!(
        blob_count(vault.path()),
        one,
        "and therefore one blob, not two — dedup still works for anything sent from a phone"
    );
}

/// **A dropped or repeated chunk fails loudly, at the moment it happens.**
///
/// Without the sequence check this is the worst failure this module could have: the file still
/// assembles, still hashes, still stores, still gets an asset note — and is wrong. It would
/// surface days later as an image that will not open, with nothing to connect it to the upload.
#[test]
fn an_out_of_order_chunk_is_refused_rather_than_assembled() {
    let (_home, _vault, app) = app();
    let bytes = payload(300_000);
    let mut it = bytes.chunks(100_000);
    let (c0, c1) = (it.next().unwrap(), it.next().unwrap());

    call(&app, "ingest_chunk", json!({ "session": "s3", "seq": 0, "vault": "notes" }), c0).unwrap();

    // Skipping 1: the exact shape of a dropped chunk.
    let e = call(&app, "ingest_chunk", json!({ "session": "s3", "seq": 2, "vault": "notes" }), c1)
        .unwrap_err();
    assert!(e.contains("expected chunk 1"), "the refusal must say what it wanted: {e}");

    // Repeating 0: the exact shape of a retried chunk that already landed.
    let e = call(&app, "ingest_chunk", json!({ "session": "s3", "seq": 0, "vault": "notes" }), c0)
        .unwrap_err();
    assert!(e.contains("expected chunk 1"), "{e}");
}

/// An empty chunk is a transport failure, exactly as an empty single-shot ingest is — and for the
/// same reason: accepting it hides a broken byte path behind a success message.
#[test]
fn an_empty_chunk_is_refused() {
    let (_home, _vault, app) = app();
    let e = call(&app, "ingest_chunk", json!({ "session": "s4", "seq": 0, "vault": "notes" }), &[])
        .unwrap_err();
    assert!(e.contains("no bytes"), "{e}");
}

/// A session id is a path segment, so it is validated rather than sanitised. A rewritten id would
/// write somewhere the caller did not name, silently.
#[test]
fn a_session_id_cannot_escape_the_sessions_directory() {
    let (_home, vault, app) = app();
    for bad in ["../escape", "a/b", "", "with space", "..", "x\0y"] {
        let e = call(
            &app,
            "ingest_chunk",
            json!({ "session": bad, "seq": 0, "vault": "notes" }),
            b"hello",
        )
        .unwrap_err();
        assert!(!e.is_empty(), "'{bad}' must be refused");
    }
    assert!(
        !vault.path().parent().unwrap().join("escape").exists(),
        "nothing was written outside the vault"
    );
}

/// Finishing an upload that never started is a caller error worth naming, not an empty file worth
/// storing — the same stance as the single-shot path's zero-byte refusal.
#[test]
fn finishing_an_upload_that_never_started_says_so() {
    let (_home, _vault, app) = app();
    let e = call(
        &app,
        "ingest_finish",
        json!({ "session": "never", "name": "x.bin", "vault": "notes" }),
        &[],
    )
    .unwrap_err();
    assert!(e.contains("no bytes"), "{e}");
}

/// Cancelling reclaims the bytes now rather than at the next sweep.
#[test]
fn cancelling_an_upload_reclaims_its_bytes() {
    let (_home, vault, app) = app();
    call(&app, "ingest_chunk", json!({ "session": "s5", "seq": 0, "vault": "notes" }), b"partial")
        .unwrap();
    let dir = fm_core::chunked::sessions_dir(vault.path()).join("s5");
    assert!(dir.exists());

    call(&app, "ingest_cancel", json!({ "session": "s5", "vault": "notes" }), &[]).unwrap();
    assert!(!dir.exists(), "cancel removes the session immediately");
}

/// **The sweep collects an abandoned session and leaves a live one alone.**
///
/// Age, not "everything": two uploads can be in flight across a restart on a device with a paired
/// tablet, and collecting a live one turns a survivable interruption into a failed upload.
#[test]
fn the_sweep_takes_stale_sessions_and_spares_fresh_ones() {
    let (_home, vault, app) = app();
    call(&app, "ingest_chunk", json!({ "session": "old", "seq": 0, "vault": "notes" }), b"abandoned")
        .unwrap();
    call(&app, "ingest_chunk", json!({ "session": "live", "seq": 0, "vault": "notes" }), b"in flight")
        .unwrap();

    // Age the abandoned one by rewriting the stamp it wrote for itself — which is the field
    // `sweep` actually reads, so this drives the real code path rather than forging a filesystem
    // timestamp the sweep was taught to ignore.
    let dir = fm_core::chunked::sessions_dir(vault.path());
    let long_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - fm_core::chunked::SESSION_TTL.as_secs() * 2;
    std::fs::write(dir.join("old").join("started"), long_ago.to_string()).unwrap();

    assert_eq!(fm_core::chunked::sweep(vault.path()), 1, "exactly the stale one");
    assert!(!dir.join("old").exists(), "the abandoned session is gone");
    assert!(
        fm_core::chunked::sessions_dir(vault.path()).join("live").exists(),
        "an upload still in flight must survive a sweep"
    );
}

fn blob_count(vault: &Path) -> usize {
    fn walk(p: &Path, n: &mut usize) {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                walk(&path, n);
            } else {
                *n += 1;
            }
        }
    }
    let mut n = 0;
    walk(&vault.join("blobs"), &mut n);
    n
}

fn find_blob(vault: &Path, hash: &str) -> Option<PathBuf> {
    fn walk(p: &Path, hash: &str) -> Option<PathBuf> {
        for e in std::fs::read_dir(p).ok()?.flatten() {
            let path = e.path();
            if path.is_dir() {
                if let Some(f) = walk(&path, hash) {
                    return Some(f);
                }
            } else if path.file_name().and_then(|n| n.to_str()) == Some(hash) {
                return Some(path);
            }
        }
        None
    }
    walk(&vault.join("blobs"), hash)
}
