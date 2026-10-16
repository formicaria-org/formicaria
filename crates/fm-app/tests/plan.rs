//! **A diet or a regime that is followed, and the week measured against it** — `decisions.md#data`,
//! 2026-10-12.
//!
//! Following is a program note that names a library option and carries its bounds. These pin what
//! "in force" means, that only the bounds on the note are held against a person, that a day with
//! nothing logged is not a day of zero, and that a week of training is counted the way the
//! sources count it.

use fm_app::log::{create_notes, set_properties, NoteDraft};
use fm_app::plan::{plan_outlook, plan_preview, plan_profile, plan_progress, CheckProgress};
use fm_core::MemoryStore;
use serde_json::json;
use std::collections::BTreeMap;

fn draft(title: &str, day: &str, props: serde_json::Value) -> NoteDraft {
    serde_json::from_value(json!({ "title": title, "start": day, "props": props })).unwrap()
}
fn check<'a>(checks: &'a [CheckProgress], id: &str) -> &'a CheckProgress {
    checks.iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check '{id}'"))
}

// 2026-10-12 is a Monday; the week measured is the 12th to the 18th.

#[test]
fn nothing_followed_is_said_plainly_and_measures_nothing() {
    let s = MemoryStore::new();
    let p = plan_progress(&s, "food", "2026-10-14").unwrap();
    assert!(p.plan.is_none() && p.note.is_none() && p.checks.is_empty());
    assert_eq!((p.from.as_str(), p.to.as_str()), ("2026-10-12", "2026-10-18"));
    assert!(plan_progress(&s, "music", "2026-10-14").is_err());
}

#[test]
fn a_day_is_measured_against_the_bounds_on_the_note_and_only_those() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            // The WHO option, with fibre and sodium kept and everything else left out by the
            // person: what they left out is not held against them.
            draft(
                "WHO healthy diet",
                "2026-10-12",
                json!({ "program": "food", "plan": "who-healthy-diet",
                        "p_fibre_min": 25_000, "p_sodium_max": 2_000_000, "p_satfat_max": 10 }),
            ),
            draft(
                "Lentils",
                "2026-10-13",
                json!({ "log": "food", "kcal": 900, "fiber_mg": 30_000, "sodium_ug": 2_500_000,
                        "saturated_fat_mg": 5_000 }),
            ),
            // Meant, not eaten: in no total.
            draft("Cake", "2026-10-13", json!({ "log": "food", "kcal": 500, "planned": 1 })),
        ],
    )
    .unwrap();

    let p = plan_progress(&s, "food", "2026-10-13").unwrap();
    assert_eq!(p.plan.as_ref().unwrap().id, "who-healthy-diet");
    let ids: Vec<&str> = p.checks.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["satfat", "fibre", "sodium"], "the library's order, the note's bounds");
    assert_eq!(
        (check(&p.checks, "fibre").value, check(&p.checks, "fibre").state.as_str()),
        (Some(30_000), "met")
    );
    assert_eq!(check(&p.checks, "sodium").state, "over");
    // 5 g of saturated fat is 45 kcal of 900: 5%.
    let sat = check(&p.checks, "satfat");
    assert_eq!((sat.unit.as_str(), sat.value, sat.state.as_str()), ("percent", Some(5), "met"));
    // Monday had nothing logged: it is not a day of zero fibre.
    assert_eq!(check(&p.checks, "fibre").days[0], None);
    assert_eq!(check(&p.checks, "fibre").days[1], Some(30_000));

    // And a day with nothing logged is not "under".
    let empty = plan_progress(&s, "food", "2026-10-14").unwrap();
    assert_eq!(check(&empty.checks, "fibre").state, "none");
}

