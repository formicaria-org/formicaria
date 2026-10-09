//! Repeating events: the `RRULE` subset real calendars use, expanded into dates.
//!
//! # Why a subset, and which one
//!
//! RFC 5545's recurrence grammar is large — `BYSETPOS`, `BYWEEKNO`, `BYYEARDAY`, hourly rules — and a
//! full implementation is a library's worth of edge cases. What a person's own calendar actually holds
//! is far narrower: *every weekday*, *every Tuesday*, *every other Thursday*, *the first Monday of the
//! month*, *the 15th of each month*, *every year on this date*. Those are read here:
//!
//! - `FREQ` = `DAILY` · `WEEKLY` · `MONTHLY` · `YEARLY`
//! - `INTERVAL`, `COUNT`, `UNTIL`, `WKST`
//! - `BYDAY` — plain weekdays for `DAILY`/`WEEKLY`; with an ordinal (`1MO`, `-1FR`) for `MONTHLY`
//! - `BYMONTHDAY` for `DAILY`/`MONTHLY`, negative counting from the month's end
//!
//! **Anything else makes [`parse`] return `None`, and the caller says so.** Expanding a rule we only
//! half understand would put meetings on days they are not — the one failure a calendar must not
//! have. Refusing shows fewer meetings and *counts* them, which a person can see and act on.
//!
//! # Time zones
//!
//! None, by the same ruling as [`crate::events`]: an occurrence keeps its series' wall-clock time on
//! each date. A weekly 10:00 meeting is 10:00 every week, including across a DST change in its own
//! zone — which is what the organiser meant.

use time::{Date, Duration, Month, Weekday};

/// A parsed rule — only ever one this module can expand exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    freq: Freq,
    interval: i64,
    count: Option<u32>,
    /// Inclusive, by date. A `Z` time on `UNTIL` is compared by its UTC day, which can include or
    /// drop one occurrence that falls within hours of midnight UTC on the last day — accepted.
    until: Option<Date>,
    byday: Vec<(Option<i8>, Weekday)>,
    bymonthday: Vec<i8>,
    wkst: Weekday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Periods to walk before giving up. A daily rule from ten years ago is 3,650; this is a guard
/// against a malformed rule, never a limit a real calendar meets.
const MAX_PERIODS: i64 = 20_000;

fn weekday(code: &str) -> Option<Weekday> {
    Some(match code {
        "MO" => Weekday::Monday,
        "TU" => Weekday::Tuesday,
        "WE" => Weekday::Wednesday,
        "TH" => Weekday::Thursday,
        "FR" => Weekday::Friday,
        "SA" => Weekday::Saturday,
        "SU" => Weekday::Sunday,
        _ => return None,
    })
}

/// Read an `RRULE` value, or `None` if it uses anything outside the subset in the module doc.
pub fn parse(rule: &str) -> Option<Rule> {
    let mut r = Rule {
        freq: Freq::Daily,
        interval: 1,
        count: None,
        until: None,
        byday: Vec::new(),
        bymonthday: Vec::new(),
        wkst: Weekday::Monday,
    };
    let mut freq = None;
    for part in rule.trim().trim_start_matches("RRULE:").split(';').filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=')?;
        match k.trim().to_ascii_uppercase().as_str() {
            "FREQ" => {
                freq = Some(match v.trim().to_ascii_uppercase().as_str() {
                    "DAILY" => Freq::Daily,
                    "WEEKLY" => Freq::Weekly,
                    "MONTHLY" => Freq::Monthly,
                    "YEARLY" => Freq::Yearly,
                    _ => return None,
                })
            }
            "INTERVAL" => r.interval = v.trim().parse().ok().filter(|n: &i64| *n >= 1)?,
            "COUNT" => r.count = Some(v.trim().parse().ok()?),
            "UNTIL" => r.until = Some(crate::events::parse_dt(v, &[], None)?.date),
            "WKST" => r.wkst = weekday(v.trim())?,
            "BYDAY" => {
                for d in v.split(',') {
                    let d = d.trim();
                    let split = d.len().checked_sub(2)?;
                    let (ord, code) = d.split_at(split);
                    let ord = match ord {
                        "" => None,
                        o => {
                            Some(o.trim_start_matches('+').parse::<i8>().ok().filter(|n| *n != 0)?)
                        }
                    };
                    r.byday.push((ord, weekday(code)?));
                }
            }
            "BYMONTHDAY" => {
                for d in v.split(',') {
                    let n: i8 = d.trim().parse().ok()?;
                    if n == 0 || !(-31..=31).contains(&n) {
                        return None;
                    }
                    r.bymonthday.push(n);
                }
            }
            // `BYSETPOS`, `BYMONTH`, `BYWEEKNO`, `BYYEARDAY`, `BYHOUR`, … — outside the subset.
            _ => return None,
        }
    }
    r.freq = freq?;
    if r.count.is_some() && r.until.is_some() {
        // RFC 5545 forbids both; a rule that has both is malformed, and guessing which wins is a guess.
        return None;
    }
    let ordinals = r.byday.iter().any(|(o, _)| o.is_some());
    match r.freq {
        Freq::Daily | Freq::Weekly if ordinals => return None,
        Freq::Weekly if !r.bymonthday.is_empty() => return None,
        Freq::Yearly if !r.byday.is_empty() || !r.bymonthday.is_empty() => return None,
        Freq::Monthly if !r.byday.is_empty() && !r.bymonthday.is_empty() => return None,
        _ => {}
    }
    Some(r)
}

