//! The person's own clock, and moving a time written on another onto it — `decisions.md` 2026-10-23,
//! *a time written in another zone is moved onto yours*.
//!
//! A note's [`Stamp`] stays naive and means **your** clock: *"14:30 means 14:30 where you are"*
//! (`decisions.md`, 2026-07-15). What this module adds is the step before writing one: a calendar
//! event written `TZID=Europe/Paris:20261027T100000` is 17:00 on a Singapore clock, and is stored as
//! 17:00. The conversion is the zone database's (`jiff`), on the event's own day, so a daylight-saving
//! change between now and then is counted — the reason a fixed offset was never enough.
//!
//! **Your zone** is the person's setting (a name such as `Asia/Singapore`, or an offset such as
//! `+08:00`), and unset it is the computer's own. `jiff` reads that safely in a threaded process
//! (`/etc/localtime`, the Windows registry, Android's tzdata), which `time`'s `local-offset` cannot —
//! the reason [`crate::events`] once had to be told an offset.

use fm_model::Stamp;
use jiff::tz::{Offset, TimeZone};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// A clock: a zone from the database, or a fixed offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone(TimeZone);

impl Zone {
    /// `Asia/Singapore`, `+08:00`, `-05:30`, `Z` or `UTC`. Anything else is refused rather than
    /// guessed: a wrong zone moves every meeting by hours, silently.
    pub fn parse(raw: &str) -> Result<Zone, String> {
        let t = raw.trim();
        let bad =
            || format!("'{t}' is not a time zone like Asia/Singapore, or an offset like +08:00");
        if t.is_empty() {
            return Err(bad());
        }
        if t.eq_ignore_ascii_case("z") || t.eq_ignore_ascii_case("utc") {
            return Ok(Zone(TimeZone::UTC));
        }
        if t.starts_with(['+', '-']) {
            let sign = if t.starts_with('-') { -1 } else { 1 };
            let (h, m) = t[1..].split_once(':').unwrap_or((&t[1..], "0"));
            let (h, m): (i32, i32) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
            if !(0..=14).contains(&h) || !(0..60).contains(&m) {
                return Err(bad());
            }
            let off = Offset::from_seconds(sign * (h * 3600 + m * 60)).map_err(|_| bad())?;
            return Ok(Zone(TimeZone::fixed(off)));
        }
        TimeZone::get(t).map(Zone).map_err(|_| bad())
    }

    /// A zone by its database name only (an iCalendar `TZID`). `None` for a name the database does
    /// not know — Outlook's Windows names, a custom `VTIMEZONE` — which is then carried as written.
    pub fn named(name: &str) -> Option<Zone> {
        TimeZone::get(name.trim().trim_matches('"')).ok().map(Zone)
    }

    /// The computer's own zone, when it can be read.
    pub fn system() -> Option<Zone> {
        TimeZone::try_system().ok().map(Zone)
    }

    /// The database name, when this is one: `Asia/Singapore`.
    pub fn name(&self) -> Option<&str> {
        self.0.iana_name()
    }

    /// The offset in force on this clock at `unix` seconds.
    pub fn offset_at(&self, unix: i64) -> UtcOffset {
        let ts = jiff::Timestamp::from_second(unix).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        UtcOffset::from_whole_seconds(self.0.to_offset(ts).seconds()).unwrap_or(UtcOffset::UTC)
    }

    /// A UTC date and time, as read on this clock.
    pub fn from_utc(&self, utc: PrimitiveDateTime) -> PrimitiveDateTime {
        let unix = utc.assume_utc().unix_timestamp();
        let local = utc.assume_utc().to_offset(self.offset_at(unix));
        PrimitiveDateTime::new(local.date(), local.time())
    }

    /// Today, on this clock.
    pub fn today(&self) -> Date {
        let now = OffsetDateTime::now_utc();
        now.to_offset(self.offset_at(now.unix_timestamp())).date()
    }

    /// `stamp`, written on `from`'s clock, read on this one. A day with no time is a day on any
    /// clock and is returned as it is; so is a stamp the database cannot place.
    pub fn onto(&self, stamp: Stamp, from: &Zone) -> Stamp {
        let Some(t) = stamp.time else { return stamp };
        if from == self {
            return stamp;
        }
        convert(stamp.date, t, &from.0, &self.0).map(|(d, t)| Stamp::at(d, t)).unwrap_or(stamp)
    }
}

fn convert(day: Date, at: Time, from: &TimeZone, to: &TimeZone) -> Option<(Date, Time)> {
    let dt = jiff::civil::DateTime::new(
        i16::try_from(day.year()).ok()?,
        u8::from(day.month()) as i8,
        day.day() as i8,
        at.hour() as i8,
        at.minute() as i8,
        at.second() as i8,
        0,
    )
    .ok()?;
    let there = dt.to_zoned(from.clone()).ok()?.with_time_zone(to.clone());
    let (d, t) = (there.date(), there.time());
    Some((
        Date::from_calendar_date(
            d.year().into(),
            Month::try_from(d.month() as u8).ok()?,
            d.day() as u8,
        )
        .ok()?,
        Time::from_hms(t.hour() as u8, t.minute() as u8, t.second() as u8).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, datetime, time};

    #[test]
    fn a_paris_time_is_read_on_a_singapore_clock_on_its_own_day() {
        let (sg, paris) =
            (Zone::parse("Asia/Singapore").unwrap(), Zone::named("Europe/Paris").unwrap());
        // Paris leaves summer time on 25 October 2026.
        let before = Stamp::at(date!(2026 - 10 - 23), time!(10:00));
        let after = Stamp::at(date!(2026 - 10 - 27), time!(10:00));
        assert_eq!(sg.onto(before, &paris), Stamp::at(date!(2026 - 10 - 23), time!(16:00)));
        assert_eq!(sg.onto(after, &paris), Stamp::at(date!(2026 - 10 - 27), time!(17:00)));
        // A day alone is a day on any clock; the same clock is no conversion.
        assert_eq!(
            sg.onto(Stamp::day(date!(2026 - 10 - 27)), &paris),
            Stamp::day(date!(2026 - 10 - 27))
        );
        assert_eq!(sg.onto(after, &sg), after);
    }

    #[test]
    fn a_name_or_an_offset_and_nothing_else() {
        assert!(Zone::parse("Europe/Rome").is_ok());
        assert!(Zone::parse("+08:00").is_ok());
        assert!(Zone::parse("-05:30").is_ok());
        assert!(Zone::parse("Z").is_ok());
        for bad in ["", "8", "+25:00", "Paris", "SGT", "+08:61"] {
            assert!(Zone::parse(bad).is_err(), "{bad} must be refused");
        }
        assert!(Zone::named("W. Europe Standard Time").is_none(), "a Windows name is not guessed");
    }

    #[test]
    fn a_utc_time_takes_the_offset_in_force_on_its_day() {
        let rome = Zone::parse("Europe/Rome").unwrap();
        assert_eq!(rome.from_utc(datetime!(2026-10-23 08:00)), datetime!(2026-10-23 10:00));
        assert_eq!(rome.from_utc(datetime!(2026-10-27 08:00)), datetime!(2026-10-27 09:00));
        let fixed = Zone::parse("+08:00").unwrap();
        assert_eq!(fixed.from_utc(datetime!(2026-10-27 20:00)), datetime!(2026-10-28 04:00));
    }
}
