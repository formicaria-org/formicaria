//! FileStore proves S0's pipeline (write -> disk -> reindex -> read) and that it
//! honours the SAME `Store` contract as `MemoryStore`.

use fm_core::{FileStore, MemoryStore, Reindex, Store, StoreError};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use tempfile::tempdir;

fn seed(store: &mut dyn Store) {
    let mut a = Object::new(Kind::Note, "trust region clipping");
    a.status = Some("doing".into());
    a.tags = vec!["meta-rl".into()];
    let mut b = Object::new(Kind::Note, "advantage estimator");
    b.status = Some("done".into());
    let c = Object::new(Kind::Note, "idea about GAE lambda");
    store.put(&a).unwrap();
    store.put(&b).unwrap();
    store.put(&c).unwrap();
}

/// The S0 acceptance test: type -> atomic write -> reindex on reload -> still there.
#[test]
fn write_reindex_read() {
    let dir = tempdir().unwrap();
    let vault = dir.path();

    let id = {
        let mut s = FileStore::open(vault).unwrap();
        let o = Object::new(Kind::Note, "still here after reload");
        let id = o.id;
        s.put(&o).unwrap();
        id
    };

    // A real Markdown file landed on disk.
    assert!(vault.join(format!("notes/{id}.md")).exists());

    // "Reload": a brand-new FileStore reindexes from files and still has it.
    let s2 = FileStore::open(vault).unwrap();
    let got = s2.get(id).unwrap().expect("note survives reload");
    assert_eq!(got.body, "still here after reload");
}

/// The index is genuinely disposable: delete it, reopen, it rebuilds from files.
#[test]
fn index_is_disposable() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    {
        let mut s = FileStore::open(vault).unwrap();
        seed(&mut s);
    }
    std::fs::remove_file(vault.join("index.sqlite")).unwrap();

    let s = FileStore::open(vault).unwrap();
    let r = s.query(&Query::default()).unwrap();
    assert_eq!(r.total, 3);
}

/// A note written on Windows — or handed to us by git's `core.autocrlf`, the default
/// there — has `\r\n` line endings. `to_file` only ever writes `\n`, but the file on disk
/// is not always ours: any Windows editor, and every checkout of a shared vault, produces
/// CRLF. A parser that only knows `\n` rejects **every** note, and because the loader is
/// deliberately tolerant they don't error — they vanish, and the vault opens empty.
///
/// Found by CI the first time it ran on Windows: all eight `e2e_vault` tests failed at
/// once, because they're the only ones that read *committed* fixture notes rather than
/// writing their own.
#[test]
fn a_note_with_windows_line_endings_loads_and_keeps_its_body_byte_for_byte() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    std::fs::create_dir_all(vault.join("notes")).unwrap();

    // Exactly what `to_file` writes, run through git's autocrlf.
    let lf = "---\nschema: 1\nid: 01JQ0000000000000000000000\ntype: note\ncreated: 2026-07-17T10:00:00Z\nupdated: 2026-07-17T10:00:00Z\n---\nfirst line\n\nsecond line\n";
    let crlf = lf.replace('\n', "\r\n");
    std::fs::write(vault.join("notes/01JQ0000000000000000000000.md"), &crlf).unwrap();

    let s = FileStore::named(vault, "win").unwrap();
    assert!(s.skipped().is_empty(), "a CRLF note is a note, not a casualty: {:?}", s.skipped());
    let got =
        s.get("01JQ0000000000000000000000".parse().unwrap()).unwrap().expect("the note loads");

    // The body is sliced, never rewritten — byte-for-byte is the invariant files-as-truth
    // rests on, so the author's line endings survive until the author changes them.
    assert_eq!(got.body, "first line\r\n\r\nsecond line\r\n", "body preserved verbatim");
    assert_eq!(got.title, None);
}

