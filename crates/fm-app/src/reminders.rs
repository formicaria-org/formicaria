//! What is coming up, and when to say so: the moments a reminder is due.
//!
//! `decisions.md#ui`, 2026-10-13, *a reminder is the note's own date, said at its time*. A note
//! that is not finished and carries a `start` (an event) or a `due` (a deadline) is reminded of:
//!
//! - **at its time**, when the date carries one;
//! - **at nine in the morning of its day**, when it carries a day and no time;
//! - **earlier by `remind` minutes**, when the note says so; and **not at all** when it says
//!   `remind: off`.
//!
//! This answers moments in the wall-clock time the notes are written in. Turning a moment into a
//! notification belongs to whatever is showing the app — a phone's alarm, a browser's own
//! notification — and so does the time zone, which is the device's. Nothing is stored: the
//! answer is worked out from the notes each time, so a date changed on another device is right
//! at the next ask.

use crate::commands::unfinished;
use fm_core::{Store, StoreError};
use fm_model::PropertyValue;
use fm_query::Query;
use serde::Serialize;
use time::{format_description::well_known::Iso8601, Date, Duration, PrimitiveDateTime, Time};

/// The property that moves a note's reminder earlier (minutes) or turns it off (`off`).
pub const REMIND: &str = "remind";
/// The hour a day with no time is reminded of.
const MORNING: u8 = 9;
/// No more than this many are answered: a device schedules a handful, not a year.
const MOST: usize = 100;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    /// `YYYY-MM-DDTHH:MM`, in the notes' own wall-clock time.
    pub at: String,
    /// `start` for something that begins then, `due` for something to be finished by then.
    pub what: String,
    /// The note's own moment, when the reminder is earlier than it: `HH:MM`, or its day.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub when: String,
    /// The note's date is a day with no time: it is reminded of in the morning.
    pub all_day: bool,
}

/// The reminders that fall on `from` or in the `days` after it, soonest first.
pub fn reminders(store: &dyn Store, from: &str, days: i64) -> Result<Vec<Reminder>, StoreError> {
    let first = Date::parse(from, &Iso8601::DATE).map_err(|_| {
        StoreError::Parse(format!("from must be a day like 2026-07-20, not '{from}'"))
    })?;
    let last = first + Duration::days(days.clamp(0, 60));
    let q = Query { filter: unfinished(), ..Default::default() };
    let mut out: Vec<(PrimitiveDateTime, Reminder)> = Vec::new();
    for o in store.query(&q)?.rows {
        let early = match o.get(REMIND) {
            PropertyValue::Text(t) if matches!(t.trim(), "off" | "no" | "none") => continue,
            PropertyValue::Int(n) if n > 0 => n.min(7 * 24 * 60),
            _ => 0,
        };
        for (what, stamp) in [("start", o.start), ("due", o.due)] {
            let Some(s) = stamp else { continue };
            let its = PrimitiveDateTime::new(
                s.date,
                s.time.unwrap_or(Time::from_hms(MORNING, 0, 0).expect("nine o'clock exists")),
            );
            let at = its - Duration::minutes(early);
            if at.date() < first || at.date() > last {
                continue;
            }
            // One moment, one reminder: a note that starts and is due at the same time.
            if out.iter().any(|(t, r)| *t == at && r.id == o.id.to_string()) {
                continue;
            }
            let clock = |t: PrimitiveDateTime| format!("{:02}:{:02}", t.hour(), t.minute());
            out.push((
                at,
                Reminder {
                    id: o.id.to_string(),
                    title: o.title.clone().unwrap_or_default(),
                    at: format!(
                        "{:04}-{:02}-{:02}T{}",
                        at.year(),
                        at.month() as u8,
                        at.day(),
                        clock(at)
                    ),
                    what: what.to_string(),
                    when: if early == 0 {
                        String::new()
                    } else if s.time.is_some() {
                        clock(its)
                    } else {
                        "that day".to_string()
                    },
                    all_day: s.time.is_none(),
                },
            ));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.title.cmp(&b.1.title)));
    Ok(out.into_iter().take(MOST).map(|(_, r)| r).collect())
}
