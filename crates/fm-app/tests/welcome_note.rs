//! **The note the archive ships must be a note the app can actually read.**
//!
//! This is the one file where a silent failure is guaranteed to be seen by exactly the person least
//! able to diagnose it. The loader is deliberately tolerant — an unparseable note does not error, it
//! **vanishes** — so a typo in the shipped welcome note does not produce a warning, it produces a
//! brand-new user opening an empty notebook and concluding the app is broken. The same tolerance
//! once made a whole vault open empty on Windows over line endings.
//!
//! So this parses the real files — the tutorials every new notebook starts with, on every device
//! (`fm_app::tutorials`, 2026-10-18) — and checks the things that would make them disappear or
//! misbehave rather than merely look wrong.

use fm_model::Kind;
use std::path::PathBuf;

/// The shipped tutorials, as (path-like name, text).
fn welcome_files() -> Vec<(PathBuf, String)> {
    fm_app::tutorials::TUTORIALS
        .iter()
        .map(|(name, text)| (PathBuf::from(name), text.to_string()))
        .collect()
}

#[test]
fn the_shipped_welcome_note_is_a_note_the_app_can_read() {
    let files = welcome_files();
    assert!(!files.is_empty(), "the archive ships a welcome note; none was found");

    for (path, text) in files {
        let obj = fm_core::frontmatter::from_file(&text)
            .unwrap_or_else(|e| panic!("{} does not parse as a note: {e}", path.display()));

        // **The filename IS the id** (`path_for(id) = notes/<id>.md`). If they disagree the app
        // indexes one identity and writes another, and the note duplicates on first edit.
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        assert_eq!(obj.id.to_string(), stem, "{}: id must equal the filename", path.display());

        // A `Kind` other than `Note` would keep it out of every planning view.
        assert_eq!(obj.kind, Kind::Note, "{}: must be an ordinary note", path.display());

        // The three properties that would make it *invisible*: a `thread_of` makes it a discussion
        // message, a `proposes` makes it a proposal, and either is excluded from every view a new
        // user will look at. Nothing here should be a hidden note-class.
        assert!(
            !fm_app::thread::is_message(&obj),
            "{}: must not read as a message",
            path.display()
        );
        assert!(
            !fm_app::thread::is_proposal(&obj),
            "{}: must not read as a proposal",
            path.display()
        );

        assert!(
            obj.title.is_some(),
            "{}: needs a title — it is the first thing shown",
            path.display()
        );
        assert!(obj.body.len() > 150, "{}: too short to orient anyone", path.display());
        // On the shelf the Library opens on.
        assert_eq!(
            obj.extra.get(fm_app::views::SHELF).map(|v| v.display()).as_deref(),
            Some("Tutorials"),
            "{}: must be on the Tutorials shelf",
            path.display()
        );

        // Round-trips byte-for-byte, so the first edit does not rewrite the whole file and make the
        // user's first change look like a hundred-line diff.
        let round = fm_core::frontmatter::to_file(&obj).unwrap();
        assert_eq!(
            round,
            text,
            "{}: must be written exactly as the app would write it",
            path.display()
        );
    }
}

#[test]
fn the_welcome_note_describes_the_app_that_actually_ships() {
    // A welcome note that names a button which is not there is worse than none: the reader concludes
    // they are looking at the wrong app. These are the affordances it points at; if one is renamed,
    // this fails and the note gets fixed with it.
    let text: String = welcome_files().into_iter().map(|(_, t)| t).collect::<Vec<_>>().join("\n");
    for view in ["Timeline", "Board", "Agenda", "Search"] {
        assert!(
            text.contains(view),
            "the note names the views a user can switch to; {view} missing"
        );
    }
    for control in ["Back up", "Settings", "Help"] {
        assert!(text.contains(control), "{control} is a control the note tells the reader to use");
    }

    // **This list used to include `Edit`, and that is how the guard kept the note wrong.**
    //
    // The check above is a *presence* test, so it can only catch a control that gets renamed while
    // the note still names it. It cannot catch the opposite — a control deleted from the app while
    // the note goes on teaching it — because the note still contains the word, so the assertion
    // still passes. `Edit` was removed from the UI on 2026-08-31 (`NotePanel.svelte`: "there is no
    // Edit button any more"), and this test went on requiring the note to mention it for another
    // nine days, right up to the docs audit that found it.
    //
    // So the guard now has both halves. These are the phrasings that named controls which no
    // longer exist — a bolded **Edit**, a clickable view name, a scroll-wheel ring of views, and a
    // top-right button strip that became a side panel on 2026-08-30. A direction is the part that
    // rots fastest, which is why the note names the strip instead of a corner.
    //
    // `double-click` joined the list on 2026-10-11, when a single click on a note's text became
    // the way to edit it. This test had required the phrase until that day, which is the same
    // trap the first paragraph describes, one gesture later.
    for gone in [
        "**Edit**",
        "top right",
        "top bar",
        "in the ring",
        "scroll the wheel",
        "double-click",
        "Double-click",
    ] {
        assert!(
            !text.contains(gone),
            "the welcome note still teaches {gone:?}, which the app no longer has"
        );
    }
    // The affordance that replaced it. Named positively so the note cannot simply drop the subject.
    assert!(
        text.contains("click or tap") || text.contains("Click a note's text"),
        "the note must say how a note is actually edited"
    );
    // The assistant ships in every download now (`agent-serve` is staged beside `fm-serve`), so a
    // tutorial may name it — by the one name it has everywhere, never by a model.
    assert!(text.contains("@Myrme"), "the assistant is named as it is called");
    assert!(!text.contains("qwen") && !text.contains("lfm"), "never by a model's name");
}

