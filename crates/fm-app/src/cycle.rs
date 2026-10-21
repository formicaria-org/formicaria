//! **The cycle only counts** (`decisions.md#ui` `#data`, 2026-10-22, *a fifth group, Health*, point
//! 8; the sources: `health-plan-2026-10-22.md` §5.4).
//!
//! A cycle is *"from the first day of bleeding in one cycle to the first day of the next cycle"*
//! (FIGO System 1, as restated by Jain, Munro and Critchley 2023), and a first day is a day the
//! person marked `period_start_yes`. Everything here is arithmetic on those days. **Nothing is
//! predicted**: no next period, no fertile window, no ovulation, nothing read from a late period.
//!
//! Two things leave a cycle out of the statistics, and each is said, never silently applied:
//! - **a cycle context** (contraception, pregnancy, postpartum, perimenopause, in the person's own
//!   words): `cycle_context: <words>` on the day it begins, `cycle_context: none` on the day it
//!   ends. FIGO's metrics hold *"provided there has been no pregnancy or puerperium"*; Apple pauses
//!   its deviation notices while a *cycle factor* is set; Li 2020 left hormonal and IUD cycles out;
//! - **missed logging**: `cycle_gap_yes: 1` on any day of a cycle. About 42 % of Clue's users had a
//!   long cycle that was a gap in their logging (Li 2020); here the person says which.

use fm_model::{Object, PropertyValue};
use fm_query::days::{self, Series};
use serde::Serialize;
use std::collections::BTreeMap;
use time::{Date, Duration};

pub const FIRST_DAY: &str = "period_start_yes";
pub const FLOW: &str = "period_flow_0to3";
pub const SPOTTING: &str = "spotting_yes";
pub const CONTEXT: &str = "cycle_context";
pub const GAP: &str = "cycle_gap_yes";
/// FIGO: *"All the metrics are based on the previous 6 months"*.
pub const SIX_MONTHS: i64 = 183;

/// One cycle, from a first day to the next.
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Cycle {
    pub start: String,
    pub length: i64,
    /// Days of it with a Health note at all: a cycle of 31 days with 6 notes is shown as such.
    pub noted: usize,
    /// The note of its first day, to mark it as missed logging from.
    pub note: String,
    /// Why it is left out of the statistics, when it is: `context` (with the words) or `gap`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_out: Option<String>,
}

fn int(o: &Object, key: &str) -> Option<i64> {
    match o.get(key) {
        PropertyValue::Int(v) => Some(v),
        _ => None,
    }
}

fn day_has(notes: &[Object], key: &str, pred: impl Fn(i64) -> bool) -> bool {
    notes.iter().any(|o| int(o, key).is_some_and(&pred))
}

/// The spans a context was in force: from the day it was set to the day before it was cleared, or
/// open to `day` when it never was.
fn contexts(notes: &BTreeMap<Date, Vec<Object>>, day: Date) -> Vec<(Date, Date, String)> {
    let mut out = Vec::new();
    let mut open: Option<(Date, String)> = None;
    for (d, here) in notes.range(..=day) {
        let said = here.iter().find_map(|o| match o.get(CONTEXT) {
            PropertyValue::Text(t) => Some(t.trim().to_string()),
            _ => None,
        });
        let Some(said) = said else { continue };
        if let Some((from, words)) = open.take() {
            out.push((from, *d - Duration::days(1), words));
        }
        if !said.is_empty() && !said.eq_ignore_ascii_case("none") {
            open = Some((*d, said));
        }
    }
    if let Some((from, words)) = open {
        out.push((from, day, words));
    }
    out
}

/// The first days recorded on or before `day`, in order.
pub fn first_days(notes: &BTreeMap<Date, Vec<Object>>, day: Date) -> Vec<Date> {
    notes
        .range(..=day)
        .filter(|(_, n)| day_has(n, FIRST_DAY, |v| v == 1))
        .map(|(d, _)| *d)
        .collect()
}

