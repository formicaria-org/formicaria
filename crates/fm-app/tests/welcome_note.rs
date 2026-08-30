//! **The note the archive ships must be a note the app can actually read.**
//!
//! This is the one file where a silent failure is guaranteed to be seen by exactly the person least
//! able to diagnose it. The loader is deliberately tolerant — an unparseable note does not error, it
//! **vanishes** — so a typo in the shipped welcome note does not produce a warning, it produces a
//! brand-new user opening an empty notebook and concluding the app is broken. The same tolerance
//! once made a whole vault open empty on Windows over line endings.
//!
//! So this parses the real file from `packaging/`, not a copy, and checks the things that would make
//! it disappear or misbehave rather than merely look wrong.

use fm_model::Kind;
use std::path::PathBuf;

fn welcome_files() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packaging/welcome/notes")
        .canonicalize()
        .expect("packaging/welcome/notes must exist — the archive stages it into the vault");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    v.sort();
    v
}

#[test]
fn the_shipped_welcome_note_is_a_note_the_app_can_read() {
    let files = welcome_files();
    assert!(!files.is_empty(), "the archive ships a welcome note; none was found");

    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
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
        assert!(!fm_app::thread::is_message(&obj), "{}: must not read as a message", path.display());
        assert!(!fm_app::thread::is_proposal(&obj), "{}: must not read as a proposal", path.display());

        assert!(obj.title.is_some(), "{}: needs a title — it is the first thing shown", path.display());
        assert!(obj.body.len() > 200, "{}: too short to orient anyone", path.display());

        // Round-trips byte-for-byte, so the first edit does not rewrite the whole file and make the
        // user's first change look like a hundred-line diff.
        let round = fm_core::frontmatter::to_file(&obj).unwrap();
        assert_eq!(round, text, "{}: must be written exactly as the app would write it", path.display());
    }
}

#[test]
fn the_welcome_note_describes_the_app_that_actually_ships() {
    // A welcome note that names a button which is not there is worse than none: the reader concludes
    // they are looking at the wrong app. These are the affordances it points at; if one is renamed,
    // this fails and the note gets fixed with it.
    let text = std::fs::read_to_string(&welcome_files()[0]).unwrap();
    for view in ["Timeline", "Board", "Agenda", "Search"] {
        assert!(text.contains(view), "the note names the views a user can switch to; {view} missing");
    }
    for control in ["Back up", "Settings", "Help", "Edit"] {
        assert!(text.contains(control), "{control} is a control the note tells the reader to use");
    }
    // The archive carries no assistant, so the note must not advertise one.
    for absent in ["assistant", "@name", "/research", "/transcribe"] {
        assert!(
            !text.contains(absent),
            "the release archive ships no assistant — the welcome note must not promise {absent:?}"
        );
    }
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

    // The staging step, and **including the part that rewrites the file**: the release dates the
    // note with the build, so what ships is never byte-identical to what is in the repo. Testing
    // the pristine source would leave the one transformation the user actually receives unchecked —
    // and a date this rewrite malformed would make the note vanish exactly as a typo would.
    for f in welcome_files() {
        let staged: String = std::fs::read_to_string(&f)
            .unwrap()
            .lines()
            .map(|l| match l.split_once(": ") {
                Some(("created", _)) => "created: 2027-01-02T03:04:05Z".to_string(),
                Some(("updated", _)) => "updated: 2027-01-02T03:04:05Z".to_string(),
                _ => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(notes.join(f.file_name().unwrap()), staged).unwrap();
    }

    let owned = vec![("personal".to_string(), dir.path().to_path_buf())];
    let app = App::new(
        MultiStore::open(&owned).expect("a staged vault must open"),
        vec![VaultConfig {
            name: "personal".into(),
            path: dir.path().to_path_buf(),
            restic: None,
        }],
        Some(home.path().join("vaults.json")),
        true,
    );

    let call = |cmd: &str, args: Value| -> Value {
        let out: Output = dispatch(cmd, &args, &[], &app, &NoHost).unwrap_or_else(|e| panic!("{cmd}: {e}"));
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
