//! **Logged entries, and what a day of them comes to** — `decisions.md#data`, 2026-10-11.
//!
//! An entry is a note carrying `log: <kind>`, a day in `start`, and whole numbers whose unit is in
//! the key. These pin the four commands over them, and the two promises around them: a day's total
//! says how complete it is, and an entry never shows up among the things you plan.

use fm_app::commands;
use fm_app::log::{create_notes, log_days, log_recent, log_recipes, set_properties, NoteDraft};
use fm_app::views::{group_of, run_shipped, shipped_views, totals_named, with_shipped, Renderer};
use fm_core::MemoryStore;
use serde_json::json;
use std::collections::BTreeMap;

fn draft(title: &str, day: &str, props: serde_json::Value) -> NoteDraft {
    serde_json::from_value(json!({ "title": title, "start": day, "props": props })).unwrap()
}
fn food(title: &str, day: &str, slot: &str, kcal: i64, protein_mg: i64) -> NoteDraft {
    draft(
        title,
        day,
        json!({ "log": "food", "slot": slot, "kcal": kcal, "protein_mg": protein_mg }),
    )
}
fn day_view() -> fm_app::views::TotalsDef {
    totals_named(&[], "Day").expect("the food day view ships with the app")
}

#[test]
fn a_day_comes_to_the_sum_of_its_entries() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            food("Oats", "2026-10-11", "breakfast", 228, 8_000),
            food("Chicken", "2026-10-11", "lunch", 248, 46_000),
            food("Rice", "2026-10-12", "lunch", 205, 4_000),
        ],
    )
    .unwrap();

    let r = log_days(&s, &day_view(), "2026-10-10", "2026-10-12", false).unwrap();
    let days: Vec<_> = r.days.iter().map(|d| (d.day.as_str(), d.of, d.sums["kcal"].sum)).collect();
    // Every day of the span is answered, the empty one included — a chart needs its gaps.
    assert_eq!(days, [("2026-10-10", 0, 0), ("2026-10-11", 2, 476), ("2026-10-12", 1, 205)]);
    assert_eq!(r.days[1].sums["protein_mg"].sum, 54_000);
    assert!(r.days[1].rows.is_none(), "rows are sent only when asked for");
}

/// A food typed in by hand has energy and nothing else. The day's protein is then a sum over
/// fewer entries than the day has — and it says so, instead of passing for the whole.
#[test]
fn a_total_says_how_many_entries_it_is_from() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            food("Oats", "2026-10-11", "breakfast", 228, 8_000),
            draft(
                "Takeaway",
                "2026-10-11",
                json!({ "log": "food", "slot": "dinner", "kcal": 720 }),
            ),
        ],
    )
    .unwrap();
    let day = &log_days(&s, &day_view(), "2026-10-11", "2026-10-11", false).unwrap().days[0];
    assert_eq!((day.of, day.sums["kcal"].n, day.sums["protein_mg"].n), (2, 2, 1));
    assert_eq!(day.sums["protein_mg"].sum, 8_000);
}

#[test]
fn a_planned_entry_is_in_the_days_rows_and_in_none_of_its_sums() {
    let mut s = MemoryStore::new();
    let made = create_notes(
        &mut s,
        "",
        &[
            food("Oats", "2026-10-11", "breakfast", 228, 8_000),
            draft(
                "Salmon",
                "2026-10-11",
                json!({ "log": "food", "slot": "dinner", "kcal": 300, "planned": 1 }),
            ),
        ],
    )
    .unwrap();
    let day = |s: &MemoryStore| {
        log_days(s, &day_view(), "2026-10-11", "2026-10-11", true).unwrap().days.remove(0)
    };
    let before = day(&s);
    assert_eq!((before.of, before.sums["kcal"].sum), (1, 228));
    assert_eq!(before.rows.as_ref().unwrap().len(), 2, "it is on the day, to be confirmed");

    // Confirming it is clearing the key — an ordinary write.
    set_properties(&mut s, &made[1].id, &BTreeMap::from([("planned".to_string(), json!(""))]))
        .unwrap();
    let after = day(&s);
    assert_eq!((after.of, after.sums["kcal"].sum), (2, 528));
}

