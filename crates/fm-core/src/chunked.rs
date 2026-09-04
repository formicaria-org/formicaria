//! **Ingest in bounded slices, so a file's size stops being a memory limit.**
//!
//! Android leaves exactly one binary transport open (`fm_ingest`, base64 in a JSON argument —
//! see `mobile/src-tauri/src/lib.rs`), and a whole file crossing it as one string is copied
//! several times between the page and Rust. That is why `MAX_INGEST` exists at all: the ceiling
//! is *a memory limit wearing a size limit's clothes*, and it is what refuses video on a phone.
//!
//! Here the file arrives as a sequence of chunks appended to one session file, and the blob is
//! stored from that file by [`crate::ingest::ingest_file_named`] — which already streams and
//! hashes in 64 KB reads. **The transient peak is one chunk, whatever the file weighs.**
//!
//! **Content addressing is untouched.** The hash is taken once, over the assembled file, exactly
//! as it is for `fm add`. Chunks are pure transport and never reach the blob store; nothing here
//! can produce a blob whose name disagrees with its bytes.
//!
//! ## Where a session lives, and why not somewhere more obvious
//!
//! `<vault>/.fm-ingest/<session>/part`.
//!
//! - **Not in `blobs/`.** That directory is content-addressed and `verify` inventories it; a
//!   half-uploaded file sitting there would be reported as a blob whose name does not match its
//!   contents, which is the signature of bit-rot. An integrity checker must not be taught to
//!   expect corruption.
//! - **Not in the notes directory.** Everything there is a note.
//! - At the vault root, dot-prefixed, so `backup` — which snapshots the notes directory and
//!   `blobs/`, never the root — excludes it for free, and so does `verify`.
//!
//! ## The one new hazard, and the sweep
//!
//! A process killed mid-upload leaves a session directory behind. On a phone that is not a rare
//! event: Android kills backgrounded apps whenever it likes. So [`sweep`] runs at boot and
//! removes sessions older than [`SESSION_TTL`]. **Age, not "everything"** — two uploads can be in
//! flight across a restart on a device with a paired tablet, and deleting a live session would
//! turn a survivable interruption into a failed one.
//!
//! **Age is read from a stamp the session writes for itself, not from mtime.** A file's mtime is
//! not ours: a sync tool, a backup restore or a `cp -a` can move it in either direction, and a
//! sweep that deletes on somebody else's timestamp is a sweep that eventually eats a live upload.
//! The stamp is written once, when the session is created, and never touched again.

use crate::StoreError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How long an untouched session survives a sweep.
///
/// Long enough that a slow upload over a bad connection is never collected underneath itself,
/// short enough that a killed one does not sit in a user's vault indefinitely. A session is
/// bytes nobody asked to keep, so erring long costs disk and erring short costs an upload — the
/// asymmetry says which way to lean.
pub const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// The largest a single chunk may be.
///
/// **A limit on the transport, not on the file.** It exists so that a caller cannot reintroduce
/// the very problem this module solves by sending the whole file as "chunk 0" — which is exactly
/// what a well-meaning frontend does when the chunk size is left to it.
pub const MAX_CHUNK: usize = 4 * 1024 * 1024;

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

/// The directory every session lives under.
pub fn sessions_dir(vault: &Path) -> PathBuf {
    vault.join(".fm-ingest")
}

/// **A session id is validated, never trusted.** It arrives from a frontend and is used as a path
/// segment, so anything that could climb out of the sessions directory is refused here rather
/// than sanitised — a rewritten id would silently write somewhere the caller did not name.
fn session_dir(vault: &Path, session: &str) -> Result<PathBuf, StoreError> {
    let ok = !session.is_empty()
        && session.len() <= 64
        && session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !ok {
        return Err(StoreError::Io(format!(
            "'{session}' is not a usable upload id — letters, digits, '-' and '_' only, at most \
             64 of them"
        )));
    }
    Ok(sessions_dir(vault).join(session))
}

