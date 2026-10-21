//! **Counting over days**: the arithmetic a diary is read with (`decisions.md#ui` `#data`,
//! 2026-10-22, *a fifth group, Health*).
//!
//! Everything here takes a series of days that are already a person's own record. It counts and
//! measures; it never labels. A day's value is `Some(n)` when the person recorded it, `None` when
//! they did not. **A recorded 0 is data, a missing day is not**, and every count says how many
//! days it could see (HealthKit separates `notPresent` from `unspecified` the same way; IHS
//! counts missing days as low compliance).
//!
//! Pure: dates in, numbers out. No store, no clock, no path. Seam 1 holds.

use std::collections::BTreeMap;
use time::{Date, Duration};

/// A tracker's values by day. A day absent from the map was not recorded.
pub type Series = BTreeMap<Date, i64>;

/// What a span of days holds for one tracker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DayCount {
    /// Days in the span, first and last included.
    pub of: usize,
    /// Days the person recorded a value on.
    pub recorded: usize,
    /// Recorded days whose value is at least the threshold asked for.
    pub at_least: usize,
    /// The recorded values added up. A mean is this over [`DayCount::recorded`], worked out by
    /// whoever shows it, so nothing here is rounded.
    pub sum: i64,
}

/// The days from `from` to `to`, both included. Empty when `to` is before `from`.
fn each_day(from: Date, to: Date) -> impl Iterator<Item = Date> {
    std::iter::successors((from <= to).then_some(from), move |d| d.next_day().filter(|n| *n <= to))
}

/// How many days of `from..=to` were recorded, and how many of them reached `min`.
///
/// A "headache day" in ICHD-3's sense — *"affected by headache for any part or the whole of the
/// day"* — is `min = 1` on a severity tracker; *moderate or severe* is `min = 2` on a `_0to3` one.
pub fn count(series: &Series, from: Date, to: Date, min: i64) -> DayCount {
    let mut c = DayCount::default();
    for d in each_day(from, to) {
        c.of += 1;
        if let Some(v) = series.get(&d) {
            c.recorded += 1;
            c.sum = c.sum.saturating_add(*v);
            if *v >= min {
                c.at_least += 1;
            }
        }
    }
    c
}

