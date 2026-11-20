//! The decoder over synthetic files (the coach plan's T2 and T4). Every file here is written by
//! `fit_synth.rs` with invented values; no real person's file is read or committed.

use super::*;

#[path = "fit_synth.rs"]
mod synth;
use synth::*;

/// A small seeded generator: the same inputs on every run, no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Recompute both checksums, so a mutation reaches the decoder rather than stopping at the frame.
fn reseal(bytes: &mut [u8]) {
    if bytes.len() < 16 {
        return;
    }
    let h = crc16(&bytes[..12]);
    bytes[12..14].copy_from_slice(&h.to_le_bytes());
    let n = bytes.len();
    let c = crc16(&bytes[..n - 2]);
    bytes[n - 2..].copy_from_slice(&c.to_le_bytes());
}

#[test]
fn t2_the_decoder_never_panics_on_random_or_mutated_input() {
    let fixtures =
        [steady_run().encode(), intervals().encode(), pool_swim().encode(), strength().encode()];
    let mut rng = Rng(0x5eed_f17e_0000_0001);
    let mut decoded = 0;
    for round in 0..1500 {
        let input: Vec<u8> = match round % 3 {
            // Pure noise, sometimes behind a plausible header.
            0 => {
                let n = rng.below(400);
                let mut b: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
                if n > 16 && rng.below(2) == 0 {
                    b[0] = 14;
                    b[8..12].copy_from_slice(b".FIT");
                    let data = (n - 16) as u32;
                    b[4..8].copy_from_slice(&data.to_le_bytes());
                    reseal(&mut b);
                }
                b
            }
            // A real fixture with bytes flipped, checksums left as they were.
            1 => {
                let mut b = fixtures[rng.below(fixtures.len())].clone();
                for _ in 0..1 + rng.below(8) {
                    let i = rng.below(b.len());
                    b[i] ^= rng.next() as u8 | 1;
                }
                b
            }
            // Flipped and resealed: the decoder itself has to cope.
            _ => {
                let mut b = fixtures[rng.below(fixtures.len())].clone();
                for _ in 0..1 + rng.below(6) {
                    let i = 14 + rng.below(b.len() - 16);
                    b[i] = rng.next() as u8;
                }
                reseal(&mut b);
                b
            }
        };
        // The assertion is that this returns at all.
        if decode(&input).is_ok() {
            decoded += 1;
        }
    }
    // Some resealed mutations still decode; the point is that none of them panicked.
    assert!(decoded > 0, "no mutated input decoded — the resealing is not reaching the decoder");
}

#[test]
fn t4_a_steady_run() {
    let spec = steady_run();
    let f = decode(&spec.encode()).unwrap();
    let id = f.file_id.as_ref().unwrap();
    assert_eq!(id.file_type.as_deref(), Some("activity"));
    assert_eq!(id.serial, Some(spec.serial));
    assert!(f.has_creator);
    assert_eq!(f.local_offset_s, Some(3600));
    let s = &f.sessions[0];
    assert_eq!(s.sport.as_deref(), Some("running"));
    assert_eq!(s.start, Some(i64::from(spec.start) + FIT_EPOCH));
    assert_eq!(s.timer_s, Some(975.0));
    assert_eq!(s.distance_m, Some(3000.0));
    assert_eq!((s.hr_avg, s.hr_max), (Some(145.0), Some(160.0)));
    assert_eq!((s.ascent_m, s.descent_m), (Some(42.0), Some(40.0)));
    assert_eq!(s.workout_rpe, Some(50.0));
    assert_eq!(s.training_effect, Some(3.2));
    assert_eq!(f.laps.len(), 3);
    assert_eq!(f.laps[1].distance_m, Some(1000.0));
    assert!(f.stream.records > 90 && f.stream.with_position == f.stream.records);
    assert_eq!(f.stream.with_hr, f.stream.records);
}

#[test]
fn t4_intervals_carry_their_workout_steps() {
    let f = decode(&intervals().encode()).unwrap();
    assert_eq!(f.steps.len(), 5);
    assert_eq!(f.steps[1].name.as_deref(), Some("Hard"));
    assert_eq!(f.steps[1].duration_s, Some(240.0));
    assert_eq!(f.laps.iter().filter_map(|l| l.wkt_step_index).count(), 5);
}