#[test]
fn a_days_entries_come_in_the_order_of_the_views_slots() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            food("Dessert", "2026-10-11", "dinner", 200, 0),
            food("Toast", "2026-10-11", "breakfast", 90, 0),
            food("Soup", "2026-10-11", "lunch", 150, 0),
            food("Egg", "2026-10-11", "breakfast", 70, 0),
        ],
    )
    .unwrap();
    let day = log_days(&s, &day_view(), "2026-10-11", "2026-10-11", true).unwrap().days.remove(0);
    let titles: Vec<_> = day.rows.unwrap().into_iter().map(|r| r.title.unwrap()).collect();
    // Slot order from the view; within a slot, the order they were made in.
    assert_eq!(titles, ["Toast", "Egg", "Soup", "Dessert"]);
}

/// Targets live on an ordinary note carrying `program: food`. The one in force on a day is the
/// latest that had started by then — so a trend draws the target as a step, not a flat line.
#[test]
fn the_targets_in_force_are_the_latest_program_started_by_that_day() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft("Maintain", "2026-09-01", json!({ "program": "food", "kcal": 2400 })),
            draft(
                "Cut",
                "2026-10-11",
                json!({ "program": "food", "kcal": 2100, "protein_mg": 140000 }),
            ),
            // A training program must never supply food targets.
            draft("Strength", "2026-10-01", json!({ "program": "workout", "kcal": 9999 })),
        ],
    )
    .unwrap();
    let r = log_days(&s, &day_view(), "2026-10-10", "2026-10-11", false).unwrap();
    assert_eq!(r.days[0].targets.get("kcal"), Some(&2400));
    assert!(!r.days[0].targets.contains_key("protein_mg"));
    assert_eq!(r.days[1].targets.get("kcal"), Some(&2100));
    assert_eq!(r.days[1].targets.get("protein_mg"), Some(&140_000));

    let before_any = log_days(&s, &day_view(), "2026-08-01", "2026-08-01", false).unwrap();
    assert!(before_any.days[0].targets.is_empty(), "no program had started yet");
}

#[test]
fn a_program_is_something_you_plan_and_an_entry_is_not() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft("Cut", "2026-10-11", json!({ "program": "food", "kcal": 2100 })),
            food("Oats", "2026-10-11", "breakfast", 228, 8_000),
        ],
    )
    .unwrap();
    let titles = |v: Vec<fm_app::dto::ObjectMeta>| -> Vec<String> {
        v.into_iter().filter_map(|m| m.title).collect()
    };
    assert_eq!(titles(commands::recent(&s).unwrap()), ["Cut"]);
    let board = commands::board(&s, "status").unwrap();
    let on_board: Vec<_> =
        board.columns.iter().flat_map(|c| c.cards.iter()).filter_map(|c| c.title.clone()).collect();
    assert_eq!(on_board, ["Cut"]);
    // Search still finds what you plan; a food log is searched from its own sheet.
    assert!(titles(commands::search(&s, "Oats").unwrap()).is_empty());
    assert_eq!(titles(commands::search(&s, "Cut").unwrap()), ["Cut"]);
}

/// The guard that stops a board grouped by `log`, plus one drag, taking a note off every view.
#[test]
fn what_kind_of_entry_a_note_is_cannot_be_set_afterwards() {
    let mut s = MemoryStore::new();
    let made =
        create_notes(&mut s, "", &[food("Oats", "2026-10-11", "breakfast", 228, 0)]).unwrap();
    let id = &made[0].id;

    let e = commands::set_property(&mut s, id, "log", "workout").unwrap_err().to_string();
    assert!(e.contains("set when the entry is made"), "{e}");

    // …and the several-at-once write refuses the lot, writing none of it.
    let props = BTreeMap::from([("kcal".to_string(), json!(999)), ("log".to_string(), json!(""))]);
    assert!(set_properties(&mut s, id, &props).is_err());
    let day = log_days(&s, &day_view(), "2026-10-11", "2026-10-11", false).unwrap().days.remove(0);
    assert_eq!(day.sums["kcal"].sum, 228, "a refused write changed nothing");
}