/// The longest run of consecutive days in `from..=to` whose value is at least `min`. A day not
/// recorded ends a run: nothing is assumed about it.
///
/// Used for *"should not be used for longer than 5 days"* (NHS, decongestant sprays).
pub fn longest_run(series: &Series, from: Date, to: Date, min: i64) -> usize {
    let (mut best, mut run) = (0, 0);
    for d in each_day(from, to) {
        if series.get(&d).is_some_and(|v| *v >= min) {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

/// The Monday a day's week starts on (ISO 8601).
pub fn week_of(d: Date) -> Date {
    d - Duration::days(i64::from(d.weekday().number_days_from_monday()))
}

/// Per ISO week (keyed by its Monday) overlapping `from..=to`: the days in the span that reached
/// `min`. A week only partly inside the span counts only its days inside it, and its [`DayCount::of`]
/// says how many those are, so a part week is never read as a whole one.
///
/// Used for *"Bedding should be washed weekly"* (AAAAI) and *"more than 2 days a week"* (NHS).
pub fn per_week(series: &Series, from: Date, to: Date, min: i64) -> BTreeMap<Date, DayCount> {
    let mut weeks: BTreeMap<Date, DayCount> = BTreeMap::new();
    for d in each_day(from, to) {
        let w = weeks.entry(week_of(d)).or_default();
        w.of += 1;
        if let Some(v) = series.get(&d) {
            w.recorded += 1;
            w.sum = w.sum.saturating_add(*v);
            if *v >= min {
                w.at_least += 1;
            }
        }
    }
    weeks
}

/// The days between each marked day and the next, in order: a cycle's length is *"from the first
/// day of bleeding in one cycle to the first day of the next cycle"* (FIGO System 1, as restated
/// by Jain, Munro and Critchley 2023). The last marked day starts a cycle that has not ended, so
/// `n` marks give `n - 1` lengths. Duplicate marks count once.
pub fn intervals(marks: &[Date]) -> Vec<i64> {
    let mut m: Vec<Date> = marks.to_vec();
    m.sort_unstable();
    m.dedup();
    m.windows(2).map(|w| (w[1] - w[0]).whole_days()).collect()
}

/// ICHD-3's window around a first day of menstruation: *"day 1 ± 2 (ie, days −2 to +3)"*, where
/// *"the first day of menstruation is day 1 and the preceding day is day −1; there is no day 0"*
/// (A1.1.1). So day −2 is two calendar days before the mark and day +3 two after it: five days.
pub fn ichd_menstrual_window(first_day: Date) -> (Date, Date) {
    (first_day - Duration::days(2), first_day + Duration::days(2))
}

/// For each marked day, whether any day of `events` reaching `min` falls in its window. In the
/// marks' order, one answer per distinct mark. This is a count for the person to read
/// (*"headache logged on days −2 to +3 in X of your last 3 cycles"*), never a classification: the
/// diary cannot establish ICHD-3's 1.1 criteria.
pub fn in_window(
    marks: &[Date],
    events: &Series,
    min: i64,
    window: impl Fn(Date) -> (Date, Date),
) -> Vec<bool> {
    let mut m: Vec<Date> = marks.to_vec();
    m.sort_unstable();
    m.dedup();
    m.into_iter()
        .map(|mark| {
            let (a, b) = window(mark);
            events.range(a..=b).any(|(_, v)| *v >= min)
        })
        .collect()
}

/// Days of `events` reaching `min` that fall in **no** mark's window: *"and additionally at other
/// times of the cycle"* (ICHD-3 A1.1.2) is a question about these. Only days from the first mark's
/// window on are looked at; before the first recorded cycle nothing is known about where a day
/// falls.
pub fn outside_windows(
    marks: &[Date],
    events: &Series,
    min: i64,
    window: impl Fn(Date) -> (Date, Date),
) -> usize {
    let spans: Vec<(Date, Date)> = marks.iter().map(|m| window(*m)).collect();
    let Some(start) = spans.iter().map(|s| s.0).min() else { return 0 };
    events
        .range(start..)
        .filter(|(d, v)| **v >= min && !spans.iter().any(|(a, b)| a <= *d && *d <= b))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Month, Weekday};

    fn day(m: u8, d: u8) -> Date {
        Date::from_calendar_date(2026, Month::try_from(m).unwrap(), d).unwrap()
    }
    fn series(v: &[(u8, u8, i64)]) -> Series {
        v.iter().map(|(m, d, x)| (day(*m, *d), *x)).collect()
    }

    #[test]
    fn a_recorded_zero_is_counted_as_recorded_and_a_missing_day_is_not() {
        let s = series(&[(10, 1, 0), (10, 2, 2), (10, 4, 1)]);
        let c = count(&s, day(10, 1), day(10, 5), 1);
        assert_eq!(c, DayCount { of: 5, recorded: 3, at_least: 2, sum: 3 });
        // Moderate or severe on a 0–3 tracker.
        assert_eq!(count(&s, day(10, 1), day(10, 5), 2).at_least, 1);
    }

    #[test]
    fn a_span_that_ends_before_it_starts_is_empty() {
        let s = series(&[(10, 1, 3)]);
        assert_eq!(count(&s, day(10, 5), day(10, 1), 1), DayCount::default());
        assert_eq!(longest_run(&s, day(10, 5), day(10, 1), 1), 0);
    }

    #[test]
    fn a_run_is_ended_by_a_day_that_was_not_recorded() {
        // 1–3 used, 4 not recorded, 5–6 used, 7 recorded as none.
        let s = series(&[(10, 1, 1), (10, 2, 1), (10, 3, 1), (10, 5, 1), (10, 6, 1), (10, 7, 0)]);
        assert_eq!(longest_run(&s, day(10, 1), day(10, 7), 1), 3);
        assert_eq!(longest_run(&s, day(10, 4), day(10, 7), 1), 2);
    }

    #[test]
    fn weeks_start_on_monday_and_a_part_week_says_how_many_days_it_had() {
        // 2026-10-19 is a Monday.
        assert_eq!(day(10, 19).weekday(), Weekday::Monday);
        assert_eq!(week_of(day(10, 25)), day(10, 19));
        assert_eq!(week_of(day(10, 19)), day(10, 19));
        let s = series(&[(10, 17, 1), (10, 20, 1), (10, 21, 0)]);
        let w = per_week(&s, day(10, 17), day(10, 25), 1);
        assert_eq!(w.len(), 2);
        assert_eq!(w[&day(10, 12)], DayCount { of: 2, recorded: 1, at_least: 1, sum: 1 });
        assert_eq!(w[&day(10, 19)], DayCount { of: 7, recorded: 2, at_least: 1, sum: 1 });
    }

    #[test]
    fn cycle_lengths_run_from_one_first_day_to_the_next() {
        let marks = [day(10, 1), day(8, 4), day(9, 2), day(9, 2)];
        // Aug 4 → Sep 2 is 29 days; Sep 2 → Oct 1 is 29 days; the cycle from Oct 1 has not ended.
        assert_eq!(intervals(&marks), vec![29, 29]);
        assert!(intervals(&[day(10, 1)]).is_empty());
        assert!(intervals(&[]).is_empty());
    }

    #[test]
    fn the_ichd_window_is_day_minus_two_to_day_plus_three_with_no_day_zero() {
        // First day Oct 10 is day 1: day −2 is Oct 8, day +3 is Oct 12.
        assert_eq!(ichd_menstrual_window(day(10, 10)), (day(10, 8), day(10, 12)));
    }

    #[test]
    fn headaches_are_counted_inside_and_outside_the_windows() {
        let marks = [day(8, 10), day(9, 8), day(10, 6)];
        // One in August's window (Aug 9), none in September's (Sep 7 is recorded as none), one in
        // October's (Oct 8, its last day); Sep 20 is outside every window.
        let headache = series(&[(8, 9, 2), (9, 7, 0), (9, 20, 1), (10, 8, 3), (8, 1, 2)]);
        assert_eq!(in_window(&marks, &headache, 1, ichd_menstrual_window), vec![true, false, true]);
        // Aug 1 is before the first window, so it is not "outside" anything; Sep 20 is.
        assert_eq!(outside_windows(&marks, &headache, 1, ichd_menstrual_window), 1);
        assert_eq!(outside_windows(&[], &headache, 1, ichd_menstrual_window), 0);
    }
}
