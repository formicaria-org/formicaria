//! **A diet or a regime that is followed, and the week measured against it** — `decisions.md#data`,
//! 2026-10-12.
//!
//! Following is a program note that names a library option and carries its bounds. These pin what
//! "in force" means, that only the bounds on the note are held against a person, that a day with
//! nothing logged is not a day of zero, and that a week of training is counted the way the
//! sources count it.

use fm_app::log::{create_notes, set_properties, NoteDraft};
use fm_app::plan::{plan_preview, plan_profile, plan_progress, CheckProgress};
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