/// **The cycles that ended in the last six months**, oldest first, each with why it is left out
/// when it is. The cycle under way has not ended and is not among them.
pub fn cycles(notes: &BTreeMap<Date, Vec<Object>>, day: Date) -> Vec<Cycle> {
    let firsts = first_days(notes, day);
    let ctx = contexts(notes, day);
    let since = day - Duration::days(SIX_MONTHS);
    let mut out = Vec::new();
    for w in firsts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a < since {
            continue;
        }
        let inside = notes.range(a..b);
        let noted = inside.clone().filter(|(_, n)| !n.is_empty()).count();
        let gap = inside.clone().any(|(_, n)| day_has(n, GAP, |v| v == 1));
        let context = ctx.iter().find(|(from, to, _)| *from < b && *to >= a);
        let left_out = match (context, gap) {
            (Some((_, _, words)), _) => Some(format!("context: {words}")),
            (None, true) => Some("gap".to_string()),
            (None, false) => None,
        };
        let note =
            notes.get(&a).and_then(|n| n.first()).map(|o| o.id.to_string()).unwrap_or_default();
        out.push(Cycle {
            start: format!("{:04}-{:02}-{:02}", a.year(), a.month() as u8, a.day()),
            length: (b - a).whole_days(),
            noted,
            note,
            left_out,
        });
    }
    out
}

/// The lengths that count: the cycles not left out.
pub fn counted_lengths(cycles: &[Cycle]) -> Vec<i64> {
    cycles.iter().filter(|c| c.left_out.is_none()).map(|c| c.length).collect()
}

/// A day of bleeding: any flow, or spotting — FIGO counts *"the light bleeding, often called
/// spotting"* in a period's length.
fn bleeding(notes: &[Object]) -> bool {
    day_has(notes, FLOW, |v| v >= 1) || day_has(notes, SPOTTING, |v| v == 1)
}

/// The latest period: its first day, and its days of bleeding in a row from it (a day with no note
/// ends it, as it ends any run).
pub fn latest_period(notes: &BTreeMap<Date, Vec<Object>>, day: Date) -> Option<(Date, i64)> {
    let first = *first_days(notes, day).last()?;
    let mut run = 0;
    let mut d = first;
    while d <= day && notes.get(&d).is_some_and(|n| bleeding(n)) {
        run += 1;
        d += Duration::days(1);
    }
    Some((first, run))
}

/// Days of the latest period at flow 3, *heavy*.
pub fn heavy_days(notes: &BTreeMap<Date, Vec<Object>>, first: Date, run: i64) -> i64 {
    (0..run)
        .filter(|i| {
            notes.get(&(first + Duration::days(*i))).is_some_and(|n| day_has(n, FLOW, |v| v == 3))
        })
        .count() as i64
}

/// Days of spotting with no flow, in the last six months.
pub fn spotting_days(notes: &BTreeMap<Date, Vec<Object>>, day: Date) -> i64 {
    notes
        .range(day - Duration::days(SIX_MONTHS)..=day)
        .filter(|(_, n)| day_has(n, SPOTTING, |v| v == 1) && !day_has(n, FLOW, |v| v >= 1))
        .count() as i64
}

