//! Backup via restic — safety-critical and, until now, untested (the module's own
//! comment: "an untested backup is not a backup"). These spin a throwaway restic
//! repo in a tempdir and prove the full round-trip: init is idempotent, a backup
//! then restores the exact note bytes, and integrity check passes. Skipped (not
//! failed) when restic is absent, so `cargo test` stays green outside the pixi
//! env while `pixi run test` — where restic is on PATH — runs them for real.

use fm_core::backup;
use fm_core::Store;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn have(bin: &str) -> bool {
    Command::new(bin).arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// **`RESTIC_CACHE_DIR` is process-global and every restic test sets it.**
///
/// Cargo runs these on many threads in one process, so two tests setting it race: one points it
/// at a `TempDir` the other is about to drop, and restic then fails on a cache directory that
/// vanished underneath it. The failure lands on whichever test lost, which is why it read as
/// three unrelated tests breaking when a fourth was added on 2026-09-04 — the file had exactly
/// one setter until then, so the race had nowhere to happen.
///
/// **Every test in this file that runs restic must take this**, including the four that never set
/// the variable at all. Those were fine while nothing set it, and became the *victims* the moment
/// something did: they inherited a path whose `TempDir` had already been dropped. A test that does
/// not touch a shared global still races on it.
///
/// **The cost, stated:** this serialises the restic tests, and the file went from ~11 s to ~34 s.
/// That is the price of a process-global that `backup::backup` has no parameter for — the cache
/// directory is reachable only through the environment — and a correct 34 s beats a green 11 s
/// that fails one run in three.
///
/// Same reason and same shape as `fm-app/src/secrets.rs`'s `ENV` lock.
/// `unwrap_or_else(|e| e.into_inner())` so one panic does not poison the mutex and turn a single
/// failure into every failure.
static CACHE_ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Take the guard and point restic's cache at a directory this test owns.
///
/// Returns the guard **and** the `TempDir`: the caller has to hold both, because dropping the
/// directory while restic is still running is the bug this exists to prevent.
fn restic_cache() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = CACHE_ENV.lock().unwrap_or_else(|e| e.into_inner());
    let cache = tempdir().unwrap();
    std::env::set_var("RESTIC_CACHE_DIR", cache.path());
    (guard, cache)
}

#[test]
fn backup_restore_round_trips_and_check_passes() {
    if !have("restic") {
        eprintln!("skipping backup test: restic not on PATH");
        return;
    }

    // Keep restic's cache out of the user's ~/.cache — the subprocess inherits this process's
    // environment, so setting it here scopes the whole test. Serialised: see `restic_cache`.
    let (_env, _cache) = restic_cache();

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "correct horse battery staple";

    // A minimal vault: one note file with recognizable bytes (the durable data).
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    let contents = "---\ntype: note\n---\nthe durable knowledge\n";
    fs::write(notes.join("01.md"), contents).unwrap();

    // init is idempotent: created the first time, already-present the second.
    assert!(backup::ensure_repo(repo.path(), password).unwrap(), "repo created on first run");
    assert!(!backup::ensure_repo(repo.path(), password).unwrap(), "repo already exists on second");

    backup::backup(vault.path(), repo.path(), password).unwrap();

    // Restore into a fresh dir; restic recreates the source tree there. The note
    // must come back byte-for-byte — the half people skip.
    let dest = tempdir().unwrap();
    backup::restore(repo.path(), password, dest.path()).unwrap();
    let restored = find_file(dest.path(), "01.md").expect("restored note is present");
    assert_eq!(
        fs::read_to_string(&restored).unwrap(),
        contents,
        "note bytes survive backup → restore"
    );

    // Integrity check, re-reading and re-hashing every pack (the off-site scrub).
    backup::check(repo.path(), password, true).unwrap();
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    for e in fs::read_dir(root).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(found) = find_file(&p, name) {
                return Some(found);
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(p);
        }
    }
    None
}

