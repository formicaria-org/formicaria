//! **A filtered view can be made from the app.**
//!
//! `save_view` wrote a renderer and a group-by and nothing else, and refused outright to touch a
//! view that carried a filter — so the only way to have one was to author YAML in a text editor,
//! for a headline feature, in an app whose owner works only through the UI
//! (`outstanding.md` §2.6b).
//!
//! The grammar is still not exposed: nine kinds of predicate behind a UI is a query builder, and
//! that ruling stands. One `tag` is not that — "the ones tagged `paper`" is a sentence a person
//! says — and a filter richer than a single tag is still refused rather than flattened.

use fm_app::views::{list_views, save_view, Renderer};
use std::fs;
use tempfile::tempdir;

fn views_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("views")
}

#[test]
fn a_saved_view_can_carry_one_tag() {
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Papers", Renderer::Board, Some("status"), Some("paper")).unwrap();

    let raw = fs::read_to_string(views_dir(dir.path()).join("papers.view")).unwrap();
    // The same YAML a person writes by hand — this is a file they own, in their vault, tracked by
    // git and read by collaborators, not an opaque app artifact.
    assert!(raw.contains("name: Papers"), "{raw}");
    assert!(raw.contains("view: board"), "{raw}");
    assert!(raw.contains("group_by: status"), "{raw}");
    assert!(raw.contains("filter:\n  - tag: paper"), "the tag becomes a real predicate:\n{raw}");

    // And it parses back as a working view, not just a file that happens to exist.
    let listed = list_views(dir.path());
    let v = listed.iter().find(|v| v.name == "Papers").expect("listed");
    assert!(v.error.is_none(), "a view the app just wrote must parse: {:?}", v.error);
}

#[test]
fn no_tag_means_no_filter_at_all() {
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Everything", Renderer::Timeline, None, None).unwrap();
    let raw = fs::read_to_string(views_dir(dir.path()).join("everything.view")).unwrap();
    assert!(!raw.contains("filter"), "an unfiltered view stays unfiltered:\n{raw}");

    // An empty string is the same as absent — the dialog sends "" when the field is untouched.
    save_view(dir.path(), "Everything", Renderer::Timeline, None, Some("  ")).unwrap();
    let raw = fs::read_to_string(views_dir(dir.path()).join("everything.view")).unwrap();
    assert!(!raw.contains("filter"), "whitespace is not a tag:\n{raw}");
}

#[test]
fn re_saving_a_views_own_tag_filter_is_allowed() {
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Papers", Renderer::Board, Some("status"), Some("paper")).unwrap();
    // Rename the tag: this surface wrote that filter, so it may faithfully rewrite it.
    save_view(dir.path(), "Papers", Renderer::Board, Some("status"), Some("reading")).unwrap();
    let raw = fs::read_to_string(views_dir(dir.path()).join("papers.view")).unwrap();
    assert!(raw.contains("- tag: reading"), "{raw}");
    assert!(!raw.contains("- tag: paper"), "the old tag is gone, not duplicated:\n{raw}");
}

#[test]
fn a_hand_written_filter_richer_than_one_tag_is_refused_not_flattened() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(views_dir(dir.path())).unwrap();
    // The kind of thing the grammar allows and this screen cannot express.
    fs::write(
        views_dir(dir.path()).join("active.view"),
        "name: Active\nview: board\ngroup_by: status\nfilter:\n  - not:\n      prop: status\n      eq: done\n",
    )
    .unwrap();

    let err = save_view(dir.path(), "Active", Renderer::Board, Some("status"), Some("paper"))
        .expect_err("must refuse rather than drop the user's filter");
    assert!(err.contains("cannot rewrite"), "the refusal says why: {err}");

    // And the file is untouched — a refusal that had already written would be worse than none.
    let raw = fs::read_to_string(views_dir(dir.path()).join("active.view")).unwrap();
    assert!(raw.contains("eq: done"), "the original filter survives the refusal:\n{raw}");
}

/// **A tag may now contain any character**, since `tags` became comma-separated in the same
/// session — so interpolating it into YAML writes a file that either fails to parse or, worse,
/// parses *wrongly*. `#todo` starts a comment: the view listed as healthy and matched nothing.
#[test]
fn a_tag_with_yaml_syntax_in_it_still_makes_a_working_view() {
    for tag in ["#todo", "@home", "a: b", "- x", "*star", "Machine Learning", "y: [1,2]"] {
        let dir = tempdir().unwrap();
        save_view(dir.path(), "V", Renderer::Board, None, Some(tag)).unwrap();
        let listed = list_views(dir.path());
        let v = listed.iter().find(|v| v.name == "V").expect("listed");
        assert!(v.error.is_none(), "tag {tag:?} wrote an unparseable view: {:?}", v.error);

        // Parsing clean is not enough — `#todo` did that. It must carry the tag it was given.
        let raw = fs::read_to_string(views_dir(dir.path()).join("v.view")).unwrap();
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&raw).unwrap();
        let got = parsed["filter"][0]["tag"].as_str();
        assert_eq!(got, Some(tag), "tag {tag:?} did not survive the file:\n{raw}");
    }
}