#[test]
fn the_plan_in_force_is_the_latest_started_and_not_yet_ended() {
    let mut s = MemoryStore::new();
    let made = create_notes(
        &mut s,
        "",
        &[
            draft(
                "First",
                "2026-10-01",
                json!({ "program": "food", "plan": "own-food", "p_energy_max": 2000 }),
            ),
            draft(
                "Second",
                "2026-10-13",
                json!({ "program": "food", "plan": "own-food", "p_energy_max": 1800 }),
            ),
            // Targets set by hand on the day screen name no plan and are not one.
            draft("By hand", "2026-10-14", json!({ "program": "food", "kcal": 1500 })),
            // Another group's plan is not this group's.
            draft(
                "Gym",
                "2026-10-14",
                json!({ "program": "training", "plan": "own-fitness", "p_minutes_min": 100 }),
            ),
        ],
    )
    .unwrap();
    let on = |s: &MemoryStore, day: &str| {
        plan_progress(s, "food", day).unwrap().note.and_then(|n| n.title)
    };
    assert_eq!(on(&s, "2026-10-12").as_deref(), Some("First"));
    assert_eq!(on(&s, "2026-10-15").as_deref(), Some("Second"));
    // Stopping is a last day; the one before it is in force again.
    set_properties(&mut s, &made[1].id, &BTreeMap::from([("due".into(), json!("2026-10-14"))]))
        .unwrap();
    assert_eq!(on(&s, "2026-10-15").as_deref(), Some("First"));
    assert_eq!(on(&s, "2026-10-14").as_deref(), Some("Second"));
}

#[test]
fn a_week_of_training_is_counted_the_way_the_sources_count_it() {
    let mut s = MemoryStore::new();
    let set = |day: &str, n: i64, region: &str| {
        draft(
            "Squat",
            day,
            json!({ "log": "workout", "exercise": "Squat", "set": n, "reps": 8, "load_g": 60_000,
                    "work_g": 480_000, "region": region }),
        )
    };
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "WHO weekly activity",
                "2026-10-12",
                json!({ "program": "training", "plan": "who-activity",
                        "p_minutes_min": 150, "p_strength_min": 2 }),
            ),
            // Moderate: 40 minutes. Vigorous: 30 minutes, worth 60. Light: nothing.
            draft("Walk", "2026-10-12", json!({ "log": "workout", "activity": "Brisk walking", "counts_as": "aerobic", "minutes": 40, "effort": 5 })),
            draft("Run", "2026-10-13", json!({ "log": "workout", "activity": "Running", "counts_as": "aerobic", "minutes": 30, "effort": 8 })),
            draft("Stroll", "2026-10-13", json!({ "log": "workout", "activity": "Walking", "counts_as": "aerobic", "minutes": 60, "effort": 3 })),
            // Strength: a day of sets, and a day of yoga, which the NHS counts as strengthening.
            set("2026-10-14", 1, "lower-push"),
            set("2026-10-14", 2, "lower-push"),
            draft("Yoga", "2026-10-15", json!({ "log": "workout", "activity": "Yoga", "counts_as": "strength", "minutes": 45, "effort": 4 })),
            // Planned for Friday: not done, not counted.
            draft("Squat", "2026-10-16", json!({ "log": "workout", "exercise": "Squat", "set": 1, "reps": 8, "planned": 1, "region": "lower-push" })),
        ],
    )
    .unwrap();

    let p = plan_progress(&s, "fitness", "2026-10-16").unwrap();
    let minutes = check(&p.checks, "minutes");
    assert_eq!(
        (minutes.value, minutes.state.as_str(), minutes.per.as_str()),
        (Some(100), "under", "week")
    );
    assert_eq!(minutes.days[..3], [Some(40), Some(60), None]);
    let strength = check(&p.checks, "strength");
    assert_eq!((strength.value, strength.state.as_str()), (Some(2), "met"));

    // The same week under a strength regime: two sets in one region, on one day.
    create_notes(
        &mut s,
        "",
        &[draft(
            "Strength, twice a week",
            "2026-10-13",
            json!({ "program": "training", "plan": "strength-health", "p_regions_min": 2 }),
        )],
    )
    .unwrap();
    let p = plan_progress(&s, "fitness", "2026-10-16").unwrap();
    let regions = check(&p.checks, "regions");
    // The least-trained region decides: three of the four were not trained at all.
    assert_eq!((regions.value, regions.state.as_str()), (Some(0), "under"));
}