/// One unreadable note must never stop the vault from opening.
///
/// This is the shape a merge conflict *always* takes here: `updated:` is rewritten
/// on every save, so any two concurrent edits to one note collide on that line, and
/// git's markers land inside the YAML fence where `from_file` rightly refuses them.
/// Before the tolerant loader that single file failed `FileStore::open` — so the
/// first conflict in a shared vault bricked the app for everyone, and a vault that
/// will not open is one whose conflict UI can never render to fix it.
#[test]
fn a_conflicted_note_is_skipped_and_named_rather_than_fatal() {
    let dir = tempdir().unwrap();
    let vault = dir.path();

    let good = {
        let mut s = FileStore::open(vault).unwrap();
        let o = Object::new(Kind::Note, "every other note in the vault");
        let id = o.id;
        s.put(&o).unwrap();
        id
    };

    // Exactly what git leaves on disk when two people saved the same note.
    std::fs::write(
        vault.join("notes/conflicted.md"),
        "---\n\
         id: 01JQ0000000000000000000000\n\
         type: note\n\
         title: two people saved this\n\
         created: 2026-07-17T10:00:00Z\n\
         <<<<<<< HEAD\n\
         updated: 2026-07-17T11:00:00Z\n\
         =======\n\
         updated: 2026-07-17T11:05:00Z\n\
         >>>>>>> theirs\n\
         ---\n\
         \n\
         the body survived; the frontmatter did not\n",
    )
    .unwrap();

    let mut s = FileStore::open(vault).expect("the vault opens despite the conflict");
    assert!(s.get(good).unwrap().is_some(), "and every readable note still serves");

    let stats = s.reindex(Reindex::Full).unwrap();
    assert_eq!(stats.scanned, 2, "both files were looked at");
    assert_eq!(stats.updated, 1, "only the readable one indexed");
    assert_eq!(stats.skipped.len(), 1, "the conflict was skipped, not fatal");
    assert!(
        stats.skipped[0].name == "conflicted.md",
        "and named, so the user can be told which file to fix: {:?}",
        stats.skipped,
    );
}

/// **Location is the permission.** A vault is one repo, one remote, one collaborator
/// list, so where a note *is* decides who can see it — and git history is forever, so a
/// permission you could typo would be permanent disclosure to everyone who ever cloned.
/// `Object.vault` therefore has to be derived and never serialized. The plan asks for
/// this test before anything else in Phase 2: `vault:` must never appear in a `.md`.
#[test]
fn vault_is_derived_from_location_and_never_written_to_a_file() {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("lab-notes");
    let mut s = FileStore::named(&vault, "lab").unwrap();

    let mut o = Object::new(Kind::Note, "a shared finding");
    // Someone tries to grant themselves an audience by typing it, or an agent does it
    // by mistake. Neither can work: the field is not content.
    o.extra.insert("vault".into(), fm_model::PropertyValue::Text("personal".into()));
    let id = o.id;
    s.put(&o).unwrap();

    let raw = std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap();
    assert!(!raw.contains("vault:"), "the permission never reaches the file:\n{raw}");

    // What the store says is what the location says — not what anyone typed.
    let got = s.get(id).unwrap().unwrap();
    assert_eq!(got.vault, "lab", "derived from where it lives");
    assert_eq!(got.get("vault"), fm_model::PropertyValue::Text("lab".into()), "and queries agree");

    // A hand-typed `vault:` in a file on disk is stripped, not honoured and not kept.
    let hand_typed = raw.replace("---\n\n", "vault: everyone\n---\n\n");
    std::fs::write(vault.join(format!("notes/{id}.md")), &hand_typed).unwrap();
    let mut s2 = FileStore::named(&vault, "lab").unwrap();
    let got = s2.get(id).unwrap().unwrap();
    assert_eq!(got.vault, "lab", "a file cannot talk its way into another audience");

    // And it does not survive a rewrite: no permission-shaped string left implying it
    // means something.
    s2.put(&got).unwrap();
    let rewritten = std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap();
    assert!(!rewritten.contains("vault:"), "stripped, not round-tripped:\n{rewritten}");
}