#[test]
fn a_log_kind_is_shape_tested_where_it_is_set() {
    let mut s = MemoryStore::new();
    let bad = draft("x", "2026-10-11", json!({ "log": "Not A Kind" }));
    let e = create_notes(&mut s, "", &[bad]).unwrap_err().to_string();
    assert!(e.contains("one short lower-case word"), "{e}");
}

/// `12.5` would be stored as text and left out of every total without a word. Refused instead,
/// and — because every draft is checked first — nothing of the batch is written.
#[test]
fn a_number_that_is_not_whole_is_refused_and_nothing_is_written() {
    let mut s = MemoryStore::new();
    let drafts = [
        food("Oats", "2026-10-11", "breakfast", 228, 0),
        draft("Milk", "2026-10-11", json!({ "log": "food", "kcal": 12.5 })),
    ];
    let e = create_notes(&mut s, "", &drafts).unwrap_err().to_string();
    assert!(e.contains("whole number") && e.contains("smaller unit"), "{e}");
    let day = log_days(&s, &day_view(), "2026-10-11", "2026-10-11", false).unwrap().days.remove(0);
    assert_eq!(day.of, 0);
}

#[test]
fn several_properties_are_one_write_and_the_answer_undoes_it() {
    let mut s = MemoryStore::new();
    let made =
        create_notes(&mut s, "", &[food("Oats", "2026-10-11", "breakfast", 228, 8_000)]).unwrap();
    let id = &made[0].id;
    let change =
        BTreeMap::from([("kcal".to_string(), json!(140)), ("amount_g".to_string(), json!(37))]);
    let previous = set_properties(&mut s, id, &change).unwrap();
    assert_eq!(previous["kcal"].as_deref(), Some("228"));
    assert_eq!(previous["amount_g"], None, "it had none");

    // Undo is the same call with the answer: a missing value goes back as "remove it".
    let back: BTreeMap<String, serde_json::Value> =
        previous.into_iter().map(|(k, v)| (k, json!(v.unwrap_or_default()))).collect();
    set_properties(&mut s, id, &back).unwrap();
    let after = commands::get(&s, id).unwrap().unwrap();
    assert_eq!(after.meta.props.get("kcal"), Some(&json!(228)), "typed as a number again");
    assert!(!after.meta.props.contains_key("amount_g"));
}

#[test]
fn what_was_logged_before_is_offered_once_each_most_used_in_this_slot_first() {
    let mut s = MemoryStore::new();
    let mut drafts = vec![
        food("Oats", "2026-10-09", "breakfast", 220, 0),
        food("Oats", "2026-10-10", "breakfast", 228, 0),
        food("Rice", "2026-10-09", "lunch", 200, 0),
        food("Rice", "2026-10-10", "lunch", 205, 0),
        food("Rice", "2026-10-11", "lunch", 210, 0),
        food("Coffee", "2026-10-11", "breakfast", 2, 0),
    ];
    // Never happened: a planned one, and a saved food with no day.
    drafts.push(draft(
        "Salmon",
        "2026-10-11",
        json!({ "log": "food", "slot": "breakfast", "planned": 1 }),
    ));
    drafts.push(draft("My smoothie", "", json!({ "log": "food", "kcal": 300 })));
    create_notes(&mut s, "", &drafts).unwrap();

    let got = log_recent(&s, &day_view(), "breakfast", "", 10).unwrap();
    let titles: Vec<_> = got.iter().map(|r| (r.title.as_str(), r.count)).collect();
    // Twice at breakfast, once at breakfast, never at breakfast.
    assert_eq!(titles, [("Oats", 2), ("Coffee", 1), ("Rice", 3)]);
    assert_eq!(got[0].props["kcal"], json!(228), "with the values it had the last time");
    assert_eq!(got[0].last, "2026-10-10");

    let rice = log_recent(&s, &day_view(), "", "ric", 10).unwrap();
    assert_eq!(rice.len(), 1);
}