/// **A vault may be a repo you already have**, and then its root also holds your source,
/// your `.env`, your `data/` and a `.git`. Snapshotting the root put all of that into
/// whatever restic repo the vault names — which for a lab's shared vault is not your repo.
///
/// So the snapshot takes what we know is ours (the notes dir and `blobs/`) rather than
/// excluding what is not, because the set to exclude has no end.
#[test]
fn a_projects_own_files_are_not_snapshotted() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();
    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "test-password";

    fs::create_dir_all(vault.path().join("notes")).unwrap();
    fs::write(vault.path().join("notes/01JQ.md"), "---\nid: x\n---\nours\n").unwrap();
    fs::create_dir_all(vault.path().join("blobs")).unwrap();
    fs::write(vault.path().join("blobs/blob-bytes"), "media\n").unwrap();

    // The things that must never leave the machine in someone else's restic repo.
    fs::write(vault.path().join(".env"), "SECRET=hunter2\n").unwrap();
    fs::create_dir_all(vault.path().join("src")).unwrap();
    fs::write(vault.path().join("src/lib.rs"), "fn theirs() {}\n").unwrap();

    backup::backup(vault.path(), repo.path(), password).unwrap();

    let listed = Command::new("restic")
        .args(["-r", repo.path().to_str().unwrap(), "ls", "latest"])
        .env("RESTIC_PASSWORD", password)
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);

    assert!(listed.contains("01JQ.md"), "our notes are backed up:\n{listed}");
    assert!(listed.contains("blob-bytes"), "our blobs are backed up:\n{listed}");
    assert!(!listed.contains(".env"), "a secret must not be in the snapshot:\n{listed}");
    assert!(!listed.contains("lib.rs"), "their code must not be either:\n{listed}");
}

/// The acquisition case, and the reason `restore_vault` exists at all: `restore` faithfully
/// recreates the *source's absolute path* under the target, so a plain restore of Ada's vault
/// hands you `dest/home/ada/vault/notes/…`. That is correct as a backup restore and useless as
/// a vault — `FileStore` would open `dest` and find no notes at all.
#[test]
fn restore_vault_lifts_the_tree_out_of_the_source_path() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();
    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "test-password";

    // A real vault, written through the real store — so what round-trips is a note this
    // codebase actually produces, not a hand-rolled approximation of one.
    {
        let mut s = fm_core::FileStore::named(vault.path(), "source").unwrap();
        let mut note = fm_model::Object::new(fm_model::Kind::Note, "the durable knowledge");
        note.title = Some("durable".into());
        s.put(&note).unwrap();
    }
    fs::create_dir_all(vault.path().join("blobs")).unwrap();
    fs::write(vault.path().join("blobs/some-hash"), "media\n").unwrap();
    assert!(vault.path().join("index.sqlite").exists(), "the source has an index");

    backup::backup(vault.path(), repo.path(), password).unwrap();

    let dest = tempdir().unwrap();
    let out = backup::restore_vault(repo.path(), password, dest.path()).unwrap();

    assert_eq!(out.notes_dir, "notes");
    assert!(out.had_blobs);
    assert!(!dest.path().join(".fm-restoring").exists(), "staging is cleaned up");
    assert_eq!(fs::read_to_string(dest.path().join("blobs/some-hash")).unwrap(), "media\n");

    // The per-machine index must never travel (the DB-corruption-by-sync lesson). `backup`
    // is what keeps it out, and this is the assertion that says so from the far end.
    assert!(
        !dest.path().join("index.sqlite").exists(),
        "the sender's index must not arrive with the vault"
    );

    // And the thing that actually matters: it opens as a vault, at <dest>, with the note in
    // it — not buried under the sender's home directory where `FileStore` would find nothing.
    let store = fm_core::FileStore::named(dest.path(), "restored").unwrap();
    let rows = store.query(&fm_query::Query::default()).unwrap().rows;
    assert_eq!(rows.len(), 1, "the restored note is visible in the vault");
    assert_eq!(rows[0].title.as_deref(), Some("durable"));
}

