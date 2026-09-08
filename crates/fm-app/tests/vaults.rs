//! `inspect_path` — the facts the create-vault form answers with, and the one claim it
//! makes about someone else's directory: that notes already there will be adopted.

use fm_app::commands::inspect_path;
use fm_core::{FileStore, Store};
use fm_query::Query;
use tempfile::tempdir;

/// The load-bearing one. `inspect_path` tells the user "this directory already has N
/// notes — they'll be adopted", and **that promise is kept by no code at all**:
/// `FileStore::named` reindexes whatever `notes/*.md` it finds. If that ever stops being
/// true, the form starts lying, so pin it here rather than trusting the comment.
#[test]
fn notes_already_on_disk_are_adopted_with_no_import_step() {
    let d = tempdir().unwrap();
    let v = d.path().join("someone-elses-folder");
    std::fs::create_dir_all(v.join("notes")).unwrap();

    // Hand-written, exactly as a text editor would leave them.
    for (id, title) in [
        ("01KXJ6629KJCAYMYNWYMM9KM46", "first"),
        ("01KXJPK8PPCNCPWGYTP0FR37VM", "second"),
        ("01KXMHDRQMB6P02X1PM49S53SJ", "third"),
    ] {
        std::fs::write(
            v.join("notes").join(format!("{id}.md")),
            format!(
                "---\nschema: 1\nid: {id}\ntype: note\ntitle: {title}\n\
                 created: 2026-07-15T06:07:19.859303633Z\n\
                 updated: 2026-07-16T07:37:23.775709462Z\n---\nbody of {title}\n"
            ),
        )
        .unwrap();
    }

    // The form's claim...
    let facts = inspect_path(&v);
    assert_eq!(facts.notes, 3, "the form must see them before promising anything");
    assert!(!facts.empty);

    // ...and the claim being kept, by opening it exactly as `create_vault` does.
    let store = FileStore::named(&v, "adopted").unwrap();
    let r = store.query(&Query::default()).unwrap();
    assert_eq!(r.total, 3, "adoption is free — no import step, no migration");
    let mut titles: Vec<&str> = r.rows.iter().filter_map(|o| o.title.as_deref()).collect();
    titles.sort();
    assert_eq!(titles, ["first", "second", "third"]);
}

#[test]
fn a_missing_path_reports_its_parent_as_the_thing_wed_create_in() {
    let d = tempdir().unwrap();

    // Parent exists: one directory to make.
    let f = inspect_path(&d.path().join("new-vault"));
    assert!(!f.exists);
    assert!(!f.parent_missing);
    assert!(f.writable, "probed against the parent, which exists and is writable");

    // Parent doesn't: a chain to make. Still writable — the probe walks up to one
    // that exists, because "can I create this?" is a question about an ancestor.
    let f = inspect_path(&d.path().join("a").join("b").join("c"));
    assert!(!f.exists);
    assert!(f.parent_missing);
    assert!(f.writable);
}

#[test]
fn a_file_is_not_a_directory_and_says_so() {
    let d = tempdir().unwrap();
    let f = d.path().join("notes.md");
    std::fs::write(&f, "I am a file").unwrap();

    let facts = inspect_path(&f);
    assert!(facts.exists);
    assert!(facts.not_a_directory);
    assert!(!facts.empty, "a file is not an empty directory");
}

#[test]
fn an_empty_directory_is_the_happy_path() {
    let d = tempdir().unwrap();
    let v = d.path().join("fresh");
    std::fs::create_dir(&v).unwrap();

    let f = inspect_path(&v);
    assert!(f.exists && f.empty && f.writable);
    assert_eq!(f.notes, 0);
    assert!(!f.not_a_directory && !f.git_repo && !f.parent_missing);
}

#[test]
fn an_existing_git_repo_is_reported_not_refused() {
    let d = tempdir().unwrap();
    let v = d.path().join("my-paper");
    std::fs::create_dir_all(v.join(".git")).unwrap();

    assert!(inspect_path(&v).git_repo, "we leave its history alone, but we must say we saw it");
}

/// The write probe is the whole point of `writable`: mode bits are configuration, and a
/// capability must mean "this will work". Skipped as root, who can write anyway — the
/// probe correctly returns true there, which is untestable rather than wrong.
#[cfg(unix)]
#[test]
fn a_read_only_directory_is_not_writable() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc_geteuid() } == 0 {
        eprintln!("skipped: running as root, who can write to anything");
        return;
    }
    let d = tempdir().unwrap();
    let v = d.path().join("locked");
    std::fs::create_dir(&v).unwrap();
    std::fs::set_permissions(&v, std::fs::Permissions::from_mode(0o555)).unwrap();

    let f = inspect_path(&v);
    assert!(!f.writable, "the probe must actually try, not read the bits");

    // Leave it removable by the tempdir teardown.
    std::fs::set_permissions(&v, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// `geteuid` without pulling in the `libc` crate for one call in one test.
#[cfg(unix)]
unsafe fn libc_geteuid() -> u32 {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() }
}