#[test]
fn a_span_must_be_days_in_order_and_not_too_many() {
    let s = MemoryStore::new();
    assert!(log_days(&s, &day_view(), "2026-10-12", "2026-10-11", false).is_err());
    assert!(log_days(&s, &day_view(), "2024-01-01", "2026-10-11", false).is_err());
    assert!(log_days(&s, &day_view(), "yesterday", "2026-10-11", false).is_err());
}

// ── the views that come with the app ─────────────────────────────────────────────────────────────

#[test]
fn every_shipped_view_parses_and_can_be_drawn() {
    let shipped = shipped_views();
    // Day and Trends for food; Session and Volume for training.
    assert_eq!(shipped.len(), 4);
    for v in &shipped {
        assert!(v.shipped && v.error.is_none() && v.group.is_some(), "{v:?}");
        assert_eq!(v.renderer, Some(Renderer::Totals));
        totals_named(&[], &v.name).unwrap_or_else(|e| panic!("{}: {e}", v.name));
    }
}

#[test]
fn a_totals_view_answers_what_it_is_and_no_rows() {
    let mut s = MemoryStore::new();
    create_notes(&mut s, "", &[food("Oats", "2026-10-11", "breakfast", 228, 0)]).unwrap();
    let r = run_shipped(&s, "Day").unwrap().unwrap();
    assert_eq!(r.renderer, Renderer::Totals);
    assert_eq!(r.rows.unwrap().len(), 0, "its days are asked for by span, never dumped whole");
    let def = r.totals.unwrap();
    assert_eq!(def.log, "food");
    assert_eq!(def.metrics[0].key, "kcal");
    assert!(run_shipped(&s, "No such view").is_none());
}

#[test]
fn a_group_is_its_views_in_listed_order_and_a_vaults_own_file_replaces_a_shipped_one() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("views")).unwrap();
    // The same name as a shipped view, renamed metrics — how a person changes what one calls things.
    std::fs::write(
        dir.path().join("views/day.view"),
        "name: Day\nview: totals\ngroup: Food\nlog: food\nadd: food\nmetrics:\n  - { key: kcal, label: Calories, role: primary }\n",
    )
    .unwrap();
    let all = with_shipped(fm_app::views::list_views(dir.path()));
    let day: Vec<_> = all.iter().filter(|v| v.name == "Day").collect();
    assert_eq!(day.len(), 1, "listed once");
    assert!(!day[0].shipped, "and it is the vault's own");

    let group = group_of(&all, "Trends").unwrap();
    assert_eq!((group.name.as_str(), group.views.len()), ("Food", 2));
    assert!(group.views.contains(&"Day".to_string()));
    assert!(group_of(&all, "nothing").is_none());

    let def = totals_named(&[dir.path().to_path_buf()], "Day").unwrap();
    assert_eq!(def.metrics[0].label, "Calories");
}

#[test]
fn a_totals_view_that_cannot_be_drawn_names_itself() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("views")).unwrap();
    std::fs::write(dir.path().join("views/x.view"), "name: Broken\nview: totals\nlog: food\n")
        .unwrap();
    let listed = fm_app::views::list_views(dir.path());
    assert_eq!(listed[0].name, "Broken");
    assert!(listed[0].renderer.is_none());
    assert!(listed[0].error.as_deref().unwrap().contains("metrics"), "{:?}", listed[0].error);
}