/// A `vault.json` puts the notes in `docs/`, and that file lives at the vault *root* — which
/// `backup` deliberately does not snapshot. Without reconstructing it, the notes come back
/// intact and the vault opens looking in `notes/`: every note invisible, nothing to say why.
#[test]
fn a_custom_notes_dir_survives_the_round_trip() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();
    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "test-password";

    fs::write(vault.path().join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
    {
        let mut s = fm_core::FileStore::named(vault.path(), "source").unwrap();
        s.put(&fm_model::Object::new(fm_model::Kind::Note, "kept in docs")).unwrap();
    }

    backup::backup(vault.path(), repo.path(), password).unwrap();

    let dest = tempdir().unwrap();
    let out = backup::restore_vault(repo.path(), password, dest.path()).unwrap();

    assert_eq!(out.notes_dir, "docs", "the snapshot's own paths are the last record of this");
    assert!(!out.had_blobs, "this vault never had any");

    // Without writing the descriptor back, this vault opens looking in `notes/` and shows
    // nothing at all — the notes are on disk and every view is empty, with no reason given.
    fm_core::descriptor::Descriptor { notes: Some(PathBuf::from(&out.notes_dir)), ..Default::default() }
        .write_new(dest.path())
        .unwrap();
    let store = fm_core::FileStore::named(dest.path(), "restored").unwrap();
    let rows = store.query(&fm_query::Query::default()).unwrap().rows;
    assert_eq!(rows.len(), 1, "the note in docs/ is visible once the descriptor is restored");
}

/// Refuse before moving anything. A half-moved vault is worse than a failed restore, and
/// "restore into the folder I already use" is an easy mistake to make.
#[test]
fn restoring_over_existing_content_refuses_and_changes_nothing() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();
    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "test-password";

    fs::create_dir_all(vault.path().join("notes")).unwrap();
    fs::write(vault.path().join("notes/01JQ.md"), "---\nid: x\n---\nbackup\n").unwrap();
    backup::backup(vault.path(), repo.path(), password).unwrap();

    // The destination already has notes of its own.
    let dest = tempdir().unwrap();
    fs::create_dir_all(dest.path().join("notes")).unwrap();
    fs::write(dest.path().join("notes/mine.md"), "---\nid: y\n---\nmine\n").unwrap();

    let err = backup::restore_vault(repo.path(), password, dest.path()).unwrap_err();

    assert!(format!("{err}").contains("already exists"), "says why: {err}");
    assert!(dest.path().join("notes/mine.md").exists(), "their note is untouched");
    assert!(!dest.path().join("notes/01JQ.md").exists(), "and nothing arrived");
    assert!(!dest.path().join(".fm-restoring").exists(), "staging is cleaned up on failure too");
}

/// A restic repo is frequently not ours. Restoring someone's photo backup as a vault because
/// it happened to be the latest snapshot is the kind of confident wrongness that costs a
/// directory, so the lookup is tag-filtered and an untagged repo simply has nothing to offer.
#[test]
fn a_repo_with_no_formicaria_snapshot_says_so() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();
    let repo = tempdir().unwrap();
    let password = "test-password";
    backup::ensure_repo(repo.path(), password).unwrap();

    // Someone else's backup, in the same repo, with no `fm` tag.
    let theirs = tempdir().unwrap();
    fs::write(theirs.path().join("holiday.jpg"), "not a note\n").unwrap();
    Command::new("restic")
        .args(["-r", repo.path().to_str().unwrap(), "backup", theirs.path().to_str().unwrap()])
        .env("RESTIC_PASSWORD", password)
        .output()
        .unwrap();

    assert_eq!(backup::latest(repo.path(), password).unwrap(), None, "not ours, not offered");

    let dest = tempdir().unwrap();
    let err = backup::restore_vault(repo.path(), password, dest.path()).unwrap_err();
    assert!(format!("{err}").contains("never backed up a vault"), "says why: {err}");
}

