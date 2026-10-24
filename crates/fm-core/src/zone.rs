//! The person's own clock, and moving a time written on another onto it — `decisions.md` 2026-10-23,
//! *a time written in another zone is moved onto yours*.
//!
//! A note's [`Stamp`] stays naive and means **your** clock: *"14:30 means 14:30 where you are"*
//! (`decisions.md`, 2026-07-15). What this module adds is the step before writing one: a calendar
//! event written `TZID=Europe/Berlin:20261027T100000` is 18:00 on a Tokyo clock, and is stored as
//! 18:00. The conversion is the zone database's (`jiff`), on the event's own day, so a daylight-saving
//! change between now and then is counted — the reason a fixed offset was never enough.
//!
//! **Your zone** is the person's setting (a name such as `Asia/Tokyo`, or an offset such as
//! `+09:00`), and unset it is the computer's own. `jiff` reads that safely in a threaded process
//! (`/etc/localtime`, the Windows registry, Android's tzdata), which `time`'s `local-offset` cannot —
//! the reason [`crate::events`] once had to be told an offset.

use fm_model::Stamp;
use jiff::tz::{Offset, TimeZone};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// A clock: a zone from the database, or a fixed offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone(TimeZone);

impl Zone {
    /// `Asia/Tokyo`, `+09:00`, `-05:30`, `Z` or `UTC`. Anything else is refused rather than
    /// guessed: a wrong zone moves every meeting by hours, silently.
    pub fn parse(raw: &str) -> Result<Zone, String> {
        let t = raw.trim();
        let bad = || format!("'{t}' is not a time zone like Asia/Tokyo, or an offset like +09:00");
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

    /// A fixed offset, in seconds east of UTC — a vault's clock read from its first commit.
    pub fn fixed(seconds: i32) -> Option<Zone> {
        Offset::from_seconds(seconds).ok().map(|o| Zone(TimeZone::fixed(o)))
    }

    /// How it is named to a person: `Asia/Tokyo`, or `+09:00` for a fixed offset — or, for a zone
    /// with no name (a hard-linked `/etc/localtime`), its offset **now**, not in 1970 (F22).
    pub fn label(&self) -> String {
        if let Some(n) = self.name() {
            return n.to_string();
        }
        offset_label(self.0.to_offset(jiff::Timestamp::now()).seconds())
    }

    /// The offset in force now, in seconds east of UTC.
    pub fn offset_now(&self) -> i32 {
        self.0.to_offset(jiff::Timestamp::now()).seconds()
    }

    /// The computer's own zone, when it can be read.
    pub fn system() -> Option<Zone> {
        TimeZone::try_system().ok().map(Zone)
    }

    /// The database name, when this is one: `Asia/Tokyo`.
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

    /// **The moment a wall time on this clock is**, in Unix seconds — a time in a daylight-saving
    /// gap moves later, one in the repeated hour takes the earlier (`jiff`'s compatible rule).
    pub fn instant(&self, day: Date, at: Time) -> Option<i64> {
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
        Some(dt.to_zoned(self.0.clone()).ok()?.timestamp().as_second())
    }

    /// The moment `day` begins on this clock, in Unix seconds.
    pub fn start_of_day(&self, day: Date) -> Option<i64> {
        self.instant(day, Time::MIDNIGHT)
    }

    /// [`Zone::start_of_day`] for a day written `YYYY-MM-DD`; `None` for anything else.
    pub fn start_of(&self, ymd: &str) -> Option<i64> {
        let d = Date::parse(ymd.get(..10)?, &time::format_description::well_known::Iso8601::DATE)
            .ok()?;
        self.start_of_day(d)
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

/// `+09:00`, `-03:30`, `+00:00`.
pub fn offset_label(seconds: i32) -> String {
    let (sign, a) = if seconds < 0 { ('-', -seconds) } else { ('+', seconds) };
    format!("{sign}{:02}:{:02}", a / 3600, a % 3600 / 60)
}

/// **The app's clock** (`decisions.md#data`, 2026-10-24, *one app clock*): the zone every part of the
/// app reads — the store's seam, the calendar and mail readers, reminders, the UI and the assistant.
/// Either the person's **choice** (Settings → Clock) or **Automatic**, the zone the system reports to
/// this process. Cheap to clone; every clone sees a change of choice at once.
///
/// **It never answers UTC as a stand-in.** When nothing is chosen and the system's zone cannot be
/// read, [`Clock::zone`] is `None`, and what needs a zone says so instead of moving every time by hours.
#[derive(Clone, Debug, Default)]
pub struct Clock(std::sync::Arc<std::sync::RwLock<Option<Zone>>>);

impl Clock {
    /// Automatic: the system's zone.
    pub fn automatic() -> Clock {
        Clock::default()
    }

    /// A chosen zone — the person's, or a test's.
    pub fn chosen(zone: Zone) -> Clock {
        let c = Clock::default();
        c.set(Some(zone));
        c
    }

    /// Choose a zone, or `None` for Automatic. Every clone follows.
    pub fn set(&self, choice: Option<Zone>) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = choice;
    }

    /// The zone chosen, if one is.
    pub fn choice(&self) -> Option<Zone> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The zone in force: the choice, else the system's; `None` when neither can be had.
    pub fn zone(&self) -> Option<Zone> {
        self.choice().or_else(Zone::system)
    }

    /// `"chosen"` or `"automatic"`.
    pub fn source(&self) -> &'static str {
        if self.choice().is_some() {
            "chosen"
        } else {
            "automatic"
        }
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
    fn a_berlin_time_is_read_on_a_tokyo_clock_on_its_own_day() {
        let (tokyo, berlin) =
            (Zone::parse("Asia/Tokyo").unwrap(), Zone::named("Europe/Berlin").unwrap());
        // Berlin leaves summer time on 25 October 2026.
        let before = Stamp::at(date!(2026 - 10 - 23), time!(10:00));
        let after = Stamp::at(date!(2026 - 10 - 27), time!(10:00));
        assert_eq!(tokyo.onto(before, &berlin), Stamp::at(date!(2026 - 10 - 23), time!(17:00)));
        assert_eq!(tokyo.onto(after, &berlin), Stamp::at(date!(2026 - 10 - 27), time!(18:00)));
        // A day alone is a day on any clock; the same clock is no conversion.
        assert_eq!(
            tokyo.onto(Stamp::day(date!(2026 - 10 - 27)), &berlin),
            Stamp::day(date!(2026 - 10 - 27))
        );
        assert_eq!(tokyo.onto(after, &tokyo), after);
    }

    #[test]
    fn a_name_or_an_offset_and_nothing_else() {
        assert!(Zone::parse("Europe/Rome").is_ok());
        assert!(Zone::parse("+09:00").is_ok());
        assert!(Zone::parse("-05:30").is_ok());
        assert!(Zone::parse("Z").is_ok());
        // Nothing that could be one of the database's aliases (`Japan`, `Egypt`): those exist on
        // some systems and not others (`fm-app`'s dispatch test, 2026-10-23).
        for bad in ["", "8", "+25:00", "Berlin time", "Europe/Atlantis", "+08:61"] {
            assert!(Zone::parse(bad).is_err(), "{bad} must be refused");
        }
        assert!(Zone::named("W. Europe Standard Time").is_none(), "a Windows name is not guessed");
    }

    #[test]
    fn a_utc_time_takes_the_offset_in_force_on_its_day() {
        let rome = Zone::parse("Europe/Rome").unwrap();
        assert_eq!(rome.from_utc(datetime!(2026-10-23 08:00)), datetime!(2026-10-23 10:00));
        assert_eq!(rome.from_utc(datetime!(2026-10-27 08:00)), datetime!(2026-10-27 09:00));
        let fixed = Zone::parse("+09:00").unwrap();
        assert_eq!(fixed.from_utc(datetime!(2026-10-27 20:00)), datetime!(2026-10-28 05:00));
    }
}
