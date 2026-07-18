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
