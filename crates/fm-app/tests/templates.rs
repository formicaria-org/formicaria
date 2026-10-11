//! Templates: "New from template" spins a fresh note off a template's body. A template is just a
//! note tagged `template` — no new `Kind`, no reserved property — so `templates()` is a plain
//! `TagsAll(["template"])` filter over the note set, and tagging/untagging makes/unmakes one.

use fm_app::commands::{agenda, board, capture, set_property, templates};
use fm_app::log::{create_notes, log_days, log_recipes, NoteDraft};
use fm_app::views::totals_named;
use fm_core::MemoryStore;
use serde_json::json;

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

/// A template carries the structure of what is made from it — and is never itself one of them
/// (`decisions.md#ui`, 2026-10-12).
#[test]
fn a_template_is_inert_not_a_task_a_program_in_force_or_a_recipe() {
    let mut s = MemoryStore::new();
    let draft = |title: &str, tags: &[&str], start: &str, props: serde_json::Value| -> NoteDraft {
        serde_json::from_value(
            json!({ "title": title, "tags": tags, "start": start, "props": props }),
        )
        .unwrap()
    };
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "Task",
                &["template", "task"],
                "",
                json!({ "status": "todo", "due": "2026-10-20" }),
            ),
            draft("Write up", &["task"], "", json!({ "status": "todo", "due": "2026-10-20" })),
            draft("Diet program", &["template"], "", json!({ "program": "food", "kcal": 1 })),
            draft(
                "Recipe",
                &["template", "recipe"],
                "",
                json!({ "log": "food", "recipe": "x", "servings": 2 }),
            ),
        ],
    )
    .unwrap();

    let cards: Vec<String> = board(&s, "status")
        .unwrap()
        .columns
        .into_iter()
        .flat_map(|c| c.cards)
        .filter_map(|c| c.title)
        .collect();
    assert_eq!(cards, ["Write up"], "the task is on the board; the template for tasks is not");
    let due: Vec<_> = agenda(&s).unwrap().into_iter().filter_map(|c| c.title).collect();
    assert_eq!(due, ["Write up"]);

    let day_view = totals_named(&[], "Day").unwrap();
    let day = &log_days(&s, &day_view, "2026-10-12", "2026-10-12", false).unwrap().days[0];
    assert!(day.targets.is_empty(), "a program template with no start is not the targets in force");
    assert!(log_recipes(&s, &day_view).unwrap().is_empty(), "nor is a recipe template a recipe");

    // Still a template, and still listed as one — but for the one that carries `log`, which is a
    // logged entry's key and hides whatever carries it.
    let listed: Vec<_> = templates(&s).unwrap().into_iter().filter_map(|m| m.title).collect();
    assert_eq!(listed.len(), 2);
    assert!(listed.contains(&"Task".to_string()) && listed.contains(&"Diet program".to_string()));
}