#[test]
fn t4_over_midnight_keeps_the_watch_offset() {
    for off in [3600, -5 * 3600, 5 * 3600 + 1800, 0] {
        let f = decode(&over_midnight(off).encode()).unwrap();
        assert_eq!(f.local_offset_s, Some(i64::from(off)), "offset {off}");
    }
}

#[test]
fn t4_a_bad_checksum_is_refused_and_says_so() {
    let mut b = steady_run().encode();
    let n = b.len();
    b[n - 1] ^= 0x55;
    match decode(&b) {
        Err(FitError::Damaged(why)) => assert!(why.contains("checksum"), "{why}"),
        other => panic!("{other:?}"),
    }
    // A flipped byte in the middle is caught the same way.
    let mut b = steady_run().encode();
    b[40] ^= 0x01;
    assert!(matches!(decode(&b), Err(FitError::Damaged(_))));
}

#[test]
fn t4_a_truncated_file_is_refused() {
    let b = steady_run().encode();
    for cut in [5, 13, 20, b.len() / 2, b.len() - 1] {
        let r = decode(&b[..cut]);
        assert!(matches!(r, Err(FitError::Damaged(_)) | Err(FitError::NotFit(_))), "{cut}: {r:?}");
    }
    match decode(&b[..b.len() / 2]) {
        Err(FitError::Damaged(why)) => assert!(why.contains("cut short"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn t4_not_a_fit_file() {
    assert!(matches!(
        decode(b"hello, this is text and not a watch file"),
        Err(FitError::NotFit(_))
    ));
    assert!(matches!(decode(b""), Err(FitError::NotFit(_))));
}

#[test]
fn t4_an_unknown_message_is_counted_and_left_alone() {
    let spec = Spec { unknown_message: true, ..steady_run() };
    let f = decode(&spec.encode()).unwrap();
    assert_eq!(f.unknown_messages, 1);
    assert_eq!(f.sessions.len(), 1);
}

#[test]
fn t4_a_pool_swim_has_its_lengths() {
    let f = decode(&pool_swim().encode()).unwrap();
    let s = &f.sessions[0];
    assert_eq!(s.sport.as_deref(), Some("swimming"));
    assert_eq!(s.sub_sport.as_deref(), Some("lap_swimming"));
    assert_eq!(s.pool_length_m, Some(25.0));
    assert_eq!(f.lengths.len(), 12);
    assert_eq!(f.lengths.iter().filter(|l| l.active).count(), 10);
    assert_eq!(f.lengths[5].stroke.as_deref(), Some("butterfly"));
    assert_eq!(f.lengths[10].stroke.as_deref(), Some("drill"));
    assert_eq!(f.lengths[0].strokes, Some(18));
}

#[test]
fn t4_strength_sets_name_their_exercise() {
    let f = decode(&strength().encode()).unwrap();
    assert_eq!(f.sets.len(), 6);
    assert_eq!(f.sets.iter().filter(|s| s.active).count(), 3);
    let first = &f.sets[0];
    assert_eq!(first.category.as_deref(), Some("bench_press"));
    assert_eq!(first.exercise.as_deref(), Some("barbell_bench_press"));
    assert_eq!(first.reps, Some(8));
    assert_eq!(first.weight_kg, Some(60.0));
    assert_eq!(f.sets[4].exercise.as_deref(), Some("goblet_squat"));
    assert!(!f.sets[1].active);
}

#[test]
fn exercise_names_come_from_the_profile() {
    assert_eq!(exercise_name("bench_press", 1).as_deref(), Some("barbell_bench_press"));
    assert_eq!(exercise_name("bench_press", 60_000), None);
    assert_eq!(exercise_name("no_such_category", 1), None);
}

#[test]
fn the_identity_tuple_needs_a_serial_or_a_time() {
    let none = FileId::default();
    assert_eq!(none.identity(), None);
    let some = FileId { serial: Some(7), ..Default::default() };
    assert!(some.identity().is_some());
}