/// **MASTERPLAN's own S6 acceptance, which the suite did not keep** (added 2026-09-04).
///
/// The plan asks for `restic backup` → restore to a scratch dir → **diff the whole vault** →
/// `verify --scrub` clean, and calls it out in its own words: *"test the restore in month one."*
/// What existed was the first test in this file — one note's bytes and `check --read-data`. That
/// is a real test and it is not this one: comparing a file you remember writing cannot see a file
/// that was never snapshotted. **Every failure worth having a backup test for is a file that is
/// missing, and only a whole-tree diff can find one.**
///
/// So this walks both trees and compares the *sets* of relative paths as well as the bytes, with
/// a note, a blob, a `.view`, a theme and a custom-named notes directory in the vault — because
/// each of those is reached by different code in `backup`, and `blobs/` in particular is the half
/// the git tier deliberately does not carry.
///
/// Then it verifies the restored vault with `scrub: true`, which re-reads and re-hashes every
/// blob. A blob whose bytes came back wrong passes a path-and-length diff and fails here.
#[test]
fn the_whole_vault_survives_a_round_trip_and_verifies_scrubbed() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "correct horse battery staple";

    // A vault with one of everything the snapshot tier is supposed to carry.
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();

    // A blob, put through the real BlobStore so it is content-addressed exactly as it would be —
    // hand-placing bytes under `blobs/` would test a layout, not the store.
    let blobs = fm_core::BlobStore::new(vault.path());
    let src = vault.path().join("photo-source.bin");
    // Non-textual and long enough that a truncation would not look like a plausible file.
    let bytes: Vec<u8> = (0..64_000u32).map(|i| (i % 251) as u8).collect();
    fs::write(&src, &bytes).unwrap();
    let stored = blobs.put_file(&src).unwrap();
    fs::remove_file(&src).unwrap();

    // Referenced from frontmatter, which is where `verify` looks — a body-only mention is
    // invisible to it, and a test whose blob is unreferenced would not exercise the check that
    // a referenced blob is actually present.
    fs::write(
        notes.join("01.md"),
        format!(
            "---\nid: 01AAAAAAAAAAAAAAAAAAAAAAAA\ntype: note\ntitle: with an attachment\ncreated: 2026-09-04T00:00:00Z\nupdated: 2026-09-04T00:00:00Z\nassets:\n  - sha256:{}\n---\n\nsee the attachment\n",
            stored.hash
        ),
    )
    .unwrap();
    fs::write(
        notes.join("02.md"),
        "---\nid: 01BBBBBBBBBBBBBBBBBBBBBBBB\ntype: note\ntitle: plain\ncreated: 2026-09-04T00:00:00Z\nupdated: 2026-09-04T00:00:00Z\n---\n\nthe durable knowledge\n",
    )
    .unwrap();

    backup::backup(vault.path(), repo.path(), password).unwrap();

    // `restore_vault`, not `restore`: it lifts the tree out of the source's absolute path, which
    // is what makes the two trees comparable at all.
    let dest = tempdir().unwrap();
    let out = backup::restore_vault(repo.path(), password, dest.path()).unwrap();
    assert!(out.had_blobs, "the snapshot must have carried blobs/ at all");
    let restored = dest.path().to_path_buf();

    // ---- the whole-vault diff -------------------------------------------------------------
    //
    // **Sets first, then bytes.** A missing file is the failure a per-file comparison cannot see,
    // and it is the one that matters: a backup that quietly carries less than the vault looks
    // perfect until the day it is needed.
    let want = tree(vault.path());
    let got = tree(&restored);
    let missing: Vec<_> = want.iter().filter(|p| !got.contains(*p)).collect();
    assert!(
        missing.is_empty(),
        "the snapshot did not carry: {missing:?}\n  (restored tree: {got:?})"
    );
    for rel in &want {
        let a = fs::read(vault.path().join(rel)).unwrap();
        let b = fs::read(restored.join(rel)).unwrap();
        assert_eq!(a, b, "{} came back with different bytes", rel.display());
    }
    // The blob specifically, named rather than left to the loop, because it is the whole reason
    // this tier exists beside git.
    assert!(
        want.iter().any(|p| p.starts_with("blobs")),
        "the test vault must actually contain a blob, or this proves nothing: {want:?}"
    );

    // ---- verify --scrub on the restored vault ----------------------------------------------
    //
    // Re-reads and re-hashes every blob. A blob restored with correct length and wrong content
    // passes the diff above only if the bytes matched — this is the second, independent check
    // that the *content address* still holds after the round trip.
    let report = fm_core::verify::verify(&restored, true).unwrap();
    assert!(report.scrubbed, "the scrub must actually have run");
    assert!(
        report.ok(),
        "the restored vault does not verify: {:?}",
        report.issues
    );
    assert_eq!(report.notes, 2, "both notes are present and parse");
    assert_eq!(report.blobs, 1, "the blob is present and hashes to its own name");
}

