//! Templates: "New from template" spins a fresh note off a template's body. A template is just a
//! note tagged `template` — no new `Kind`, no reserved property — so `templates()` is a plain
//! `TagsAll(["template"])` filter over the note set, and tagging/untagging makes/unmakes one.

use fm_app::commands::{capture, set_property, templates};
use fm_core::MemoryStore;

#[test]
fn a_note_tagged_template_is_listed_and_a_plain_note_is_not() {
    let mut s = MemoryStore::new();
    let tpl = capture(&mut s, "# Meeting\n- [ ] agenda", "").unwrap().id;
    set_property(&mut s, &tpl, "tags", "template").unwrap();
    capture(&mut s, "just an ordinary note", "").unwrap();

    let listed: Vec<String> = templates(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(listed, vec![tpl], "only the note tagged `template` is a template");
}

#[test]
fn untagging_removes_a_note_from_the_template_list() {
    let mut s = MemoryStore::new();
    let id = capture(&mut s, "scaffold", "").unwrap().id;
    set_property(&mut s, &id, "tags", "template").unwrap();
    assert_eq!(templates(&s).unwrap().len(), 1, "tagged: a template");

    set_property(&mut s, &id, "tags", "").unwrap();
    assert!(templates(&s).unwrap().is_empty(), "untagged: no longer a template");
}