#[test]
fn a_fasting_day_is_a_logged_day_at_a_quarter_of_the_energy_need_or_less() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "Two fasting days a week",
                "2026-10-12",
                json!({ "program": "food", "plan": "fasting-two-days", "p_fastdays_min": 2, "p_energy": 2000 }),
            ),
            draft("Soup", "2026-10-12", json!({ "log": "food", "kcal": 480 })),
            draft("Dinner", "2026-10-13", json!({ "log": "food", "kcal": 1900 })),
            draft("Broth", "2026-10-15", json!({ "log": "food", "kcal": 500 })),
        ],
    )
    .unwrap();
    let p = plan_progress(&s, "food", "2026-10-18").unwrap();
    let fast = check(&p.checks, "fastdays");
    // Monday and Thursday at 500 kcal or less; Tuesday not; the days with nothing logged are
    // not fasting days, because nothing says they were.
    assert_eq!((fast.value, fast.state.as_str()), (Some(2), "met"));
    assert_eq!(fast.days, [Some(1), Some(0), None, Some(1), None, None, None]);
}

#[test]
fn a_day_kept_is_what_the_person_says_and_belongs_to_the_plan_it_was_said_of() {
    let mut s = MemoryStore::new();
    let made = create_notes(
        &mut s,
        "",
        &[
            draft(
                "Time-restricted eating",
                "2026-10-12",
                json!({ "program": "food", "plan": "time-restricted-eating", "p_kept_min": 7 }),
            ),
            draft("Kept", "2026-10-12", json!({ "log": "kept", "plan": "time-restricted-eating" })),
            draft("Kept", "2026-10-13", json!({ "log": "kept", "plan": "time-restricted-eating" })),
            draft("Kept", "2026-10-13", json!({ "log": "kept", "plan": "nordic-2023" })),
        ],
    )
    .unwrap();
    let p = plan_progress(&s, "food", "2026-10-13").unwrap();
    let kept = check(&p.checks, "kept");
    assert_eq!((kept.value, kept.state.as_str()), (Some(2), "under"));
    assert_eq!(p.kept_today.as_deref(), Some(made[2].id.as_str()));
    assert!(plan_progress(&s, "food", "2026-10-14").unwrap().kept_today.is_none());
}

#[test]
fn a_profile_is_a_note_and_a_preview_is_the_option_worked_out_for_it() {
    let mut s = MemoryStore::new();
    assert_eq!(plan_profile(&s).unwrap().id, None);
    let before = plan_preview(&s, "energy-for-me", "2026-10-12").unwrap();
    assert_eq!(before.missing, ["sex", "born", "height_cm", "weight_g", "pal_x10"]);
    assert_eq!(before.energy, None);

    create_notes(
        &mut s,
        "",
        &[draft(
            "About me",
            "",
            json!({ "profile": "me", "sex": "male", "born": 1991, "height_cm": 180,
                    "weight_g": 80_000, "pal_x10": 16 }),
        )],
    )
    .unwrap();
    let p = plan_preview(&s, "energy-for-me", "2026-10-12").unwrap();
    assert!(p.missing.is_empty());
    assert_eq!(p.energy, Some(2798));
    assert_eq!(p.bounds["energy"].max, Some(2798));
    assert_eq!(p.bounds["protein"].min, Some(66_400)); // 0.83 g × 80 kg

    // An option that needs nothing asks for nothing, and is the same for everybody.
    let who = plan_preview(&s, "who-activity", "2026-10-12").unwrap();
    assert!(who.missing.is_empty() && who.energy.is_none());
    assert_eq!(who.bounds["minutes"].min, Some(150));
    assert!(plan_preview(&s, "no-such-plan", "2026-10-12").is_err());

    // A weight written down since is the person's weight: the latest weighing, not the form.
    create_notes(
        &mut s,
        "",
        &[
            draft("Weight", "2026-10-10", json!({ "log": "body", "weight_g": 79_000 })),
            draft("Weight", "2026-10-11", json!({ "log": "body", "weight_g": 78_000 })),
        ],
    )
    .unwrap();
    assert_eq!(plan_profile(&s).unwrap().weight_g, Some(78_000));
    assert_eq!(plan_profile(&s).unwrap().sex.as_deref(), Some("male"));
}

