//! A scheduling point: a calendar day, optionally narrowed to a wall-clock time.

use std::fmt;
use std::str::FromStr;
use time::macros::format_description;
use time::{Date, Time};

/// What `start` and `due` carry: a calendar day, plus an optional wall-clock
/// time. `2026-07-20` is an all-day item; `2026-07-20T14:30` is a 14:30 one.
///
/// Deliberately **naive** — no UTC offset. A 14:30 meeting is at 14:30 where you
/// are, and the file is the truth: pinning an offset would make the on-disk text
/// drift with travel and DST, and would read wrong in a plain editor. `created`/
/// `updated` are different in kind — they are *instants*, not wall-clock
/// intentions, so they stay `OffsetDateTime`/RFC 3339.
///
/// One type rather than a second `PropertyValue` variant: that enum derives `Ord`
/// from variant order first, so a `Date` variant sitting beside a `DateTime` one
/// would sort *every* all-day item before *every* timed item regardless of the
/// actual day. Here ordering is (day, then `None` before `Some(t)`), so an
/// all-day item simply leads its own day.
///
/// Minute granularity: seconds are accepted on parse (iCalendar emits them) and
/// truncated, so `Display` is always a form `FromStr` accepts unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Stamp {
    pub date: Date,
    pub time: Option<Time>,
}

impl Stamp {
    /// An all-day point.
    pub fn day(date: Date) -> Self {
        Stamp { date, time: None }
    }

    /// A point at a wall-clock time, truncated to the minute.
    pub fn at(date: Date, time: Time) -> Self {
        Stamp { date, time: Some(trunc(time)) }
    }

    /// True when a time is set. The calendar draws these as timed events and the
    /// all-day ones as day markers.
    pub fn has_time(&self) -> bool {
        self.time.is_some()
    }
}

impl fmt::Display for Stamp {
    /// `2026-07-20` or `2026-07-20T14:30` — exactly the two forms `FromStr`
    /// accepts. That equivalence is load-bearing: `PropertyValue::display()` is
    /// fed straight back into `apply_property` by the board's drag write-back, so
    /// a lossy rendering here would silently erase the time on every drag.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format explicitly rather than leaning on `Date`/`Time`'s own Display —
        // `Time` renders seconds and sub-seconds, which we never want on disk.
        let d = self
            .date
            .format(&format_description!("[year]-[month]-[day]"))
            .map_err(|_| fmt::Error)?;
        match self.time {
            Some(t) => {
                let t =
                    t.format(&format_description!("[hour]:[minute]")).map_err(|_| fmt::Error)?;
                write!(f, "{d}T{t}")
            }
            None => write!(f, "{d}"),
        }
    }
}

impl FromStr for Stamp {
    type Err = String;

    /// Accepts `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM`, and — because hand-editing a
    /// note in Vim is a first-class way to use this vault — the `HH:MM:SS` and
    /// space-separated spellings. Those normalize to the two `Display` forms.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (d, t) = match s.split_once(['T', ' ']) {
            Some((d, t)) => (d, Some(t.trim())),
            None => (s, None),
        };
        let date = Date::parse(d, &format_description!("[year]-[month]-[day]"))
            .map_err(|e| format!("{e}"))?;
        let time = match t {
            None => None,
            Some(t) => Some(parse_time(t)?),
        };
        Ok(Stamp { date, time })
    }
}

fn trunc(t: Time) -> Time {
    Time::from_hms(t.hour(), t.minute(), 0).unwrap_or(t)
}

fn parse_time(s: &str) -> Result<Time, String> {
    Time::parse(s, &format_description!("[hour]:[minute]"))
        .or_else(|_| Time::parse(s, &format_description!("[hour]:[minute]:[second]")))
        .map(trunc)
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, time};

    #[test]
    fn an_all_day_stamp_renders_and_parses_as_a_bare_date() {
        let s = Stamp::day(date!(2026 - 07 - 20));
        assert_eq!(s.to_string(), "2026-07-20");
        assert_eq!("2026-07-20".parse::<Stamp>().unwrap(), s);
        assert!(!s.has_time());
    }

    #[test]
    fn a_timed_stamp_renders_and_parses_with_its_time() {
        let s = Stamp::at(date!(2026 - 07 - 20), time!(14:30));
        assert_eq!(s.to_string(), "2026-07-20T14:30");
        assert_eq!("2026-07-20T14:30".parse::<Stamp>().unwrap(), s);
        assert!(s.has_time());
    }

    #[test]
    fn display_is_always_reparseable_the_whole_round_trip_rests_on_this() {
        // `PropertyValue::display()` is echoed back into `apply_property`, so any
        // Display form that FromStr rejects (or reads differently) is data loss.
        for s in [
            Stamp::day(date!(2026 - 01 - 01)),
            Stamp::at(date!(2026 - 12 - 31), time!(00:00)),
            Stamp::at(date!(2026 - 07 - 20), time!(09:05)),
            Stamp::at(date!(2026 - 07 - 20), time!(23:59)),
        ] {
            assert_eq!(s.to_string().parse::<Stamp>().unwrap(), s, "{s} did not round-trip");
        }
    }

    #[test]
    fn hand_written_spellings_normalize_to_the_canonical_form() {
        let want = Stamp::at(date!(2026 - 07 - 20), time!(14:30));
        for raw in
            ["2026-07-20T14:30", "2026-07-20 14:30", "2026-07-20T14:30:00", " 2026-07-20T14:30 "]
        {
            assert_eq!(raw.parse::<Stamp>().unwrap(), want, "failed on {raw:?}");
        }
        // Seconds are truncated, not preserved — minute granularity is the rule.
        assert_eq!("2026-07-20T14:30:45".parse::<Stamp>().unwrap().to_string(), "2026-07-20T14:30");
    }

    #[test]
    fn garbage_is_rejected_rather_than_silently_defaulted() {
        for raw in
            ["", "not-a-date", "2026-13-01", "2026-07-20T25:00", "2026-07-20T14", "20/07/2026"]
        {
            assert!(raw.parse::<Stamp>().is_err(), "{raw:?} should not parse");
        }
    }

    #[test]
    fn an_all_day_item_sorts_before_a_timed_one_on_the_same_day() {
        // The reason Stamp is one type instead of two PropertyValue variants: a
        // mixed vault must still sort by real chronology.
        let mut v = vec![
            Stamp::at(date!(2026 - 07 - 20), time!(14:30)),
            Stamp::day(date!(2026 - 07 - 21)),
            Stamp::day(date!(2026 - 07 - 20)),
            Stamp::at(date!(2026 - 07 - 20), time!(09:00)),
        ];
        v.sort();
        assert_eq!(
            v.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            ["2026-07-20", "2026-07-20T09:00", "2026-07-20T14:30", "2026-07-21"]
        );
    }
}