/// **An empty tag box means "leave the filter alone", not "delete it".** The dialog opens empty
/// and does not prefill, so re-saving a tag-filtered view to change its grouping would otherwise
/// drop the filter silently — the exact guarantee the 2026-08-28 ruling protects, lost by the
/// change that claimed to extend it.
#[test]
fn re_saving_without_a_tag_keeps_the_filter_that_is_there() {
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Papers", Renderer::Board, Some("status"), Some("paper")).unwrap();
    // Same view, new grouping, tag box untouched.
    save_view(dir.path(), "Papers", Renderer::Board, Some("year"), None).unwrap();

    let raw = fs::read_to_string(views_dir(dir.path()).join("papers.view")).unwrap();
    assert!(raw.contains("group_by: year"), "the grouping changed:\n{raw}");
    assert!(raw.contains("tag: paper"), "and the filter survived:\n{raw}");
}

/// **A view the app cannot draw must say so, not quietly draw something else.**
///
/// `renderer: gallery` parsed, loaded clean, and then rendered as a Timeline — so an author who
/// asked for a gallery got a flat day-bucketed list and nothing anywhere said why. That is the one
/// case the "a broken view names itself, never vanishes" discipline had not been applied to,
/// because from the loader's point of view nothing was broken.
#[test]
fn a_gallery_view_names_itself_instead_of_impersonating_a_timeline() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(views_dir(dir.path())).unwrap();
    fs::write(
        views_dir(dir.path()).join("shots.view"),
        "name: Shots\nview: gallery\n",
    )
    .unwrap();

    let listed = list_views(dir.path());
    let v = listed.iter().find(|v| v.name == "Shots").expect("it must still be listed, not vanish");
    assert!(v.renderer.is_none(), "it must not claim a renderer it cannot draw");
    let err = v.error.as_deref().unwrap_or("");
    assert!(err.contains("gallery"), "the error names the problem: {err:?}");
    assert!(
        err.contains("timeline") || err.contains("board"),
        "and says what to do about it: {err:?}"
    );
}

#[test]
fn the_app_will_not_author_a_view_it_cannot_draw() {
    let dir = tempdir().unwrap();
    let err = save_view(dir.path(), "Shots", Renderer::Gallery, None, None).unwrap_err();
    assert!(err.contains("gallery"), "{err}");
    assert!(
        !views_dir(dir.path()).join("shots.view").exists(),
        "a refused save must leave no file behind"
    );
}

/// **Renaming must not be save-then-delete.** That composition looks equivalent: save under the new
/// name, delete the old file. It is not — `save_view` refuses to flatten a filter it cannot express
/// by noticing the target *already exists*, and a brand-new name hits no existing file, so the
/// guard never fires and the fresh file is written without the filter. Deleting the original then
/// destroys the only copy. This is the test that a rename keeps what the author wrote.
#[test]
fn renaming_a_view_keeps_a_filter_the_app_could_never_have_written() {
    use fm_app::views::rename_view;
    let dir = tempdir().unwrap();
    fs::create_dir_all(views_dir(dir.path())).unwrap();
    // A filter far richer than one tag, plus a comment and a key order the app does not produce.
    let hand = "# my own view\nname: Active\nview: board\ngroup_by: status\n\
                filter:\n  - tag: paper\n  - not:\n      prop: status\n      eq: done\n";
    fs::write(views_dir(dir.path()).join("active.view"), hand).unwrap();

    rename_view(dir.path(), "Active", "Reading now").unwrap();

    assert!(!views_dir(dir.path()).join("active.view").exists(), "the old file is gone");
    let moved = fs::read_to_string(views_dir(dir.path()).join("reading-now.view")).unwrap();
    assert!(moved.contains("name: Reading now"), "the label is the new one:\n{moved}");
    // Every other byte survives — the comment, both predicates, the nesting, the order.
    assert!(moved.contains("# my own view"), "the comment survived:\n{moved}");
    assert!(moved.contains("- tag: paper"), "{moved}");
    assert!(moved.contains("- not:"), "the filter the app cannot write survived:\n{moved}");
    assert!(moved.contains("eq: done"), "{moved}");

    // And it still parses as the same working view.
    let v = list_views(dir.path()).into_iter().find(|v| v.name == "Reading now").expect("listed");
    assert!(v.error.is_none(), "{:?}", v.error);
}

#[test]
fn renaming_onto_a_name_already_taken_is_refused_rather_than_overwriting() {
    use fm_app::views::rename_view;
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Papers", Renderer::Board, None, Some("paper")).unwrap();
    save_view(dir.path(), "Reading", Renderer::Timeline, None, None).unwrap();

    let err = rename_view(dir.path(), "Reading", "Papers").unwrap_err();
    assert!(err.contains("already"), "{err}");
    // Both survive untouched — the refusal must not be halfway done.
    let names: Vec<_> = list_views(dir.path()).into_iter().map(|v| v.name).collect();
    assert!(names.contains(&"Papers".to_string()) && names.contains(&"Reading".to_string()), "{names:?}");
    assert!(
        fs::read_to_string(views_dir(dir.path()).join("papers.view")).unwrap().contains("tag: paper"),
        "the view that was already there kept its filter"
    );
}

#[test]
fn a_name_with_yaml_syntax_in_it_still_renames_to_a_working_view() {
    use fm_app::views::rename_view;
    let dir = tempdir().unwrap();
    save_view(dir.path(), "Papers", Renderer::Timeline, None, None).unwrap();
    // `#` starts a comment and `:` splits a key — either would silently corrupt the file if the
    // label were interpolated rather than serialised.
    rename_view(dir.path(), "Papers", "#reading: now").unwrap();
    let v = list_views(dir.path()).into_iter().find(|v| v.error.is_none()).expect("still parses");
    assert_eq!(v.name, "#reading: now");
}