/// **The vault list is never overwritten when it cannot be understood**, and nothing checked it.
///
/// `save`'s three refusals — not JSON, not an object, `"vaults"` not an array — are the only thing
/// between a hand-edited `vaults.json` and losing every vault registration on this machine. The
/// notes themselves are safe (they are files on disk), but a user whose list is gone opens the app
/// to a first-run screen with their work sitting in directories nothing refers to any more. That is
/// the failure `dispatch.rs` names: *overwriting a hand-edited file we could not parse.*
///
/// **Deliberately not a permissions test.** The suite's other write-refusal test skips as root,
/// which is every container; these three branches need no permissions at all, so they always run.
///
/// The assertion is the file's **bytes**, before and after — not the error text. A refusal that
/// still rewrote the file would satisfy any message-shaped check.
///
/// Proven red by replacing each `?` refusal with a fallback to an empty object: the malformed file
/// is then silently replaced by a well-formed one holding only the vault being added.
#[test]
fn a_vault_list_that_will_not_parse_is_refused_and_left_byte_identical() {
    use fm_app::vaults::{save, VaultConfig};

    let d = tempdir().unwrap();
    let one =
        vec![VaultConfig { name: "notes".into(), path: d.path().join("notes"), restic: None }];

    for (label, content) in [
        ("truncated mid-object", "{\n  \"vaults\": [\n"),
        ("a comment, which JSON has no such thing as", "{ // my vaults\n  \"vaults\": []\n}\n"),
        ("valid JSON, but an array at the root", "[{\"name\":\"notes\"}]\n"),
        ("valid JSON, but \"vaults\" is an object", "{\"vaults\": {\"notes\": \"/x\"}}\n"),
    ] {
        let list = d.path().join("vaults.json");
        std::fs::write(&list, content).unwrap();

        let err = save(&one, &list).expect_err(&format!("must refuse: {label}"));
        assert!(err.contains("fix it first"), "the refusal must tell them what to do: {err}");
        assert_eq!(
            std::fs::read_to_string(&list).unwrap(),
            content,
            "the file must be byte-identical after a refusal ({label})"
        );
    }
}

/// **A key we do not know about survives a save.** `save`'s doc is explicit that it merges into the
/// parsed tree rather than doing a typed round-trip, precisely so *"every entry we did not create
/// keeps its own bytes, and unknown top-level keys survive"* — turning "I don't understand this"
/// into "I silently dropped it" is the failure it was shaped to avoid.
///
/// Nothing asserted it. A future refactor to `#[derive(Serialize)]` over a typed struct would look
/// tidier, pass every other test here, and quietly delete whatever the user had added.
///
/// Proven red by serializing a typed struct instead of merging: `theme` disappears and the
/// hand-written entry is reformatted.
#[test]
fn a_save_keeps_keys_and_entries_it_did_not_write() {
    use fm_app::vaults::{save, VaultConfig};

    let d = tempdir().unwrap();
    let list = d.path().join("vaults.json");
    std::fs::write(
        &list,
        "{\n  \"theme\": \"solarized\",\n  \"vaults\": [\n    { \"name\": \"lab\", \
         \"path\": \"/srv/lab\", \"note\": \"shared with the group\" }\n  ]\n}\n",
    )
    .unwrap();

    save(&[VaultConfig { name: "personal".into(), path: d.path().join("p"), restic: None }], &list)
        .unwrap();

    let after = std::fs::read_to_string(&list).unwrap();
    assert!(after.contains("solarized"), "an unknown top-level key survived: {after}");
    assert!(after.contains("shared with the group"), "so did a key inside their entry: {after}");
    assert!(after.contains("\"personal\""), "and the new vault was appended: {after}");
    assert!(after.contains("\"lab\""), "beside the one that was already there: {after}");
}

/// **The form's note count follows the vault's own `notes:` setting.**
///
/// `inspect_path` is what draws "This folder already has N notes in it" — the line a person reads
/// when pointing the app at a folder, and the one that says whether they are about to adopt work or
/// start empty. A vault may keep its notes anywhere (`vault.json`'s `notes:`), and counting `notes/`
/// regardless would tell someone their vault was empty while it holds two hundred.
///
/// Pinned here because the same wrapper was re-implemented four times and one copy
/// (`recoverable_vaults`, 2026-09-08) did exactly that. All four now ask
/// `fm_core::descriptor::note_count`, and this is the assertion that notices if one stops.
///
/// Proven red by counting `<root>/notes` instead of the descriptor's directory.
#[test]
fn the_note_count_follows_the_vaults_own_notes_directory() {
    let d = tempdir().unwrap();
    let v = d.path().join("elsewhere");
    std::fs::create_dir_all(v.join("docs")).unwrap();
    std::fs::write(v.join("vault.json"), r#"{"notes":"docs"}"#).unwrap();
    for id in ["01KXJ6629KJCAYMYNWYMM9KM46", "01KXJPK8PPCNCPWGYTP0FR37VM"] {
        std::fs::write(
            v.join("docs").join(format!("{id}.md")),
            format!(
                "---\nschema: 1\nid: {id}\ntype: note\ntitle: t\n\
                 created: 2026-07-15T06:07:19Z\nupdated: 2026-07-16T07:37:23Z\n---\nbody\n"
            ),
        )
        .unwrap();
    }
    // An empty `notes/` beside it, which is what the hardcoded version would have counted.
    std::fs::create_dir_all(v.join("notes")).unwrap();

    let facts = inspect_path(&v);
    assert_eq!(facts.notes, 2, "the two notes in `docs/` are found, not the empty `notes/`");
    assert!(!facts.empty, "and the folder is not reported as empty");
}