/// Every occurrence date from `start` up to and including `to`, in order, honouring `COUNT` and
/// `UNTIL`. The first is `start` itself when it matches the rule, as RFC 5545 requires of `DTSTART`.
/// Past dates are included — `COUNT` is counted from the series' start, so they cannot be skipped
/// before counting; the caller filters.
pub fn occurrences(start: Date, rule: &Rule, to: Date) -> Vec<Date> {
    let mut out = Vec::new();
    let mut n = 0u32;
    for k in 0..MAX_PERIODS {
        let Some((first, mut cands)) = period(start, rule, k) else { break };
        if first > to {
            break;
        }
        cands.sort();
        cands.dedup();
        for d in cands.into_iter().filter(|d| *d >= start) {
            if rule.until.is_some_and(|u| d > u) || d > to {
                return out;
            }
            n += 1;
            if rule.count.is_some_and(|c| n > c) {
                return out;
            }
            out.push(d);
        }
    }
    out
}

/// The `k`th period's first day and its candidate dates.
fn period(start: Date, rule: &Rule, k: i64) -> Option<(Date, Vec<Date>)> {
    let step = k.checked_mul(rule.interval)?;
    match rule.freq {
        Freq::Daily => {
            let d = start.checked_add(Duration::days(step))?;
            let keep = (rule.byday.is_empty() || rule.byday.iter().any(|(_, w)| *w == d.weekday()))
                && (rule.bymonthday.is_empty()
                    || rule.bymonthday.iter().any(|m| month_day(d, *m) == Some(d)));
            Some((d, if keep { vec![d] } else { vec![] }))
        }
        Freq::Weekly => {
            let back = days_from(rule.wkst, start.weekday());
            let week =
                start.checked_sub(Duration::days(back))?.checked_add(Duration::weeks(step))?;
            let days: Vec<Weekday> = if rule.byday.is_empty() {
                vec![start.weekday()]
            } else {
                rule.byday.iter().map(|(_, w)| *w).collect()
            };
            let cands = days
                .into_iter()
                .filter_map(|w| week.checked_add(Duration::days(days_from(rule.wkst, w))))
                .collect();
            Some((week, cands))
        }
        Freq::Monthly => {
            let m0 = i64::from(start.month() as u8) - 1 + step;
            let year = i32::try_from(i64::from(start.year()) + m0.div_euclid(12)).ok()?;
            let month = Month::try_from(u8::try_from(m0.rem_euclid(12) + 1).ok()?).ok()?;
            let first = Date::from_calendar_date(year, month, 1).ok()?;
            let mut cands = Vec::new();
            if !rule.bymonthday.is_empty() {
                cands.extend(rule.bymonthday.iter().filter_map(|m| month_day(first, *m)));
            } else if !rule.byday.is_empty() {
                for (ord, w) in &rule.byday {
                    cands.extend(weekdays_in_month(first, *w, *ord));
                }
            } else if let Ok(d) = Date::from_calendar_date(year, month, start.day()) {
                // A month without this day (the 31st in April) has no occurrence — RFC 5545 §3.3.10.
                cands.push(d);
            }
            Some((first, cands))
        }
        Freq::Yearly => {
            let year = i32::try_from(i64::from(start.year()) + step).ok()?;
            let first = Date::from_calendar_date(year, Month::January, 1).ok()?;
            // 29 February in a year without one has no occurrence.
            let cands = Date::from_calendar_date(year, start.month(), start.day())
                .ok()
                .into_iter()
                .collect();
            Some((first, cands))
        }
    }
}

/// Days from `from` forward to `to` within a week (0–6).
fn days_from(from: Weekday, to: Weekday) -> i64 {
    (i64::from(to.number_days_from_monday()) - i64::from(from.number_days_from_monday()))
        .rem_euclid(7)
}

/// The `n`th day of the month `in_month` falls in; negative counts from the end. `None` if absent.
fn month_day(in_month: Date, n: i8) -> Option<Date> {
    let (y, m) = (in_month.year(), in_month.month());
    let len = m.length(y) as i8;
    let day = if n > 0 { n } else { len + 1 + n };
    if day < 1 || day > len {
        return None;
    }
    Date::from_calendar_date(y, m, day as u8).ok()
}