/// A weight taken twice in a day is one weight. The day's answer carries the sum and the count, and
/// the view says the value is a mean — so nothing is computed differently, only read differently.
#[test]
fn a_measured_value_is_read_as_the_mean_of_the_days_entries() {
    use fm_app::views::Agg;
    // No view that ships measures anything since *Weight* was withdrawn (`decisions.md#ui`,
    // 2026-10-12), so the one read here is a vault's own — which is also how a person adds one.
    let vault = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(vault.path().join("views")).unwrap();
    std::fs::write(
        vault.path().join("views/Weight.view"),
        "name: Weight\nview: totals\nlog: body\nadd: body\nspan: range\nmetrics:\n  \
         - { key: weight_g, label: Weight, role: primary, agg: mean }\n",
    )
    .unwrap();
    let def = totals_named(&[vault.path().to_path_buf()], "Weight").unwrap();
    assert_eq!((def.log.as_str(), def.metrics[0].agg), ("body", Agg::Mean));
    assert_eq!(day_view().metrics[0].agg, Agg::Sum);

    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft("Weight", "2026-10-11", json!({ "log": "body", "weight_g": 78400 })),
            draft("Weight", "2026-10-11", json!({ "log": "body", "weight_g": 78000 })),
        ],
    )
    .unwrap();
    let day = log_days(&s, &def, "2026-10-11", "2026-10-11", false).unwrap().days.remove(0);
    let w = &day.sums["weight_g"];
    assert_eq!((w.sum, w.n), (156_400, 2), "the mean, 78,200 g, is the reader's to take");
    // And a weight is not food: the food day does not see it.
    let food = log_days(&s, &day_view(), "2026-10-11", "2026-10-11", false).unwrap().days.remove(0);
    assert_eq!(food.of, 0);
}

/// A program may have a last day. After it, the targets are whatever was in force before — which
/// is how "eight weeks at 1,900, then back to maintenance" is two notes and no bookkeeping.
#[test]
fn a_program_with_an_end_stops_supplying_targets_the_day_after() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft("Maintain", "2026-09-01", json!({ "program": "food", "kcal": 2400 })),
            draft(
                "Cut",
                "2026-10-01",
                json!({ "program": "food", "kcal": 1900, "due": "2026-10-10" }),
            ),
        ],
    )
    .unwrap();
    let r = log_days(&s, &day_view(), "2026-10-10", "2026-10-11", false).unwrap();
    assert_eq!(r.days[0].targets.get("kcal"), Some(&1900), "its last day still counts");
    assert_eq!(
        r.days[1].targets.get("kcal"),
        Some(&2400),
        "then the earlier one is in force again"
    );
}

#[test]
fn a_recipe_is_the_sum_of_its_ingredients_and_says_what_an_ingredient_lacked() {
    let mut s = MemoryStore::new();
    let saved = |title: &str, props: serde_json::Value| -> NoteDraft {
        serde_json::from_value(json!({ "title": title, "props": props })).unwrap()
    };
    create_notes(
        &mut s,
        "",
        &[
            saved("Dal", json!({ "log": "food", "recipe": "r1", "servings": 4 })),
            saved(
                "Lentils",
                json!({ "log": "food", "recipe": "r1", "amount_g": 300, "kcal": 1050,
                        "protein_mg": 75_000, "iron_ug": 19_500 }),
            ),
            saved(
                "Onion",
                json!({ "log": "food", "recipe": "r1", "amount_g": 150, "kcal": 60,
                        "protein_mg": 1_650 }),
            ),
            // Another recipe's ingredient, and an ingredient of no recipe at all.
            saved("Rice", json!({ "log": "food", "recipe": "r2", "kcal": 700 })),
            food("Oats", "2026-10-11", "breakfast", 228, 8_000),
        ],
    )
    .unwrap();

    let all = log_recipes(&s, &day_view()).unwrap();
    assert_eq!(all.len(), 1, "a recipe is the note with servings; `r2` has none");
    let dal = &all[0];
    assert_eq!((dal.title.as_str(), dal.servings, dal.of), ("Dal", 4, 2));
    assert_eq!((dal.sums["kcal"].sum, dal.sums["kcal"].n), (1110, 2));
    assert_eq!((dal.sums["amount_g"].sum, dal.sums["protein_mg"].sum), (450, 76_650));
    assert_eq!(
        (dal.sums["iron_ug"].sum, dal.sums["iron_ug"].n),
        (19_500, 1),
        "one of the two ingredients had an iron value, and the total says so"
    );

    // Saved, not eaten: in no day, and not offered as something logged before.
    let day = &log_days(&s, &day_view(), "2026-10-11", "2026-10-11", false).unwrap().days[0];
    assert_eq!((day.of, day.sums["kcal"].sum), (1, 228));
    let recent = log_recent(&s, &day_view(), "", "", 20).unwrap();
    assert_eq!(recent.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), ["Oats"]);
}

