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

#[test]
fn backup_restore_round_trips_and_check_passes() {
    if !have("restic") {
        eprintln!("skipping backup test: restic not on PATH");
        return;
    }

    // Keep restic's cache out of the user's ~/.cache — the subprocess inherits
    // this process's environment, so setting it here scopes the whole test.
    let cache = tempdir().unwrap();
    std::env::set_var("RESTIC_CACHE_DIR", cache.path());

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
