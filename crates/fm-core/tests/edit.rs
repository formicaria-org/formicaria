//! S2: properties are editable — a changed property is written back to the
//! Markdown file and survives reload; and (the load-bearing part) editing a note
//! never drops custom frontmatter a user added by hand. That last guarantee is
//! also the precondition for "board by ANY property" in S3.

use fm_core::{frontmatter, FileStore, Store};
use fm_model::{Kind, Object, PropertyValue, Stamp};
use fm_query::Query;
use std::fs;
use tempfile::tempdir;
use time::macros::{date, datetime, time};

#[test]
fn set_property_is_written_to_disk_and_survives_reload() {
    let dir = tempdir().unwrap();
    let id = {
        let mut s = FileStore::open(dir.path()).unwrap();
        let mut o = Object::new(Kind::Note, "ship S2");
        let id = o.id;
        // **Both stamps are pinned, and that is the point.** `Object::new` sets `created` to
        // *now*, so leaving it there and hard-coding `updated` to a fixed instant made the
        // final assertion a time bomb: it held only while the wall clock was behind that
        // instant, and started failing the morning the clock passed it. A test that depends
        // on today's date is a test that will fail on a day nobody changed anything.
        o.created = datetime!(2026-07-19 9:00 UTC);
        s.put(&o).unwrap();

        // Edit: set status + due, stamp updated (what `fm set` does).
        o.status = Some("doing".into());
        o.due = Some(Stamp::day(date!(2026 - 08 - 01)));
        o.updated = datetime!(2026-07-20 9:00 UTC);
        s.put(&o).unwrap();
        id
    };

    // The raw file reflects the edit...
    let raw = fs::read_to_string(dir.path().join(format!("notes/{id}.md"))).unwrap();
    assert!(raw.contains("status: doing"), "frontmatter carries the new status:\n{raw}");
    assert!(raw.contains("due: 2026-08-01"), "and the due date:\n{raw}");

    // ...and a fresh store (reindex from files) reads it back.
    let s2 = FileStore::open(dir.path()).unwrap();
    let got = s2.get(id).unwrap().unwrap();
    assert_eq!(got.status.as_deref(), Some("doing"));
    assert_eq!(got.due, Some(Stamp::day(date!(2026 - 08 - 01))));
    assert!(got.updated > got.created, "editing bumped updated past created");
}

#[test]
fn setting_a_due_time_writes_it_to_disk_and_reads_it_back() {
    // The `set_property` path the UI's time input and `fm set` both take — the
    // time has to reach the file as text and come back typed.
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();
    let mut o = Object::new(Kind::Note, "supervision");
    let id = o.id;
    s.put(&o).unwrap();

    fm_core::edit::apply_property(&mut o, "start", "2026-08-01T14:30").unwrap();
    fm_core::edit::apply_property(&mut o, "due", "2026-08-01T15:00").unwrap();
    s.put(&o).unwrap();

    let raw = fs::read_to_string(dir.path().join(format!("notes/{id}.md"))).unwrap();
    assert!(raw.contains("start: 2026-08-01T14:30"), "start carries its time:\n{raw}");
    assert!(raw.contains("due: 2026-08-01T15:00"), "due carries its time:\n{raw}");

    let got = FileStore::open(dir.path()).unwrap().get(id).unwrap().unwrap();
    assert_eq!(got.start, Some(Stamp::at(date!(2026 - 08 - 01), time!(14:30))));
    assert_eq!(got.due, Some(Stamp::at(date!(2026 - 08 - 01), time!(15:00))));
}

#[test]
fn clearing_and_rejecting_a_due_value_both_still_work() {
    let mut o = Object::new(Kind::Note, "body");
    fm_core::edit::apply_property(&mut o, "due", "2026-08-01T15:00").unwrap();

    // An empty value clears (the "drag to (none)" gesture).
    fm_core::edit::apply_property(&mut o, "due", "").unwrap();
    assert_eq!(o.due, None);

    // Garbage is refused rather than silently dropped or defaulted to today.
    assert!(fm_core::edit::apply_property(&mut o, "due", "next tuesday").is_err());
    assert_eq!(o.due, None, "a rejected write must not mutate the field");
}

#[test]
fn editing_a_note_preserves_hand_added_custom_properties() {
    // The data-integrity guarantee: a property fm doesn't know about (added in
    // Vim) must not vanish when fm rewrites the file after an unrelated edit.
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();

    // Hand-author a note carrying a custom `project` property.
    let mut hand = Object::new(Kind::Note, "external note");
    hand.extra.insert("project".into(), PropertyValue::Text("alpha".into()));
    let id = hand.id;
    fs::write(notes.join(format!("{id}.md")), frontmatter::to_file(&hand).unwrap()).unwrap();

    // fm loads it, edits an unrelated property, writes back.
    let mut s = FileStore::open(dir.path()).unwrap();
    let mut got = s.get(id).unwrap().unwrap();
    assert_eq!(got.get("project"), PropertyValue::Text("alpha".into()));
    got.status = Some("doing".into());
    s.put(&got).unwrap();

    // The custom property is still there after the edit.
    let reread = FileStore::open(dir.path()).unwrap().get(id).unwrap().unwrap();
    assert_eq!(reread.get("project"), PropertyValue::Text("alpha".into()));
    assert_eq!(reread.status.as_deref(), Some("doing"));
}