/// With a time on each entry, the hours without eating are measured: from the last thing with
/// energy the day before to the first that day. A drink with no energy does not end a fast, and a
/// day with no time to go by has nothing to measure.
#[test]
fn a_nights_fast_is_measured_from_the_times_of_what_was_logged() {
    let mut s = MemoryStore::new();
    let food =
        |title: &str, at: &str, kcal: i64| draft(title, at, json!({ "log": "food", "kcal": kcal }));
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "Eating in a window",
                "2026-10-05",
                json!({ "program": "food", "plan": "time-restricted-eating", "p_fast_min": 840 }),
            ),
            // The Sunday before the week: its last meal starts Monday's fast.
            food("Dinner", "2026-10-11T19:30", 600),
            food("Black coffee", "2026-10-12T07:00", 2),
            food("Lunch", "2026-10-12T12:00", 500),
            food("Dinner", "2026-10-12T20:00", 700),
            food("Breakfast", "2026-10-13T08:00", 300),
            // Wednesday's entry has no time: nothing to measure that day or the next.
            food("Something", "2026-10-14", 400),
        ],
    )
    .unwrap();
    let mon = plan_progress(&s, "food", "2026-10-12").unwrap();
    let c = check(&mon.checks, "fast");
    assert_eq!((c.unit.as_str(), c.per.as_str()), ("duration", "day"));
    assert_eq!((c.value, c.state.as_str()), (Some(16 * 60 + 30), "met"));
    let tue = plan_progress(&s, "food", "2026-10-13").unwrap();
    assert_eq!(check(&tue.checks, "fast").value, Some(12 * 60));
    assert_eq!(check(&tue.checks, "fast").state, "under");
    let wed = plan_progress(&s, "food", "2026-10-14").unwrap();
    assert_eq!(check(&wed.checks, "fast").state, "none");
    assert_eq!(check(&wed.checks, "fast").days[..3], [Some(990), Some(720), None]);
}

/// A day on the menu is held against the diet too — when asked, and only then: what is planned is
/// in no account of what a person did.
#[test]
fn what_is_planned_counts_only_when_the_outlook_is_asked_for() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "Fibre",
                "2026-10-05",
                json!({ "program": "food", "plan": "who-healthy-diet", "p_fibre_min": 25_000 }),
            ),
            draft("Oats", "2026-10-13", json!({ "log": "food", "kcal": 300, "fiber_mg": 8_000 })),
            draft(
                "Bean chili",
                "2026-10-13",
                json!({ "log": "food", "kcal": 500, "fiber_mg": 18_000, "planned": 1 }),
            ),
        ],
    )
    .unwrap();
    let did = plan_progress(&s, "food", "2026-10-13").unwrap();
    assert_eq!(
        (check(&did.checks, "fibre").value, check(&did.checks, "fibre").state.as_str()),
        (Some(8_000), "under")
    );
    let would = plan_outlook(&s, "food", "2026-10-13").unwrap();
    assert_eq!(
        (check(&would.checks, "fibre").value, check(&would.checks, "fibre").state.as_str()),
        (Some(26_000), "met")
    );
}

/// A protocol known by its name is followed by doing it and saying so: the days of the week that
/// have an activity logged under that name. Another activity, however hard, is not it.
#[test]
fn a_named_protocol_counts_the_days_it_was_logged_under_its_name() {
    let mut s = MemoryStore::new();
    let act = |name: &str, day: &str| {
        draft(
            name,
            day,
            json!({ "log": "workout", "activity": name, "counts_as": "aerobic",
                    "minutes": 38, "effort": 9 }),
        )
    };
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "4×4",
                "2026-10-05",
                json!({ "program": "training", "plan": "norwegian-4x4", "p_sessions_min": 2 }),
            ),
            act("4×4 intervals", "2026-10-12"),
            act("4×4 intervals", "2026-10-12"),
            act("Running", "2026-10-13"),
            act("4×4 INTERVALS", "2026-10-15"),
        ],
    )
    .unwrap();
    let wed = plan_progress(&s, "fitness", "2026-10-14").unwrap();
    let c = check(&wed.checks, "sessions");
    // Two sessions on one day are one day; the run is not a 4×4.
    assert_eq!((c.value, c.state.as_str()), (Some(1), "under"));
    let fri = plan_progress(&s, "fitness", "2026-10-16").unwrap();
    let c = check(&fri.checks, "sessions");
    assert_eq!((c.value, c.state.as_str()), (Some(2), "met"));
    // The week's example layouts come with the fitness library, Sunday to Saturday.
    let lib = fm_app::plan::plan_library(&fm_core::MemoryStore::new(), "fitness").unwrap();
    assert_eq!(lib.weeks[0].examples.len(), 3);
    assert!(lib.weeks[0].examples.iter().all(|e| e.days.len() == 7));
    assert!(fm_app::plan::plan_library(&fm_core::MemoryStore::new(), "food")
        .unwrap()
        .weeks
        .is_empty());
}