/// Every `w` in the month starting `first`, or only the `ord`th (negative from the end).
fn weekdays_in_month(first: Date, w: Weekday, ord: Option<i8>) -> Vec<Date> {
    let len = i64::from(first.month().length(first.year()));
    let all: Vec<Date> = (0..len)
        .filter_map(|i| first.checked_add(Duration::days(i)))
        .filter(|d| d.weekday() == w)
        .collect();
    match ord {
        None => all,
        Some(n) if n > 0 => all.get(n as usize - 1).copied().into_iter().collect(),
        Some(n) => all
            .len()
            .checked_sub(n.unsigned_abs() as usize)
            .and_then(|i| all.get(i))
            .copied()
            .into_iter()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    fn exp(start: Date, rule: &str, to: Date) -> Vec<Date> {
        occurrences(start, &parse(rule).expect("rule in the subset"), to)
    }

    #[test]
    fn weekly_on_its_own_weekday() {
        // 2026-10-13 is a Tuesday.
        let got = exp(date!(2026 - 10 - 13), "FREQ=WEEKLY", date!(2026 - 11 - 03));
        assert_eq!(
            got,
            vec![
                date!(2026 - 10 - 13),
                date!(2026 - 10 - 20),
                date!(2026 - 10 - 27),
                date!(2026 - 11 - 03)
            ]
        );
    }

    #[test]
    fn every_weekday_skips_the_weekend() {
        let got =
            exp(date!(2026 - 10 - 09), "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", date!(2026 - 10 - 14));
        // Fri 9th, then Mon 12th – Wed 14th. No Saturday or Sunday.
        assert_eq!(
            got,
            vec![
                date!(2026 - 10 - 09),
                date!(2026 - 10 - 12),
                date!(2026 - 10 - 13),
                date!(2026 - 10 - 14)
            ]
        );
    }

    #[test]
    fn every_other_week_counts_weeks_from_the_series_start() {
        let got =
            exp(date!(2026 - 10 - 15), "FREQ=WEEKLY;INTERVAL=2;BYDAY=TH", date!(2026 - 11 - 30));
        assert_eq!(
            got,
            vec![
                date!(2026 - 10 - 15),
                date!(2026 - 10 - 29),
                date!(2026 - 11 - 12),
                date!(2026 - 11 - 26)
            ]
        );
    }

    #[test]
    fn count_is_counted_from_the_start_not_from_today() {
        let got = exp(date!(2026 - 01 - 05), "FREQ=WEEKLY;COUNT=3", date!(2026 - 12 - 31));
        assert_eq!(got, vec![date!(2026 - 01 - 05), date!(2026 - 01 - 12), date!(2026 - 01 - 19)]);
    }

    #[test]
    fn until_is_inclusive() {
        let got =
            exp(date!(2026 - 10 - 13), "FREQ=DAILY;UNTIL=20261015T235959Z", date!(2026 - 12 - 31));
        assert_eq!(got, vec![date!(2026 - 10 - 13), date!(2026 - 10 - 14), date!(2026 - 10 - 15)]);
    }

    #[test]
    fn first_monday_and_last_friday_of_the_month() {
        let got = exp(date!(2026 - 10 - 05), "FREQ=MONTHLY;BYDAY=1MO", date!(2026 - 12 - 31));
        assert_eq!(got, vec![date!(2026 - 10 - 05), date!(2026 - 11 - 02), date!(2026 - 12 - 07)]);
        let got = exp(date!(2026 - 10 - 30), "FREQ=MONTHLY;BYDAY=-1FR", date!(2026 - 12 - 31));
        assert_eq!(got, vec![date!(2026 - 10 - 30), date!(2026 - 11 - 27), date!(2026 - 12 - 25)]);
    }

    #[test]
    fn a_month_without_the_day_has_no_occurrence() {
        let got = exp(date!(2026 - 01 - 31), "FREQ=MONTHLY", date!(2026 - 05 - 31));
        assert_eq!(got, vec![date!(2026 - 01 - 31), date!(2026 - 03 - 31), date!(2026 - 05 - 31)]);
    }

    #[test]
    fn the_last_day_of_each_month() {
        let got = exp(date!(2026 - 01 - 31), "FREQ=MONTHLY;BYMONTHDAY=-1", date!(2026 - 03 - 31));
        assert_eq!(got, vec![date!(2026 - 01 - 31), date!(2026 - 02 - 28), date!(2026 - 03 - 31)]);
    }

    #[test]
    fn yearly_on_the_same_date() {
        let got = exp(date!(2026 - 03 - 10), "FREQ=YEARLY", date!(2028 - 12 - 31));
        assert_eq!(got, vec![date!(2026 - 03 - 10), date!(2027 - 03 - 10), date!(2028 - 03 - 10)]);
    }

    #[test]
    fn a_rule_outside_the_subset_is_refused_rather_than_half_read() {
        for rule in [
            "FREQ=HOURLY",
            "FREQ=MONTHLY;BYDAY=MO,TU;BYSETPOS=-1",
            "FREQ=YEARLY;BYMONTH=3;BYDAY=2SU",
            "FREQ=WEEKLY;BYDAY=1MO",
            "FREQ=WEEKLY;COUNT=3;UNTIL=20261231",
            "INTERVAL=2",
            "FREQ=WEEKLY;INTERVAL=0",
        ] {
            assert!(parse(rule).is_none(), "{rule} must be refused");
        }
    }
}