/// An incremental reindex re-reads only what changed — the thing that makes a 3 s
/// poll affordable rather than a full re-parse of the vault every 3 seconds. It must
/// see the whole story an external writer can tell: a change, an addition, and a
/// deletion (all three are what a `git pull` does).
#[test]
fn an_incremental_reindex_sees_external_changes_without_re_reading_the_vault() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    let mut s = FileStore::open(vault).unwrap();
    seed(&mut s);

    // Nothing has moved: three files looked at, none re-parsed.
    let quiet = s.reindex(Reindex::Incremental).unwrap();
    assert_eq!(quiet.scanned, 3, "every file is still checked — that part is a stat");
    assert_eq!(quiet.updated, 0, "but nothing was re-read: this is the poll's common case");

    // Now a "pull": one note rewritten, one added, one removed — none of it through
    // the app, which is the whole point.
    let ids: Vec<_> = s.query(&Query::default()).unwrap().rows.iter().map(|o| o.id).collect();
    let changed = vault.join(format!("notes/{}.md", ids[0]));
    let removed = vault.join(format!("notes/{}.md", ids[1]));
    std::thread::sleep(std::time::Duration::from_millis(10));
    let text = std::fs::read_to_string(&changed).unwrap();
    std::fs::write(&changed, text.replace("trust region clipping", "their better paragraph"))
        .unwrap();
    std::fs::remove_file(&removed).unwrap();
    std::fs::write(
        vault.join("notes/01JQ2222222222222222222222.md"),
        "---\nid: 01JQ2222222222222222222222\ntype: note\ntitle: theirs\ncreated: 2026-07-17T10:00:00Z\nupdated: 2026-07-17T10:00:00Z\n---\n\narrived in a pull\n",
    )
    .unwrap();

    let after = s.reindex(Reindex::Incremental).unwrap();
    assert_eq!(after.updated, 2, "only the changed and the new file were re-read");

    // And the index now agrees with the disk, which is what a pull has to become
    // visible through: `get`/`query` serve SQLite, never the file.
    assert_eq!(s.get(ids[1]).unwrap(), None, "the deleted note is gone from the index");
    assert!(
        s.get(ids[0]).unwrap().unwrap().body.contains("their better paragraph"),
        "the external edit is visible",
    );
    assert!(
        s.query(&Query::default())
            .unwrap()
            .rows
            .iter()
            .any(|o| o.title.as_deref() == Some("theirs")),
        "and so is the note that arrived",
    );
}

/// *The* lost-update bug. `get` serves SQLite, not the file, so once anything else
/// writes a note — a `git pull`, a merge driver, Vim — every writer above `put` is
/// holding a stale copy and would rewrite the whole file from it. Refusing is the
/// only safe answer: the caller must re-read and decide.
#[test]
fn a_note_that_changed_on_disk_refuses_the_write_that_would_erase_it() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    let mut s = FileStore::open(vault).unwrap();

    let mut o = Object::new(Kind::Note, "my half of the note");
    let id = o.id;
    s.put(&o).unwrap();

    // Someone else writes the file — a pull landing a collaborator's paragraph. The
    // app never sees it: `get` reads the index, and reindex only runs at `open`.
    let path = vault.join(format!("notes/{id}.md"));
    let theirs = std::fs::read_to_string(&path)
        .unwrap()
        .replace("my half of the note", "my half of the note\n\ntheir hard-won paragraph");
    // A distinct mtime, without depending on the clock ticking between two writes.
    std::thread::sleep(std::time::Duration::from_millis(10));
    std::fs::write(&path, &theirs).unwrap();

    // Our stale copy tries to go back. This is the board-drag shape too: the caller
    // only meant to change a property, and would have taken the body down with it.
    o.status = Some("doing".into());
    let err = s.put(&o).unwrap_err();
    assert!(matches!(err, StoreError::Conflict(c) if c == id), "refused as a conflict: {err}");

    // The whole point: their paragraph is still on disk.
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(on_disk.contains("their hard-won paragraph"), "their work survived");
    assert!(!on_disk.contains("status: doing"), "and ours did not land");
}

/// The guard must not cost the ordinary path anything: a note nobody else touched
/// saves as many times as you like, and a brand-new note has nothing to conflict
/// with.
#[test]
fn ordinary_repeated_saves_are_unaffected_by_the_guard() {
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();

    let mut o = Object::new(Kind::Note, "first");
    s.put(&o).expect("a brand-new note has nothing on disk to lose");
    for body in ["second", "third", "fourth"] {
        o.body = body.into();
        s.put(&o).expect("our own last write is what the index holds");
    }
    assert_eq!(s.get(o.id).unwrap().unwrap().body, "fourth");
}