/// Every regime but "My own" has a week to start from; each is seven days from Monday, says what
/// it is and whose, and names only exercise cards that exist.
#[test]
fn every_fitness_regime_has_a_week_to_start_from() {
    let lib = fm_app::plan::plan_library(&fm_core::MemoryStore::new(), "fitness").unwrap();
    let cards = fm_app::log::exercise_library().unwrap();
    let ids: std::collections::BTreeSet<&str> =
        cards["exercises"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap()).collect();
    for p in &lib.plans {
        // A count written down each day (steps) is not a regime of sessions: no week to lay out.
        let counted = p.checks.iter().all(|c| c.log == "body");
        if p.rests_on == "own" || counted {
            continue;
        }
        assert!(!p.examples.is_empty(), "{} has no week to start from", p.id);
        for e in &p.examples {
            assert!(["published", "assembled"].contains(&e["kind"].as_str().unwrap()), "{}", p.id);
            assert!(e["note"].as_str().unwrap().len() > 20, "{}", p.id);
            assert!(e["url"].as_str().unwrap().starts_with("https://"), "{}", p.id);
            let week = e["week"].as_array().unwrap();
            let days: Vec<&str> = week.iter().map(|d| d["day"].as_str().unwrap()).collect();
            assert_eq!(days, ["mon", "tue", "wed", "thu", "fri", "sat", "sun"], "{}", p.id);
            for item in week.iter().flat_map(|d| d["items"].as_array().unwrap()) {
                for x in item["exercises"].as_array().into_iter().flatten() {
                    let card = x["card"].as_str().unwrap();
                    assert!(ids.contains(card), "{}: no card {card}", p.id);
                }
            }
        }
    }
}

/// A diet's example day is either a body's own, as published, or chosen by arithmetic from the
/// carried recipes — and then names recipes that exist, and says what it falls short of.
#[test]
fn a_diets_example_day_is_published_or_chosen_from_recipes_that_exist() {
    let lib = fm_app::plan::plan_library(&fm_core::MemoryStore::new(), "food").unwrap();
    let recipes = fm_app::log::recipe_library().unwrap();
    let ids: std::collections::BTreeSet<&str> =
        recipes["recipes"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
    let mut with = 0;
    for p in &lib.plans {
        for e in &p.examples {
            with += 1;
            assert!(e["note"].as_str().unwrap().len() > 20, "{}", p.id);
            match e["kind"].as_str().unwrap() {
                "published" => {
                    assert!(!e["day"]["meals"].as_array().unwrap().is_empty(), "{}", p.id)
                }
                "selected" => {
                    assert!(e["against"].as_str().unwrap().len() > 20, "{}", p.id);
                    for r in e["recipes"].as_array().unwrap() {
                        let id = r["recipe"].as_str().unwrap();
                        assert!(ids.contains(id), "{}: no recipe {id}", p.id);
                    }
                }
                other => panic!("{}: an example of kind {other}", p.id),
            }
        }
    }
    assert!(with >= 7);
}

// ── A base and add-ons, trends, the watch set and the opt-in flags (2026-10-16) ─────────────────

/// **One plan per question**, so WHO's weekly activity and a strength regime are followed together;
/// a later plan of the *same* question replaces the earlier (`decisions.md#ui`, 2026-10-16).
#[test]
fn a_plan_per_question_is_followed_and_a_later_one_of_the_same_question_replaces_it() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "WHO weekly activity",
                "2026-10-12",
                json!({ "program": "training", "plan": "who-activity", "question": "moving",
                        "p_minutes_min": 150, "p_strength_min": 2 }),
            ),
            draft(
                "Strength, twice a week",
                "2026-10-13",
                json!({ "program": "training", "plan": "strength-health", "question": "stronger",
                        "p_regions_min": 2 }),
            ),
            draft("Walk", "2026-10-14", json!({ "log": "workout", "activity": "Brisk walking", "counts_as": "aerobic", "minutes": 30, "effort": 5 })),
        ],
    )
    .unwrap();
    let p = plan_progress(&s, "fitness", "2026-10-16").unwrap();
    let both: Vec<&str> = p.followed.iter().map(|f| f.plan.as_ref().unwrap().id.as_str()).collect();
    assert_eq!(both, ["who-activity", "strength-health"], "in the order of the questions");
    // The first is what the screens that show one plan read.
    assert_eq!(p.plan.as_ref().unwrap().id, "who-activity");
    // One walk counts for every plan it satisfies.
    assert_eq!(check(&p.followed[0].checks, "minutes").value, Some(30));

    // WHO 65 and over answers the same question: it replaces WHO, the strength regime stays.
    create_notes(
        &mut s,
        "",
        &[draft(
            "WHO weekly activity, 65 and over",
            "2026-10-15",
            json!({ "program": "training", "plan": "who-activity-65", "question": "moving",
                    "p_minutes_min": 150 }),
        )],
    )
    .unwrap();
    let p = plan_progress(&s, "fitness", "2026-10-16").unwrap();
    let now: Vec<&str> = p.followed.iter().map(|f| f.plan.as_ref().unwrap().id.as_str()).collect();
    assert_eq!(now, ["who-activity-65", "strength-health"]);
}