#[test]
fn custom_property_is_groupable_like_any_other() {
    // Proves extra properties flow through get() into the generic query engine —
    // the precondition for grouping a board by a user-invented property in S3.
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();
    for (body, proj) in [("a", "alpha"), ("b", "alpha"), ("c", "beta")] {
        let mut o = Object::new(Kind::Note, body);
        o.extra.insert("project".into(), PropertyValue::Text(proj.into()));
        s.put(&o).unwrap();
    }

    let q = Query { group_by: Some("project".into()), ..Default::default() };
    let groups = s.query(&q).unwrap().groups.unwrap();

    let mut labels: Vec<_> = groups.iter().map(|g| g.label.clone()).collect();
    labels.sort();
    assert_eq!(labels, vec!["alpha", "beta"]);
    let alpha = groups.iter().find(|g| g.label == "alpha").unwrap();
    assert_eq!(alpha.rows.len(), 2);
}

/// **`assets` and `code` had no write path at all, and losing one was silent.** `apply_property`
/// matched neither key, so both fell through to the `extra` catch-all and were written as a
/// *scalar* (`assets: sha256:…`) — while `from_file` reads them with `as_string_seq`, which
/// answers `Vec::new()` for anything that is not a YAML sequence. The value was therefore gone on
/// the very next load, with no error at any layer. Found 2026-08-29.
///
/// `assets` is the field that links a note to its blob, so this is the one property whose silent
/// loss detaches a note from its attachment. `code` shares the shape and the bug.
#[test]
fn setting_assets_or_code_survives_a_reload() {
    let dir = tempdir().unwrap();
    let mut s = FileStore::open(dir.path()).unwrap();
    let mut o = Object::new(Kind::Note, "a note that points at a blob");
    let id = o.id;
    s.put(&o).unwrap();

    fm_core::edit::apply_property(&mut o, "assets", "sha256:aa11").unwrap();
    fm_core::edit::apply_property(&mut o, "code", "src/main.rs").unwrap();
    s.put(&o).unwrap();

    // It must land in the typed field, not in `extra` — otherwise it serialises as a scalar.
    assert_eq!(o.assets, vec!["sha256:aa11".to_string()], "assets is a typed field, not `extra`");
    assert_eq!(o.code, vec!["src/main.rs".to_string()], "code is a typed field, not `extra`");

    // The file carries a YAML *sequence*, which is the only shape `as_string_seq` reads back.
    let raw = fs::read_to_string(dir.path().join(format!("notes/{id}.md"))).unwrap();
    assert!(raw.contains("- sha256:aa11"), "assets must serialise as a sequence:\n{raw}");

    // And the round trip holds.
    let got = FileStore::open(dir.path()).unwrap().get(id).unwrap().unwrap();
    assert_eq!(got.assets, vec!["sha256:aa11".to_string()], "assets survived the reload");
    assert_eq!(got.code, vec!["src/main.rs".to_string()], "code survived the reload");
}

/// Several references at once, and the separator is a comma — **never a space**, unlike `tags`.
/// A blob reference cannot contain a space, and copying the `tags` arm's `split([',', ' '])` is
/// exactly the bug that makes a multi-word tag unrepresentable; it must not spread to a new key.
#[test]
fn assets_takes_several_references_separated_by_commas() {
    let mut o = Object::new(Kind::Note, "two attachments");
    fm_core::edit::apply_property(&mut o, "assets", "sha256:aa11, sha256:bb22").unwrap();
    assert_eq!(o.assets, vec!["sha256:aa11".to_string(), "sha256:bb22".to_string()]);

    // Empty clears it, the way every other optional property clears.
    fm_core::edit::apply_property(&mut o, "assets", "").unwrap();
    assert!(o.assets.is_empty(), "an empty value clears the list");
}