/// FileStore and MemoryStore honour one contract: the same queries return the
/// same objects (compared as sets, since neither promises order without a sort).
#[test]
fn filestore_matches_memorystore() {
    let dir = tempdir().unwrap();
    let mut fs_store = FileStore::open(dir.path()).unwrap();
    seed(&mut fs_store);
    // **The shared seed is notes only**, which would make every `Kind(Asset)` case below compare
    // empty against empty and pass while proving nothing. An asset is added here, locally, so the
    // kind cases actually discriminate — and so a pushdown that dropped or kept the wrong rows
    // shows up as a disagreement rather than as two identical empties.
    let mut asset = Object::new(Kind::Asset, "extracted text of a trust region paper");
    asset.title = Some("paper.pdf".into());
    fs_store.put(&asset).unwrap();

    let mut mem = MemoryStore::new();
    for o in fs_store.query(&Query::default()).unwrap().rows {
        mem.put(&o).unwrap();
    }

    // **`Kind` is pushed into SQL by `FileStore` and evaluated in memory by `MemoryStore`**, so it
    // is the one predicate where the two backends run genuinely different code — and until
    // 2026-08-29 this list, the guard that exists to prove they agree, did not mention it once.
    // The pushdown was correct; nothing here would have noticed if it had not been. Every shape
    // that could diverge is enumerated: the empty list, an intersection, a *disjoint* intersection
    // (which must answer nothing), and the two compositions under which a `Kind` must NOT be
    // pushed down because it no longer narrows the result.
    let queries = vec![
        Query { filter: Filter::new().and(Predicate::Text("trust".into())), ..Default::default() },
        Query { group_by: Some("status".into()), ..Default::default() },
        Query { sort: vec![SortKey::asc("created")], ..Default::default() },
        Query {
            filter: Filter::new().and(Predicate::Kind(vec![Kind::Note])),
            ..Default::default()
        },
        Query {
            filter: Filter::new().and(Predicate::Kind(vec![Kind::Asset])),
            ..Default::default()
        },
        Query {
            filter: Filter::new().and(Predicate::Kind(vec![Kind::Note, Kind::Asset])),
            ..Default::default()
        },
        Query { filter: Filter::new().and(Predicate::Kind(vec![])), ..Default::default() },
        // Two that intersect to one kind, and two that intersect to none.
        Query {
            filter: Filter::new()
                .and(Predicate::Kind(vec![Kind::Note, Kind::Asset]))
                .and(Predicate::Kind(vec![Kind::Note])),
            ..Default::default()
        },
        Query {
            filter: Filter::new()
                .and(Predicate::Kind(vec![Kind::Note]))
                .and(Predicate::Kind(vec![Kind::Asset])),
            ..Default::default()
        },
        // Under a negation and under a disjunction a `Kind` does not narrow the result set, so
        // pushing it would silently drop rows the engine keeps.
        Query {
            filter: Filter::new().and(Predicate::Not(Box::new(Predicate::Kind(vec![Kind::Asset])))),
            ..Default::default()
        },
        Query {
            filter: Filter::new().and(Predicate::Any(vec![
                Predicate::Kind(vec![Kind::Asset]),
                Predicate::Text("trust".into()),
            ])),
            ..Default::default()
        },
        // And the combination the planning views actually issue: a kind plus a sort.
        Query {
            filter: Filter::new().and(Predicate::Kind(vec![Kind::Note])),
            sort: vec![SortKey::asc("created")],
            ..Default::default()
        },
    ];
    for q in &queries {
        let a = fs_store.query(q).unwrap();
        let b = mem.query(q).unwrap();
        assert_eq!(a.total, b.total);
        let mut ida: Vec<_> = a.rows.iter().map(|o| o.id).collect();
        let mut idb: Vec<_> = b.rows.iter().map(|o| o.id).collect();
        ida.sort();
        idb.sort();
        assert_eq!(ida, idb, "FileStore and MemoryStore disagree");
    }
}

/// **The poll used to flap forever on a duplicated `id:`.**
///
/// `objects.id` is the primary key and `index_object` is INSERT OR REPLACE, so two files
/// carrying one id collapse to a single row whose `path` alternates. Every beat found
/// whichever path the row was *not* pointing at, re-indexed it, and reported `updated: 1` —
/// so the UI refreshed at the beat interval, indefinitely, on a vault nobody was touching.
///
/// The fix is a skip, not a tie-break: serving one file's content under another's id is
/// worse than serving neither. So this asserts the vault goes **quiet**, and says why.
#[test]
fn a_duplicated_id_settles_instead_of_flapping() {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();

    // The realistic way this happens: someone copies a note file instead of creating one.
    let note = |body: &str| {
        format!(
            "---\nschema: 1\nid: 01JQ0000000000000000000000\ntype: note\ntitle: t\n\
             created: 2026-07-18T10:00:00Z\nupdated: 2026-07-18T10:00:00Z\n---\n{body}\n"
        )
    };
    std::fs::write(notes.join("01JQ0000000000000000000000.md"), note("original")).unwrap();
    std::fs::write(notes.join("a-copy.md"), note("the copy")).unwrap();

    let mut store = FileStore::open(dir.path()).unwrap();

    // Whichever file lost, it is *named* rather than silently dropped.
    assert_eq!(store.skipped().len(), 1, "the loser is reported: {:?}", store.skipped());
    assert!(
        store.skipped()[0].reason.contains("duplicate id"),
        "and says what is wrong: {:?}",
        store.skipped()
    );

    // The point: three consecutive quiet polls must all report nothing changed. Before the
    // guard, every one of these came back `updated: 1`.
    for beat in 1..=3 {
        let stats = store.reindex(Reindex::Incremental).unwrap();
        assert_eq!(stats.updated, 0, "beat {beat} re-indexed something on a quiet vault");
        assert_eq!(stats.removed, 0, "beat {beat} reported a removal on a quiet vault");
    }
}