/// **A trend is the last four weeks**, oldest first: a day check's average over the days that had
/// something, `None` for a week with nothing logged.
#[test]
fn a_check_carries_its_four_week_trend() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "WHO healthy diet",
                "2026-09-28",
                json!({ "program": "food", "plan": "who-healthy-diet", "question": "healthy",
                        "p_fibre_min": 25_000 }),
            ),
            // The week of the 28th: two days, 20 g and 24 g → 22 g.
            draft("Oats", "2026-09-29", json!({ "log": "food", "kcal": 500, "fiber_mg": 20_000 })),
            draft("Beans", "2026-09-30", json!({ "log": "food", "kcal": 500, "fiber_mg": 24_000 })),
            // The week of the 5th: nothing logged.
            // This week: one day so far, 30 g.
            draft(
                "Lentils",
                "2026-10-13",
                json!({ "log": "food", "kcal": 500, "fiber_mg": 30_000 }),
            ),
        ],
    )
    .unwrap();
    let p = plan_progress(&s, "food", "2026-10-14").unwrap();
    let fibre = check(&p.checks, "fibre");
    assert_eq!(fibre.trend, [None, Some(22_000), None, Some(30_000)]);
}

/// **The "worth watching" set** is measured whichever plan is followed, or none, with the
/// person's own numbers — and a line their profile hides is left out.
#[test]
fn the_watch_set_is_measured_without_a_plan_and_follows_the_profile() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft("About me", "2026-10-01", json!({ "profile": "me", "sex": "female", "born": 1950, "weight_g": 60_000 })),
            draft("Lunch", "2026-10-13", json!({ "log": "food", "kcal": 600, "fiber_mg": 12_000, "protein_mg": 40_000, "sodium_ug": 900_000, "saturated_fat_mg": 4_000 })),
        ],
    )
    .unwrap();
    let w = fm_app::plan::plan_watch(&s, "food", "2026-10-13").unwrap();
    let ids: Vec<&str> = w.checks.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["satfat", "sodium", "fibre", "protein"]);
    assert_eq!(check(&w.checks, "fibre").state, "under");
    // Born in 1950: past 65, so PROT-AGE's 1.0 g per kg — 60 g, not EFSA's 49.8 g.
    assert_eq!(check(&w.checks, "protein").min, Some(60_000));

    // Told of kidney disease: protein is left out of the set, not worked out.
    let me = plan_profile(&s).unwrap().id.unwrap();
    set_properties(&mut s, &me, &BTreeMap::from([("kidney".into(), json!(true))])).unwrap();
    let w = fm_app::plan::plan_watch(&s, "food", "2026-10-13").unwrap();
    assert!(w.checks.iter().all(|c| c.id != "protein"));

    let f = fm_app::plan::plan_watch(&s, "fitness", "2026-10-13").unwrap();
    let ids: Vec<&str> = f.checks.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["minutes", "strength"]);
}

