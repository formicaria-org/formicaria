//! **T18** (the coach plan's S.8): a workout written by the encoder decodes back — through
//! `fit::decode`, the reader the import uses — to the same steps, and no step carries a percentage
//! target. Every session here is invented.

use super::*;
use crate::fit;

fn step(part: Part, name: &str, length: Length, target: Target) -> Step {
    Step {
        part,
        name: name.into(),
        notes: format!("{name}, as planned"),
        length,
        target,
        weight_kg: None,
    }
}

/// An invented interval run: warm-up by time, 4 × (800 m at a pace, 2 min recovery under a heart
/// rate), cool-down open.
fn intervals() -> Workout {
    Workout {
        name: "Intervals · Run".into(),
        sport: Sport::Running,
        created: 1_780_000_000,
        items: vec![
            Item::Step(step(Part::Warmup, "Warm-up", Length::Seconds(600.0), Target::Open)),
            Item::Repeat {
                times: 4,
                steps: vec![
                    step(
                        Part::Work,
                        "Work",
                        Length::Metres(800.0),
                        Target::Speed { low: 1000.0 / 240.0, high: 1000.0 / 225.0 },
                    ),
                    step(
                        Part::Recover,
                        "Recovery",
                        Length::Seconds(120.0),
                        Target::HeartRate { low: 110, high: 135 },
                    ),
                ],
            },
            Item::Step(step(Part::Cooldown, "Cool-down", Length::Open, Target::Open)),
        ],
    }
}

#[test]
fn t18_a_workout_reads_back_step_for_step() {
    let bytes = encode(&intervals()).unwrap();
    // The reader's own frame check passes: header, signature, sizes and both checksums.
    fit::check_frame(&bytes).unwrap();
    let f = fit::decode(&bytes).unwrap();
    assert_eq!(f.file_id.as_ref().and_then(|i| i.file_type.as_deref()), Some("workout"));
    let w = f.workout.clone().unwrap();
    assert_eq!(w.name.as_deref(), Some("Intervals · Run"));
    assert_eq!(w.sport.as_deref(), Some("running"));
    assert_eq!(w.steps, Some(5));
    assert!(f.sessions.is_empty(), "a workout is not an activity");

    let s = &f.steps;
    assert_eq!(s.len(), 5);
    let idx: Vec<_> = s.iter().map(|x| x.index).collect();
    assert_eq!(idx, vec![Some(0), Some(1), Some(2), Some(3), Some(4)]);
    // Warm-up, 10 minutes.
    assert_eq!(s[0].intensity.as_deref(), Some("warmup"));
    assert_eq!(s[0].duration_s, Some(600.0));
    assert_eq!(s[0].name.as_deref(), Some("Warm-up"));
    assert_eq!(s[0].notes.as_deref(), Some("Warm-up, as planned"));
    // Work: 800 m at 4:00–3:45 a km, as m/s × 1000.
    assert_eq!(s[1].intensity.as_deref(), Some("active"));
    assert_eq!(s[1].distance_m, Some(800.0));
    assert_eq!(s[1].target_type.as_deref(), Some("speed"));
    assert_eq!(s[1].target_value, Some(0));
    assert_eq!(s[1].target_low, Some(4167));
    assert_eq!(s[1].target_high, Some(4444));
    // Recovery: 2 minutes under 110–135 bpm, written with the +100 offset.
    assert_eq!(s[2].intensity.as_deref(), Some("recovery"));
    assert_eq!(s[2].duration_s, Some(120.0));
    assert_eq!(s[2].target_type.as_deref(), Some("heart_rate"));
    assert_eq!((s[2].target_low, s[2].target_high), (Some(210), Some(235)));
    // The repeat: back to step 1, four times in all.
    assert_eq!(s[3].duration_type.as_deref(), Some("repeat_until_steps_cmplt"));
    assert_eq!(s[3].duration_count, Some(1));
    assert_eq!(s[3].target_value, Some(4));
    // Cool-down: until the lap key.
    assert_eq!(s[4].duration_type.as_deref(), Some("open"));
    assert_eq!(s[4].target_type.as_deref(), Some("open"));
}

#[test]
fn t18_no_step_carries_a_percentage_target() {
    let f = fit::decode(&encode(&intervals()).unwrap()).unwrap();
    for s in &f.steps {
        match s.target_type.as_deref() {
            Some("heart_rate") => {
                // A custom range above 100 is bpm; at or below it would be % of maximum.
                assert_eq!(s.target_value, Some(0), "a zone, not a range: {s:?}");
                assert!(s.target_low.unwrap() > HR_OFFSET && s.target_high.unwrap() > HR_OFFSET);
            }
            Some("speed") => assert_eq!(s.target_value, Some(0), "{s:?}"),
            Some("open") | None => {}
            other => panic!("a target fm never writes: {other:?}"),
        }
    }
}

#[test]
fn a_strength_step_keeps_its_reps_and_weight() {
    let mut s = step(Part::Work, "Squat", Length::Reps(8), Target::Open);
    s.weight_kg = Some(42.5);
    let w = Workout {
        name: "Strength".into(),
        sport: Sport::Strength,
        created: 1_780_000_000,
        items: vec![Item::Repeat { times: 3, steps: vec![s] }],
    };
    let f = fit::decode(&encode(&w).unwrap()).unwrap();
    assert_eq!(f.workout.unwrap().sport.as_deref(), Some("training"));
    assert_eq!(f.steps[0].duration_type.as_deref(), Some("reps"));
    assert_eq!(f.steps[0].duration_count, Some(8));
    assert_eq!(f.steps[0].weight_kg, Some(42.5));
    assert_eq!(f.steps[1].target_value, Some(3));
}

#[test]
fn what_a_watch_cannot_follow_is_refused_with_a_sentence() {
    let w = |target| Workout {
        name: "x".into(),
        sport: Sport::Running,
        created: 1_780_000_000,
        items: vec![Item::Step(step(Part::Work, "Work", Length::Open, target))],
    };
    assert!(encode(&w(Target::HeartRate { low: 5, high: 300 })).unwrap_err().contains("bpm"));
    let empty = Workout { items: vec![], ..w(Target::Open) };
    assert!(encode(&empty).unwrap_err().contains("no steps"));
    let long = Workout {
        items: (0..=MAX_STEPS)
            .map(|_| Item::Step(step(Part::Work, "Work", Length::Open, Target::Open)))
            .collect(),
        ..w(Target::Open)
    };
    assert!(encode(&long).unwrap_err().contains("at most"));
}

#[test]
fn a_long_name_is_cut_on_a_character_boundary() {
    let mut w = intervals();
    w.name = "é".repeat(200);
    let f = fit::decode(&encode(&w).unwrap()).unwrap();
    let n = f.workout.unwrap().name.unwrap();
    assert!(n.len() <= MAX_TEXT && n.chars().all(|c| c == 'é'), "{n}");
}