/// Append one chunk, and return how many bytes the session holds now.
///
/// **`seq` is checked, not decorative.** It is the index of this chunk counting from zero, and a
/// mismatch is refused. Without that check a dropped or duplicated chunk produces a file that is
/// merely *wrong* — it still hashes, still stores, still gets an asset note, and the corruption
/// surfaces later as an image that will not open. An out-of-order append is a transport failure
/// and has to fail like one, at the moment it happens, naming what it expected.
pub fn append(vault: &Path, session: &str, seq: u32, bytes: &[u8]) -> Result<u64, StoreError> {
    if bytes.is_empty() {
        return Err(StoreError::Io(
            "an upload chunk arrived with no bytes — nothing was appended. This is a transport \
             problem, not a problem with the file."
                .into(),
        ));
    }
    if bytes.len() > MAX_CHUNK {
        return Err(StoreError::Io(format!(
            "an upload chunk of {} bytes is over the {} byte limit — send smaller slices",
            bytes.len(),
            MAX_CHUNK
        )));
    }
    let dir = session_dir(vault, session)?;
    fs::create_dir_all(&dir).map_err(io)?;
    let part = dir.join("part");

    // Written once, at creation. See the module note: this is the clock `sweep` reads, and it is
    // deliberately ours rather than the filesystem's.
    let started = dir.join("started");
    if !started.exists() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        fs::write(&started, now.to_string()).map_err(io)?;
    }

    // The expected sequence number is derived from a counter file rather than from the part's
    // length, because chunks are not required to be a fixed size — the last one never is, and a
    // frontend may well slice differently on a slow connection.
    let seen = dir.join("seq");
    let expected: u32 = match fs::read_to_string(&seen) {
        Ok(t) => t.trim().parse().unwrap_or(0),
        Err(_) => 0,
    };
    if seq != expected {
        return Err(StoreError::Io(format!(
            "upload '{session}' expected chunk {expected} and got {seq} — a chunk was lost or \
             sent twice, so the file would be assembled wrong. Start the upload again."
        )));
    }

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .map_err(io)?;
    f.write_all(bytes).map_err(io)?;
    // **Durability before the counter moves.** If the process dies between the write and the
    // counter, the next chunk is refused as out of order and the upload restarts — which is
    // recoverable. The other order could accept a chunk whose bytes never landed, and that
    // assembles a file with a hole in it that nothing downstream can detect.
    f.sync_all().map_err(io)?;
    fs::write(&seen, (seq + 1).to_string()).map_err(io)?;

    Ok(fs::metadata(&part).map_err(io)?.len())
}

/// The assembled file for a session, ready to be ingested.
///
/// Fails when the session does not exist: finishing an upload that never started is a caller
/// error worth naming, not an empty file worth storing.
pub fn assembled(vault: &Path, session: &str) -> Result<PathBuf, StoreError> {
    let part = session_dir(vault, session)?.join("part");
    if !part.exists() {
        return Err(StoreError::Io(format!(
            "upload '{session}' has no bytes — it was never started, or it has already been \
             finished or swept"
        )));
    }
    Ok(part)
}

/// Remove a session's directory. Called after a successful ingest, and on an abandoned upload.
pub fn discard(vault: &Path, session: &str) -> Result<(), StoreError> {
    let dir = session_dir(vault, session)?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(io)?;
    }
    Ok(())
}

/// Delete sessions older than [`SESSION_TTL`], and report how many went.
///
/// **Best-effort by design.** A session that will not delete — a permission problem, a file held
/// open — must not stop the app from starting, so the error is skipped rather than raised. The
/// cost of leaving one behind is disk; the cost of refusing to boot is the whole notebook.
pub fn sweep(vault: &Path) -> usize {
    sweep_older_than(vault, SESSION_TTL)
}

/// [`sweep`], with the age spelled out — so a test can drive it without waiting a day or forging
/// a filesystem timestamp.
pub fn sweep_older_than(vault: &Path, ttl: Duration) -> usize {
    let dir = sessions_dir(vault);
    let Ok(entries) = fs::read_dir(&dir) else { return 0 };
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut removed = 0;
    for e in entries.flatten() {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        // **A session with no readable stamp is swept.** It is either from before this file
        // existed or damaged; either way nothing can finish it, and leaving unreadable bytes in a
        // user's vault forever is the worse of the two failures.
        let started: u64 = fs::read_to_string(path.join("started"))
            .ok()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
        if now.saturating_sub(started) > ttl.as_secs() && fs::remove_dir_all(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}
