//! Layer-1 end-to-end: real `.md` text files on disk, through the real
//! `FileStore` and the real command functions, to the exact DTO/JSON the window
//! receives. This is "run the app's backend with some text files and check every
//! link up to the IPC boundary" — no webview, but the whole
//! parse -> index -> query -> DTO -> JSON chain over hand-authored fixture notes
//! of different kinds (a rich note, a due task, an asset, a custom-prop note, and
//! a unicode/`---`-in-body edge case). The fixtures live in
//! `tests/fixtures/vault/notes/*.md`, named by their ULID so an in-place edit
//! (`update_body`) overwrites the same file, exactly like a real vault.

use std::path::PathBuf;

use fm_app::commands::{agenda, board, gallery, get, update_body};
use fm_core::FileStore;
use tempfile::{tempdir, TempDir};

// Fixture ids — must match the `id:` frontmatter (and filename) of each note.
const RICH: &str = "01KXGCF14BWT0XPKN8TH1YWRC0"; // note,  status doing, math+mermaid+asset+table
const TASK: &str = "01KXGCF14BWT0XPKN8TH1YWTK0"; // task,  status todo, due 2026-07-20
const ASSET: &str = "01KXGCF14BWT0XPKN8TH1YWAS0"; // asset (gallery)
const CUSTOM: &str = "01KXGCF14BWT0XPKN8TH1YWCS0"; // note, status todo, due 2026-07-16, project: alpha
const UNICODE: &str = "01KXGCF14BWT0XPKN8TH1YWVN0"; // note, status done, unicode + `---` in body

/// Copy the committed fixture vault into a fresh tempdir (so reindex/edit writes
/// never touch the checked-in files), then open a real `FileStore` over it — the
/// same "reindex on open" the CLI and the desktop app do.
fn open_fixture_vault() -> (TempDir, FileStore) {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault/notes");
    for entry in std::fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|e| e == "md") {
            std::fs::copy(entry.path(), notes.join(entry.file_name())).unwrap();
        }
    }
    let store = FileStore::open(dir.path()).expect("open fixture vault");
    (dir, store)
}

#[test]
fn every_text_file_in_the_vault_loads_and_is_queryable() {
    let (_dir, store) = open_fixture_vault();
    for id in [RICH, TASK, ASSET, CUSTOM, UNICODE] {
        assert!(get(&store, id).unwrap().is_some(), "note {id} did not load");
    }
}

#[test]
fn board_groups_the_real_notes_by_status_and_by_a_custom_property() {
    let (_dir, store) = open_fixture_vault();

    let b = board(&store, "status").unwrap();
    let col = |label: &str| b.columns.iter().find(|c| c.label == label);
    assert_eq!(col("doing").unwrap().cards.len(), 1, "doing = rich");
    assert_eq!(col("todo").unwrap().cards.len(), 2, "todo = task + custom");
    assert_eq!(col("done").unwrap().cards.len(), 1, "done = unicode");
    // The asset gets no card at all — not even a "(none)" one for its missing
    // status. It is a blob the notes reference; `gallery`/`search` still see it.
    assert!(
        b.columns.iter().flat_map(|c| c.cards.iter()).all(|m| m.id != ASSET),
        "the asset leaked onto the board",
    );

    // Grouping by a user-invented key needs no backend change; props flow through.
    let pb = board(&store, "project").unwrap();
    let alpha = pb.columns.iter().find(|c| c.label == "alpha").unwrap();
    assert_eq!(alpha.cards.len(), 1);
    assert_eq!(alpha.cards[0].id, CUSTOM);
    assert_eq!(alpha.cards[0].props.get("project").unwrap(), &serde_json::json!("alpha"));
}

#[test]
fn gallery_returns_only_the_asset() {
    let (_dir, store) = open_fixture_vault();
    let assets = gallery(&store).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, ASSET);
    assert_eq!(assets[0].kind, "asset");
}

#[test]
fn agenda_shows_due_unfinished_notes_soonest_first() {
    let (_dir, store) = open_fixture_vault();
    let ids: Vec<String> = agenda(&store).unwrap().iter().map(|m| m.id.clone()).collect();
    // custom (due 07-16) before task (due 07-20); unicode is done, rich/asset undated.
    assert_eq!(ids, vec![CUSTOM.to_string(), TASK.to_string()]);
}

#[test]
fn get_returns_the_render_payload_verbatim() {
    let (_dir, store) = open_fixture_vault();
    let note = get(&store, RICH).unwrap().expect("rich note exists");
    assert_eq!(note.meta.kind, "note");
    assert_eq!(note.meta.status.as_deref(), Some("doing"));
    assert_eq!(note.meta.tags, vec!["meta-rl".to_string()]);
    // The body is the exact markdown render.ts will turn into HTML — every content
    // type the window must handle is present and untouched.
    for needle in ["# GAE", "$\\lambda", "$$", "```mermaid", "asset:sha256-deadbeef", "| step |"] {
        assert!(note.body.contains(needle), "body is missing {needle:?}");
    }
}

#[test]
fn note_detail_serializes_to_the_shape_the_frontend_expects() {
    let (_dir, store) = open_fixture_vault();
    let note = get(&store, CUSTOM).unwrap().unwrap();
    let json = serde_json::to_value(&note).unwrap();
    // Matches ui/src/lib/types.ts `NoteDetail`: flat meta + body, `type` (not
    // `kind`), custom keys under `props`, `due` as an ISO string. This is the IPC
    // contract the Layer-2 mock backend mirrors.
    for key in [
        "id", "type", "title", "preview", "status", "due", "hard", "created", "updated", "tags",
        "props", "body",
    ] {
        assert!(json.get(key).is_some(), "serialized note is missing key `{key}`");
    }
    assert!(json.get("kind").is_none(), "kind must serialize as `type`");
    assert_eq!(json["type"], "note");
    assert_eq!(json["due"], "2026-07-16");
    assert_eq!(json["props"]["project"], "alpha");
}

#[test]
fn editing_a_note_body_round_trips_byte_for_byte_to_disk() {
    let (dir, mut store) = open_fixture_vault();
    let tricky = "Edited.\n\n---\n\n$\\lambda = 0.95$ café ☕\n";
    update_body(&mut store, RICH, tricky).unwrap();
    // Reopen (index rebuilt from the .md file) — the edit hit the real file.
    let reopened = FileStore::open(dir.path()).unwrap();
    let note = get(&reopened, RICH).unwrap().unwrap();
    assert_eq!(note.body, tricky, "the edited body round-trips exactly");
}

#[test]
fn the_unicode_note_round_trips_including_a_dashes_line_inside_the_body() {
    let (_dir, store) = open_fixture_vault();
    let note = get(&store, UNICODE).unwrap().unwrap();
    assert!(note.body.contains("café ☕ — naïve façade"), "unicode survived");
    assert!(note.body.contains("\n---\n"), "the body's own `---` was kept, not eaten as a fence");
}