/// The recipe's own note is an ordinary, visible note; its ingredients are still entries
/// (`decisions.md#ui`, 2026-10-12).
#[test]
fn a_recipe_note_is_a_visible_note_and_its_ingredients_are_not() {
    let mut s = MemoryStore::new();
    let saved = |title: &str, tags: &[&str], props: serde_json::Value| -> NoteDraft {
        serde_json::from_value(json!({ "title": title, "tags": tags, "props": props })).unwrap()
    };
    create_notes(
        &mut s,
        "",
        &[
            saved("Dal", &["recipe"], json!({ "recipe": "r1", "servings": 4 })),
            saved(
                "Lentils",
                &[],
                json!({ "log": "food", "recipe": "r1", "amount_g": 300, "kcal": 1050 }),
            ),
            // A set of a workout that happens to carry the key is not an ingredient of anything.
            saved("Squat", &[], json!({ "log": "workout", "recipe": "r1", "reps": 5 })),
            // A recipe with nothing in it yet — made from a template, say — is still a recipe.
            saved("Soup", &["recipe"], json!({ "recipe": "r2", "servings": 2 })),
        ],
    )
    .unwrap();

    let all = log_recipes(&s, &day_view()).unwrap();
    let got: Vec<_> = all.iter().map(|r| (r.title.as_str(), r.servings, r.of)).collect();
    assert_eq!(got, [("Dal", 4, 1), ("Soup", 2, 0)]);
    assert_eq!(all[0].sums["kcal"].sum, 1050);
    assert!(!all[0].sums.contains_key("reps"));

    // In the Library: the two recipes, and none of what goes in them.
    let mut seen: Vec<_> =
        commands::recent(&s).unwrap().into_iter().filter_map(|n| n.title).collect();
    seen.sort();
    assert_eq!(seen, ["Dal", "Soup"]);
}

/// **The names of the shipped groups and views are these files' and nobody else's** — every
/// system's backend reads them (`decisions.md#ui`, 2026-10-12, *the names never differ*). The two
/// places in the front end that have to say them again are held to them here: the in-memory
/// backend that `pnpm dev` and the UI's tests run on, and the Settings switch that names the groups.
#[test]
fn the_front_end_names_the_shipped_groups_and_views_as_the_files_do() {
    let mock = include_str!("../../../ui/src/lib/mockTracker.ts");
    let settings = include_str!("../../../ui/src/lib/TrackersSection.svelte");
    let shipped = shipped_views();
    assert!(!shipped.is_empty());
    for v in &shipped {
        let group = v.group.as_deref().unwrap();
        assert!(
            mock.contains(&format!("\n  {}: {{", v.name)),
            "the in-memory backend has no view called {} — it is `{}` in the shipped file",
            v.name,
            v.name
        );
        assert!(
            mock.contains(&format!("name: '{group}'")),
            "the in-memory backend has no group called {group}"
        );
        assert!(settings.contains(group), "Settings does not name the group {group}");
    }
    // And it lists no view the app does not ship.
    let defs = mock.split("const DEFS").nth(1).and_then(|t| t.split("\n};").next()).unwrap();
    let listed = defs
        .lines()
        .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.contains(": {"))
        .count();
    assert_eq!(listed, shipped.len(), "the in-memory backend lists a view that is not shipped");
}