/// **Stage a vault exactly as the release does, then open it with the real app.**
///
/// Parsing the file is not the same claim as the app finding it. The release copies these files
/// into a `notes/` folder and ships it; whether that folder then opens as a working vault — index
/// built, note listed, searchable — is a separate question, and it is the one the first-time user
/// actually asks. So this reproduces the copy and drives the same door `fm-serve` uses.
#[test]
fn a_vault_staged_like_the_release_opens_with_the_note_in_it() {
    use fm_app::{dispatch, vaults::VaultConfig, App, Host, Output};
    use fm_core::MultiStore;
    use serde_json::{json, Value};

    struct NoHost;
    impl Host for NoHost {
        fn open_external(&self, _p: &std::path::Path) -> Result<(), String> {
            Err("not in a test".into())
        }
    }

    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();

    // **Written exactly as every new notebook gets them** — the one function the phone's first
    // notebook and *new notebook* call, and byte-for-byte what the download stages: no date of the
    // build in them, so two devices' copies are the same files and merge as one.
    assert_eq!(fm_app::tutorials::write_into(dir.path()).unwrap(), welcome_files().len());
    assert_eq!(
        fm_app::tutorials::write_into(dir.path()).unwrap(),
        0,
        "never into a notebook with notes"
    );
    let _ = &notes;

    let owned = vec![("personal".to_string(), dir.path().to_path_buf())];
    let app = App::new(
        MultiStore::open(&owned).expect("a staged vault must open"),
        vec![VaultConfig { name: "personal".into(), path: dir.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );

    let call = |cmd: &str, args: Value| -> Value {
        let out: Output =
            dispatch(cmd, &args, &[], &app, &NoHost).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        match out {
            Output::Json(b) if b.is_empty() => Value::Null,
            o => serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null),
        }
    };

    // The first screen. An empty Timeline here is the failure this whole file exists to prevent.
    let recent = call("recent", json!({ "limit": 20 }));
    let rows = recent.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(rows > 0, "the Timeline is empty in a freshly staged vault: {recent}");

    // And it must be *findable*, not merely present — the note tells the reader to use Search.
    let hits = call("search", json!({ "query": "notebook" }));
    let found = hits.as_array().is_some_and(|a| !a.is_empty());
    assert!(found, "Search finds nothing for a word in the shipped note: {hits}");
}

/// **A way back** (2026-10-18): *Put the tutorials back* writes the ones that are missing, as they
/// shipped, and leaves one that is here — changed by its reader — exactly as it is.
#[test]
fn putting_the_tutorials_back_restores_only_what_is_missing() {
    use fm_core::{FileStore, Store};
    let dir = tempfile::tempdir().unwrap();
    fm_app::tutorials::write_into(dir.path()).unwrap();
    let mut store = FileStore::named(dir.path(), "personal").unwrap();
    let ids: Vec<fm_model::Id> = fm_app::tutorials::TUTORIALS
        .iter()
        .map(|(name, _)| name.trim_end_matches(".md").parse().unwrap())
        .collect();
    // One deleted, one changed by its reader.
    store.delete(ids[1]).unwrap();
    let mut changed = store.get(ids[0]).unwrap().unwrap();
    changed.body = "My own words now.".into();
    store.put(&changed).unwrap();

    let back = fm_app::tutorials::put_back(&mut store, "personal").unwrap();
    assert_eq!(back.len(), 1, "only the deleted one");
    assert!(store.get(ids[1]).unwrap().is_some());
    assert_eq!(store.get(ids[0]).unwrap().unwrap().body, "My own words now.", "left as it is");
    assert!(fm_app::tutorials::put_back(&mut store, "personal").unwrap().is_empty(), "idempotent");
}