/// A note written by hand — or by the app before 2026-08-29, when `apply_property` had no
/// `assets` arm and wrote a scalar — must still be read. The strict sequence-only read is what
/// made that bug silent; this is the recovery half of it.
#[test]
fn a_hand_written_scalar_asset_is_read_as_a_one_element_list() {
    let dir = tempdir().unwrap();
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    // Author a valid note, then degrade exactly the one line — a scalar where a sequence
    // would normally be, which is precisely what the missing `apply_property` arm produced.
    let mut hand = Object::new(Kind::Note, "x");
    hand.assets = vec!["sha256:aa11".into()];
    let id = hand.id;
    let good = frontmatter::to_file(&hand).unwrap();
    assert!(good.contains("- sha256:aa11"), "precondition: normally a sequence:\n{good}");
    let degraded = good.replace("assets:\n- sha256:aa11", "assets: sha256:aa11");
    assert!(degraded.contains("assets: sha256:aa11"), "degraded form:\n{degraded}");
    fs::write(notes.join(format!("{id}.md")), degraded).unwrap();

    let got = FileStore::open(dir.path()).unwrap().get(id).unwrap().unwrap();
    assert_eq!(got.assets, vec!["sha256:aa11".to_string()], "a scalar reads as one reference");

    // And rewriting it normalises the file to a sequence, without losing the reference.
    let mut s = FileStore::open(dir.path()).unwrap();
    s.put(&got).unwrap();
    let raw = fs::read_to_string(notes.join(format!("{id}.md"))).unwrap();
    assert!(raw.contains("- sha256:aa11"), "rewritten as a sequence:\n{raw}");
}

/// A custom property typed into the app must end up as the same `PropertyValue` a text editor
/// would have produced, because `PropertyValue`'s derived `Ord` compares the **variant before the
/// value** — so a corpus holding `year` as both `Int` and `Text` sorts into two disjoint blocks
/// depending on nothing but which surface wrote it.
#[test]
fn a_typed_in_property_gets_the_type_the_file_would_have_given_it() {
    let cases = [
        ("2017", PropertyValue::Int(2017)),
        ("-3", PropertyValue::Int(-3)),
        ("true", PropertyValue::Bool(true)),
        ("false", PropertyValue::Bool(false)),
        ("NeurIPS", PropertyValue::Text("NeurIPS".into())),
        ("10.48550/arXiv.1706.03762", PropertyValue::Text("10.48550/arXiv.1706.03762".into())),
    ];
    for (raw, want) in cases {
        let mut o = Object::new(Kind::Note, "x");
        fm_core::edit::apply_property(&mut o, "year", raw).unwrap();
        assert_eq!(o.get("year"), want, "typing {raw:?} should store {want:?}");
    }

    // And it agrees with the file: the same text written into frontmatter by hand parses the same.
    let mut o = Object::new(Kind::Note, "x");
    fm_core::edit::apply_property(&mut o, "year", "2017").unwrap();
    let reparsed = frontmatter::from_file(&frontmatter::to_file(&o).unwrap()).unwrap();
    assert_eq!(reparsed.get("year"), PropertyValue::Int(2017), "app and file agree");
}

/// **The inference is lossless-only.** A value YAML would *normalise* keeps its text, because for
/// these the exact string is the datum: an identifier with leading zeros, a version, a phone
/// number. Getting this wrong silently rewrites user data, which is worse than the bug it fixes.
#[test]
fn a_value_yaml_would_rewrite_keeps_its_text() {
    for raw in ["007123", "1.50", "+7", "0x1f", "1e3", "2026-08-01"] {
        let mut o = Object::new(Kind::Note, "x");
        fm_core::edit::apply_property(&mut o, "code_no", raw).unwrap();
        assert_eq!(
            o.get("code_no"),
            PropertyValue::Text(raw.into()),
            "{raw:?} must survive verbatim — inferring a type here would rewrite it"
        );
    }
}

/// **A multi-word tag was unrepresentable through the app.** `tags` split on `[',', ' ']`
/// unconditionally, so `Machine Learning` became two unrelated tags — silently, and fatally for
/// anything mapping an external hierarchy (a Zotero collection, a folder, an imported keyword)
/// onto tags. A comma is now the separator when there is one; whitespace still is when there
/// isn't, so nobody's `todo urgent` habit breaks.
#[test]
fn a_comma_lets_a_tag_contain_a_space() {
    let mut o = Object::new(Kind::Note, "x");
    fm_core::edit::apply_property(&mut o, "tags", "Machine Learning, To Read").unwrap();
    assert_eq!(o.tags, vec!["Machine Learning".to_string(), "To Read".to_string()]);

    // One separator, no heuristic: `todo urgent` is now a single tag. A "comma if present, else
    // whitespace" rule was tried and rejected — it left a *single* multi-word tag needing a
    // trailing comma, which is a rule nobody would guess.
    fm_core::edit::apply_property(&mut o, "tags", "todo urgent").unwrap();
    assert_eq!(o.tags, vec!["todo urgent".to_string()]);

    // And a multi-word tag survives the file, which is the point of writing it.
    fm_core::edit::apply_property(&mut o, "tags", "Machine Learning").unwrap();
    let back = frontmatter::from_file(&frontmatter::to_file(&o).unwrap()).unwrap();
    assert_eq!(back.tags, vec!["Machine Learning".to_string()]);
}