/// **A flag only uses a number a body publishes, or leaves a number out** — and a plan that is not
/// for the person is left out of the library, by name.
#[test]
fn flags_swap_in_published_numbers_and_leave_out_what_is_not_for_the_person() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[draft(
            "About me",
            "2026-10-01",
            json!({ "profile": "me", "sex": "female", "born": 1990, "height_cm": 165,
                    "weight_g": 60_000, "pal_x10": 16 }),
        )],
    )
    .unwrap();
    let efsa = plan_preview(&s, "efsa-reference", "2026-10-12").unwrap();
    assert_eq!(efsa.bounds["iron"].min, Some(16_000), "before the menopause");
    assert_eq!(efsa.bounds["folate"].min, Some(330));
    assert!(efsa.bounds["potassium"].min.is_some());
    let energy = plan_preview(&s, "energy-for-me", "2026-10-12").unwrap().energy.unwrap();

    let me = plan_profile(&s).unwrap().id.unwrap();
    let flag = |s: &mut MemoryStore, k: &str, v: bool| {
        set_properties(s, &me, &BTreeMap::from([(k.to_string(), json!(v))])).unwrap();
    };
    flag(&mut s, "postmenopausal", true);
    assert_eq!(
        plan_preview(&s, "efsa-reference", "2026-10-12").unwrap().bounds["iron"].min,
        Some(11_000)
    );
    flag(&mut s, "pregnant", true);
    assert_eq!(
        plan_preview(&s, "efsa-reference", "2026-10-12").unwrap().bounds["folate"].min,
        Some(600)
    );
    assert!(plan_preview(&s, "energy-for-me", "2026-10-12").unwrap().bounds["energy"]
        .max
        .is_none());
    flag(&mut s, "pregnant", false);
    flag(&mut s, "breastfeeding", true);
    let bf = plan_preview(&s, "energy-for-me", "2026-10-12").unwrap();
    assert_eq!(bf.energy, Some(energy + 500), "EFSA's 500 kcal for lactation");
    assert_eq!(
        plan_preview(&s, "efsa-reference", "2026-10-12").unwrap().bounds["folate"].min,
        Some(500)
    );
    flag(&mut s, "kidney", true);
    let k = plan_preview(&s, "efsa-reference", "2026-10-12").unwrap();
    assert_eq!((k.bounds["potassium"].min, k.bounds["protein"].min), (None, None));

    let lib = fm_app::plan::plan_library(&s, "food").unwrap();
    let fasting = ["fasting-two-days", "fasting-alternate-days", "time-restricted-eating"];
    assert!(lib.plans.iter().all(|p| !fasting.contains(&p.id.as_str())), "breastfeeding");
    assert_eq!(lib.hidden, fasting);
}

/// WHO's protein share is a floor: 10–15% "is generally sufficient", and no ceiling is held.
#[test]
fn whos_protein_has_no_upper_limit() {
    let s = MemoryStore::new();
    let who = plan_preview(&s, "who-healthy-diet", "2026-10-12").unwrap();
    assert_eq!((who.bounds["protein"].min, who.bounds["protein"].max), (Some(10), None));
}

/// **Steps are written down like a weight** — an entry of the body log — and the steps card reads
/// them from there, beside the training log the other regimes read.
#[test]
fn the_steps_card_reads_the_steps_written_down() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            draft(
                "Steps a day",
                "2026-10-12",
                json!({ "program": "training", "plan": "steps-daily", "question": "steps", "p_steps_min": 7_000 }),
            ),
            draft("Steps", "2026-10-13", json!({ "log": "body", "steps": 8_200 })),
            draft("Walk", "2026-10-13", json!({ "log": "workout", "activity": "Brisk walking", "counts_as": "aerobic", "minutes": 30, "effort": 5 })),
        ],
    )
    .unwrap();
    let p = plan_progress(&s, "fitness", "2026-10-13").unwrap();
    let steps = check(&p.checks, "steps");
    assert_eq!((steps.value, steps.state.as_str()), (Some(8_200), "met"));
}

/// DASH is rich in potassium: with a kidney flag that number is left out, the rest kept.
#[test]
fn dash_leaves_out_potassium_for_kidney_disease() {
    let mut s = MemoryStore::new();
    let dash = plan_preview(&s, "dash", "2026-10-12").unwrap();
    assert_eq!(dash.bounds["potassium"].min, Some(4_700_000));
    assert_eq!(dash.bounds["satfat"].max, Some(6));
    create_notes(
        &mut s,
        "",
        &[draft("About me", "2026-10-01", json!({ "profile": "me", "kidney": true }))],
    )
    .unwrap();
    let dash = plan_preview(&s, "dash", "2026-10-12").unwrap();
    assert_eq!(dash.bounds["potassium"].min, None);
    assert_eq!(dash.bounds["fibre"].min, Some(30_000));
}
