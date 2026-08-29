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