/// **Headaches around a period** (ICHD-3 A1.1): for the last `n` first days whose window has
/// passed, whether a headache was recorded on days −2 to +3; and the headache days outside every
/// window since the first of them. A count for the person to read, never a type of migraine.
pub fn around_periods(
    notes: &BTreeMap<Date, Vec<Object>>,
    headache: &Series,
    day: Date,
    n: usize,
) -> (usize, usize, usize) {
    let marks: Vec<Date> = first_days(notes, day)
        .into_iter()
        .filter(|f| days::ichd_menstrual_window(*f).1 <= day)
        .collect();
    let last: Vec<Date> = marks.iter().rev().take(n).rev().copied().collect();
    let hits = days::in_window(&last, headache, 1, days::ichd_menstrual_window);
    let outside = days::outside_windows(&last, headache, 1, days::ichd_menstrual_window);
    (hits.iter().filter(|h| **h).count(), last.len(), outside)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_model::Kind;
    use time::Month;

    fn d(m: Month, day: u8) -> Date {
        Date::from_calendar_date(2026, m, day).unwrap()
    }
    fn note(kv: &[(&str, PropertyValue)]) -> Object {
        let mut o = Object::new(Kind::Note, "");
        for (k, v) in kv {
            o.extra.insert((*k).into(), v.clone());
        }
        o
    }
    fn first(flow: i64) -> Object {
        note(&[(FIRST_DAY, PropertyValue::Int(1)), (FLOW, PropertyValue::Int(flow))])
    }
    fn flow(v: i64) -> Object {
        note(&[(FLOW, PropertyValue::Int(v))])
    }

    #[test]
    fn a_cycle_runs_from_one_first_day_to_the_next_and_the_one_under_way_is_not_counted() {
        let mut n: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
        n.insert(d(Month::July, 1), vec![first(2)]);
        n.insert(d(Month::July, 29), vec![first(2)]);
        n.insert(d(Month::August, 30), vec![first(2)]);
        n.insert(d(Month::September, 28), vec![first(2)]);
        let c = cycles(&n, d(Month::October, 22));
        assert_eq!(c.iter().map(|c| c.length).collect::<Vec<_>>(), vec![28, 32, 29]);
        assert!(c.iter().all(|c| c.left_out.is_none()));
        assert_eq!(c[0].noted, 1, "a cycle with one note says so");
    }

    #[test]
    fn a_context_or_a_marked_gap_leaves_a_cycle_out_and_says_why() {
        let mut n: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
        n.insert(d(Month::June, 1), vec![first(1)]);
        n.insert(d(Month::June, 29), vec![first(1)]);
        // A context set in the second cycle and cleared in the third.
        n.insert(
            d(Month::July, 10),
            vec![note(&[(CONTEXT, PropertyValue::Text("started the pill".into()))])],
        );
        n.insert(d(Month::July, 30), vec![first(1)]);
        n.insert(d(Month::August, 5), vec![note(&[(CONTEXT, PropertyValue::Text("none".into()))])]);
        n.insert(d(Month::August, 28), vec![first(1)]);
        // A fourth cycle the person marked as missed logging.
        n.insert(d(Month::September, 3), vec![note(&[(GAP, PropertyValue::Int(1))])]);
        n.insert(d(Month::October, 9), vec![first(1)]);
        let c = cycles(&n, d(Month::October, 22));
        let why: Vec<Option<&str>> = c.iter().map(|c| c.left_out.as_deref()).collect();
        assert_eq!(
            why,
            vec![
                None,
                Some("context: started the pill"),
                Some("context: started the pill"),
                Some("gap")
            ]
        );
        assert_eq!(counted_lengths(&c), vec![28]);
    }

    #[test]
    fn a_period_is_its_bleeding_days_in_a_row_spotting_included() {
        let mut n: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
        n.insert(d(Month::October, 1), vec![first(3)]);
        n.insert(d(Month::October, 2), vec![flow(3)]);
        n.insert(d(Month::October, 3), vec![flow(2)]);
        n.insert(d(Month::October, 4), vec![note(&[(SPOTTING, PropertyValue::Int(1))])]);
        n.insert(d(Month::October, 5), vec![flow(0)]);
        n.insert(d(Month::October, 15), vec![note(&[(SPOTTING, PropertyValue::Int(1))])]);
        let (f, run) = latest_period(&n, d(Month::October, 22)).unwrap();
        assert_eq!((f, run), (d(Month::October, 1), 4));
        assert_eq!(heavy_days(&n, f, run), 2);
        // Spotting with no flow: the 4th and the 15th.
        assert_eq!(spotting_days(&n, d(Month::October, 22)), 2);
    }

    #[test]
    fn headaches_are_counted_around_the_last_three_periods_and_outside_them() {
        let mut n: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
        for f in
            [d(Month::July, 10), d(Month::August, 8), d(Month::September, 6), d(Month::October, 4)]
        {
            n.insert(f, vec![first(2)]);
        }
        let headache: Series =
            [(d(Month::August, 6), 2), (d(Month::October, 6), 1), (d(Month::September, 20), 1)]
                .into_iter()
                .collect();
        // The last three: Aug 8 (Aug 6 is day −2), Sep 6 (none), Oct 4 (Oct 6 is day +3).
        assert_eq!(around_periods(&n, &headache, d(Month::October, 22), 3), (2, 3, 1));
        // On Oct 5 the October window has not passed: the three are July, August, September.
        assert_eq!(around_periods(&n, &headache, d(Month::October, 5), 3).1, 3);
    }
}