/// **V3, and the reason it exists.** Adopting a repo you already own means the notes are in
/// `docs/`, not in a `notes/` directory the notebook demanded you create. `vault.json` says
/// where they are, and everything downstream must simply work.
#[test]
fn a_vault_can_keep_its_notes_somewhere_other_than_notes() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("vault.json"),
        r#"{"name":"the paper","description":"notes beside the manuscript","notes":"docs"}"#,
    )
    .unwrap();
    // A note already sitting in the project's own docs directory, hand-written.
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    let o = Object::new(Kind::Note, "the introduction needs work");
    std::fs::write(
        dir.path().join("docs").join(format!("{}.md", o.id)),
        fm_core::frontmatter::to_file(&o).unwrap(),
    )
    .unwrap();

    let mut store = FileStore::open(dir.path()).unwrap();

    // It is adopted with no import step, exactly as a note in `notes/` would be.
    assert_eq!(store.query(&Query::default()).unwrap().total, 1);
    assert_eq!(store.description(), Some("notes beside the manuscript"));
    // And `FileStore::open` takes the name from the descriptor rather than the directory,
    // which is a tempdir with a random name.
    assert_eq!(store.name(), "the paper");

    // Writes land there too — a descriptor that only affected reading would relocate a note
    // on its first edit, which is the trapdoor the design rejected.
    let fresh = Object::new(Kind::Note, "written through the app");
    store.put(&fresh).unwrap();
    assert!(
        dir.path().join("docs").join(format!("{}.md", fresh.id)).exists(),
        "a new note must land beside the others, not in a notes/ dir nobody asked for"
    );
    assert!(!dir.path().join("notes").exists(), "and no stray notes/ is created");
}

/// The vault list's name is the audience *this user* chose. A repo they cloned does not get
/// to rename it out from under them.
#[test]
fn an_explicit_name_beats_the_descriptors() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("vault.json"), r#"{"name":"their name"}"#).unwrap();

    let store = FileStore::named(dir.path(), "my name").unwrap();

    assert_eq!(store.name(), "my name");
}

/// A conflicted note appears **mid-session**, when a pull lands — not at startup. So the
/// skipped list has to stay current across incremental reindexes, or the app can only ever
/// report the notes that were already broken when it opened.
///
/// This is what lets the UI say "3 notes could not be read" instead of those notes simply
/// being absent from every view with no explanation, which is what stderr-at-startup meant
/// in a browser-first product.
#[test]
fn a_note_that_breaks_mid_session_shows_up_in_skipped() {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let good = Object::new(Kind::Note, "fine");
    std::fs::write(
        notes.join(format!("{}.md", good.id)),
        fm_core::frontmatter::to_file(&good).unwrap(),
    )
    .unwrap();

    let mut store = FileStore::open(dir.path()).unwrap();
    assert!(store.skipped().is_empty(), "a healthy vault reports nothing");

    // A merge lands markers inside the YAML fence — the usual cause.
    std::fs::write(
        notes.join("01JQ0000000000000000000000.md"),
        "---\nschema: 1\nid: 01JQ0000000000000000000000\n<<<<<<< ours\ntitle: mine\n=======\ntitle: theirs\n>>>>>>> theirs\n---\nbody\n",
    )
    .unwrap();
    store.reindex(Reindex::Incremental).unwrap();

    assert_eq!(store.skipped().len(), 1, "named after the poll, not only at open");
    assert!(
        store.skipped()[0].name == "01JQ0000000000000000000000.md",
        "and says which file: {:?}",
        store.skipped()
    );

    // It must also be re-reported on every later beat — an unreadable note has no index row,
    // so it is re-read each pass. If it were only reported once, a tab opened afterwards
    // would never learn about it.
    store.reindex(Reindex::Incremental).unwrap();
    assert_eq!(store.skipped().len(), 1, "still reported on the next beat");

    // …and clears the moment a human fixes it.
    std::fs::remove_file(notes.join("01JQ0000000000000000000000.md")).unwrap();
    store.reindex(Reindex::Incremental).unwrap();
    assert!(store.skipped().is_empty(), "resolved means silent again");
}