/// Every file under `root`, as paths relative to it, sorted.
///
/// `.git` is skipped deliberately: the snapshot tier does not carry history and says so, so
/// including it would make this test assert the opposite of the design.
fn tree(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            // `.git` for the reason above; the SQLite index because it is explicitly disposable
            // and is rebuilt on open, so it is not part of what a backup owes anybody.
            if name == ".git" || name == "index.sqlite" || name == ".fm-restoring" {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    out
}

/// **A snapshot that says what it contained.**
///
/// `backup` answered unit until 2026-09-05, so a caller could report that a snapshot had been
/// taken and nothing whatever about what was in it — which is how the panel came to print the
/// same fixed phrase, *"notes and attachments"*, over vaults that have no attachments. Three
/// claims, and each is a different way the old return value could not be wrong because it said
/// nothing:
///
/// 1. **The directories reported are the ones that went in.** A vault keeping its notes in
///    `docs/` says `docs`, not the default; a vault with no `blobs/` says so rather than being
///    described as carrying attachments it does not have.
/// 2. **The numbers are restic's, not ours.** Backing the same vault up twice moves both files
///    from `files_new` to `files_unmodified`. Nothing this crate computes could do that, so the
///    assertion also proves the summary is parsed rather than invented.
/// 3. **The short id names the snapshot the way [`backup::latest`] does**, so *"what this run
///    contained"* and *"when this vault was last backed up"* can be joined by a reader.
///
/// Proven red twice: making `summary()` return `None` fails at *"restic describes the snapshot
/// it just wrote"*, and hardcoding `blobs: true` fails at *"this vault has no `blobs/`"*.
#[test]
fn a_snapshot_reports_what_went_into_it() {
    if !have("restic") {
        eprintln!("skipping: restic not on PATH");
        return;
    }
    let (_env, _cache) = restic_cache();

    let vault = tempdir().unwrap();
    let repo = tempdir().unwrap();
    let password = "correct horse battery staple";

    // Notes in `docs/`, so the reported name cannot be the hardcoded default coming back at us,
    // and one attachment so `blobs` has something true to say.
    fs::write(vault.path().join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
    let docs = vault.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("01.md"), "---\ntype: note\n---\nthe durable knowledge\n").unwrap();
    let blobs = vault.path().join("blobs");
    fs::create_dir_all(&blobs).unwrap();
    fs::write(blobs.join("aa.bin"), vec![7u8; 4096]).unwrap();

    let first = backup::backup(vault.path(), repo.path(), password).unwrap();
    assert_eq!(first.notes_dir.as_deref(), Some("docs"), "the notes directory it actually took");
    assert!(first.blobs, "`blobs/` exists, so it went in and the answer must say so");
    let c = first.contents.expect("restic describes the snapshot it just wrote");
    assert_eq!(c.id.len(), 8, "restic's short id, the way `latest` reports it: {}", c.id);
    assert_eq!(c.files_new, 2, "one note and one attachment, both new");
    assert_eq!(c.files_unmodified, 0, "nothing was already in an empty repository");
    assert!(c.bytes_added > 0, "a first snapshot grows the repository");
    assert!(c.bytes_processed >= 4096, "the attachment was read: {}", c.bytes_processed);

    // The same vault again, untouched. **These are restic's numbers**: nothing here could move
    // two files from `new` to `unmodified` between two identical calls.
    let again = backup::backup(vault.path(), repo.path(), password).unwrap();
    let c = again.contents.expect("the second snapshot is described too");
    assert_eq!(c.files_new, 0, "nothing is new the second time");
    assert_eq!(c.files_unmodified, 2, "both files were already in the repository");

    // A vault with notes and no attachments — the ordinary case, and the one the fixed phrase
    // was wrong about.
    let bare = tempdir().unwrap();
    let notes = bare.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("01.md"), "---\ntype: note\n---\nno attachments here\n").unwrap();
    let plain = backup::backup(bare.path(), repo.path(), password).unwrap();
    assert_eq!(plain.notes_dir.as_deref(), Some("notes"), "the default name, reported by name");
    assert!(!plain.blobs, "this vault has no `blobs/`, and the answer must not claim otherwise");
}
