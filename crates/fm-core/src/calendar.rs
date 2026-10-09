//! Event records → notes: which to create, which a newer `SEQUENCE` moves, which are cancelled.
//!
//! # The half with no model in it
//!
//! `decisions.md`, *mail and your own calendars become notes on their own* (2026-10-09): an invite is
//! structured, so it goes in deterministically. This module is that rule as a pure function. It takes
//! the records [`crate::events`] parsed and the notes a vault already holds, and returns what to
//! write. It touches no store, no disk and no network — the caller owns all three, the same split
//! `import::convert` / `commands::write_import` make.
//!
//! # What may change on a note that already exists, and what may not
//!
//! Only the fields the publisher owns, and only when the publisher says they changed:
//!
//! - **`start`, `due` and `location` are rewritten on a `SEQUENCE` bump**, and at no other time. A
//!   publisher bumps `SEQUENCE` when an invite is rescheduled; that is the one signal that the times
//!   on a note somebody may have annotated are stale (Track S #2, `plan.md`).
//! - **`cancelled` marks a note and never deletes one.** The person may have written in it.
//! - **The title and the body are never touched.** A person renames a meeting note to what it means
//!   to them, and writes their notes under it. Neither belongs to the feed.
//!
//! # Repeating meetings
//!
//! A series is expanded into **one note per occurrence** over the next [`HORIZON_DAYS`], each keyed
//! `uid#<date>`; every hourly read extends the window, so a weekly meeting is always there for the
//! coming weeks without a year of notes appearing at once. An occurrence the organiser moved
//! (`RECURRENCE-ID`) replaces the one it overrides, an `EXDATE` removes one, and when a series stops
//! producing a date it used to — rescheduled to another weekday, shortened, a week dropped — that
//! occurrence's note is **marked cancelled, never deleted**. Rules outside [`crate::recur`]'s subset
//! are not guessed at: their first date is shown and they are counted.
//!
//! # What is not created
//!
//! An event with no `UID` (a re-read could not find it again, so every run would add a copy), an
//! event with no date, one already over, and one already cancelled. Each is **counted**, so the
//! surface can say how many it left out and why rather than quietly showing fewer meetings than the
//! calendar holds — the same discipline as [`EventRecord::is_dated`].

use std::collections::{HashMap, HashSet};

use fm_model::{Kind, Object, PropertyValue, Stamp};
use serde::Serialize;
use time::Date;

use crate::events::EventRecord;

/// The event's identity: its `UID`, plus `RECURRENCE-ID` for an overridden occurrence.
pub const ICS_UID: &str = "ics_uid";
/// The `SEQUENCE` this note's times were last written from.
pub const ICS_SEQ: &str = "ics_seq";
/// Which calendar it came from — the label the person gave it.
pub const ICS_SOURCE: &str = "ics_source";
/// `cancelled`, once the publisher says so. Never cleared by a later read: un-cancelling is rare, and
/// a meeting that silently reappears is worse than one the person re-opens by hand.
pub const ICS_STATUS: &str = "ics_status";
/// The zone the publisher wrote the time in, when it named one. Shown, never resolved.
pub const ICS_TIME_ZONE: &str = "ics_time_zone";
pub const LOCATION: &str = "location";
pub const ORGANIZER: &str = "organizer";

/// The tag every calendar note carries, beside its source's own.
pub const MEETING_TAG: &str = "meeting";
/// The tag a cancelled meeting gains, so the board and the agenda can show it without a new field.
pub const CANCELLED_TAG: &str = "cancelled";

/// What a read did, in counts the surface repeats back.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub created: usize,
    /// A `SEQUENCE` bump that changed the times or the place.
    pub moved: usize,
    pub cancelled: usize,
    pub unchanged: usize,
    /// Over before this read, so not created.
    pub past: usize,
    pub undated: usize,
    /// No `UID`, so a later read could not find it again.
    #[serde(rename = "noId")]
    pub no_id: usize,
    /// Repeating events whose rule is outside [`crate::recur`]'s subset, so only their first date is
    /// shown. Counted, because a series quietly reduced to one meeting is the failure to avoid.
    pub repeating: usize,
    /// Repeating events that were read and expanded.
    pub series: usize,
}

/// What to write: new notes, and existing notes with their new fields already applied.
#[derive(Debug, Default)]
pub struct Plan {
    pub create: Vec<Object>,
    pub update: Vec<Object>,
    pub report: Report,
}

/// How far ahead a repeating meeting is laid out as notes. Eight-and-a-bit weeks: the agenda's month
/// view is always full, and an hourly read keeps the window moving.
pub const HORIZON_DAYS: i64 = 60;

/// The identity a record is matched on, or `None` when it has no `UID`. One occurrence of a series is
/// `uid#<day it was due>` — the same key whether it came from expanding the rule or from an override
/// (`RECURRENCE-ID`), which is what lets an override replace the occurrence it moves.
pub fn key_of(r: &EventRecord) -> Option<String> {
    let uid = r.uid.as_deref().map(str::trim).filter(|u| !u.is_empty())?;
    Some(match r.recurrence_id {
        Some(rid) => occurrence_key(uid, rid.date),
        None => uid.to_string(),
    })
}

fn occurrence_key(uid: &str, day: Date) -> String {
    format!("{uid}#{}", Stamp::day(day))
}

/// Replace each readable series by its occurrences from `today` to the horizon. Returns the records to
/// reconcile and, per expanded series, the keys it produced — what [`reconcile`] needs to notice a
/// date the series no longer has.
fn expand(
    records: &[EventRecord],
    today: Date,
    report: &mut Report,
) -> (Vec<EventRecord>, HashMap<String, HashSet<String>>) {
    let to = today.checked_add(time::Duration::days(HORIZON_DAYS)).unwrap_or(today);
    let overrides: HashSet<String> =
        records.iter().filter(|r| r.recurrence_id.is_some()).filter_map(key_of).collect();
    let mut out = Vec::new();
    let mut produced: HashMap<String, HashSet<String>> = HashMap::new();
    for r in records {
        let (Some(text), Some(start), Some(uid)) = (r.rrule.as_deref(), r.start, r.uid.as_deref())
        else {
            out.push(r.clone());
            continue;
        };
        let Some(rule) = crate::recur::parse(text) else {
            report.repeating += 1;
            out.push(r.clone());
            continue;
        };
        report.series += 1;
        let keys = produced.entry(uid.trim().to_string()).or_default();
        // The overrides of this series are part of what it now holds, wherever they moved to.
        keys.extend(
            overrides.iter().filter(|k| k.starts_with(&format!("{}#", uid.trim()))).cloned(),
        );
        let excluded: HashSet<Date> = r.exdates.iter().map(|s| s.date).collect();
        for day in crate::recur::occurrences(start.date, &rule, to) {
            let key = occurrence_key(uid.trim(), day);
            if day < today || excluded.contains(&day) || overrides.contains(&key) {
                continue;
            }
            let shift = day - start.date;
            let mut occ = r.clone();
            occ.start = Some(Stamp { date: day, time: start.time });
            occ.end = r.end.map(|e| Stamp { date: e.date + shift, time: e.time });
            occ.recurrence_id = Some(Stamp::day(day));
            occ.rrule = None;
            occ.exdates.clear();
            keys.insert(key);
            out.push(occ);
        }
    }
    (out, produced)
}

/// Index notes by their [`ICS_UID`]. Notes without one are not calendar notes and are ignored.
pub fn index<'a>(notes: impl IntoIterator<Item = &'a Object>) -> HashMap<String, Object> {
    notes
        .into_iter()
        .filter_map(|o| match o.get(ICS_UID) {
            PropertyValue::Text(k) if !k.is_empty() => Some((k, o.clone())),
            _ => None,
        })
        .collect()
}

/// Decide what a read of one calendar should write.
///
/// `source` is the calendar's label, stored on each note and added as a tag. `today` is the caller's
/// day, in the same wall clock the stamps are in; an event whose last day is before it is not created.
pub fn reconcile(
    records: &[EventRecord],
    existing: &HashMap<String, Object>,
    source: &str,
    today: Date,
) -> Plan {
    let mut plan = Plan::default();
    let mut seen = HashSet::new();
    let (records, produced) = expand(records, today, &mut plan.report);
    for r in &records {
        let Some(key) = key_of(r) else {
            plan.report.no_id += 1;
            continue;
        };
        // A feed that lists one event twice is the publisher's mistake; the first copy wins, and the
        // second must not write a second note in the same run.
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(start) = r.start else {
            plan.report.undated += 1;
            continue;
        };
        let (start, due) = span(start, r.end);
        let cancelled = r.status.as_deref() == Some("cancelled");

        match existing.get(&key) {
            Some(note) => {
                let mut note = note.clone();
                let mut changed = false;
                let stored = match note.get(ICS_SEQ) {
                    PropertyValue::Int(n) => n,
                    _ => 0,
                };
                if i64::from(r.seq.unwrap_or(0)) > stored {
                    let before = (note.start, note.due, note.get(LOCATION));
                    note.start = start;
                    note.due = Some(due);
                    set_text(&mut note, LOCATION, r.location.as_deref());
                    set_text(&mut note, ICS_TIME_ZONE, r.tzid.as_deref());
                    note.extra
                        .insert(ICS_SEQ.into(), PropertyValue::Int(r.seq.unwrap_or(0).into()));
                    if before != (note.start, note.due, note.get(LOCATION)) {
                        plan.report.moved += 1;
                    }
                    changed = true;
                }
                let already =
                    matches!(note.get(ICS_STATUS), PropertyValue::Text(s) if s == "cancelled");
                if cancelled && !already {
                    note.extra.insert(ICS_STATUS.into(), PropertyValue::Text("cancelled".into()));
                    if !note.tags.iter().any(|t| t == CANCELLED_TAG) {
                        note.tags.push(CANCELLED_TAG.into());
                    }
                    plan.report.cancelled += 1;
                    changed = true;
                }
                if changed {
                    plan.update.push(note);
                } else {
                    plan.report.unchanged += 1;
                }
            }
            None => {
                // A cancelled event nobody has a note for is simply not on the calendar any more.
                if cancelled {
                    continue;
                }
                if due.date < today {
                    plan.report.past += 1;
                    continue;
                }
                let mut note = Object::new(Kind::Note, "");
                note.title = Some(r.title.clone());
                note.start = start;
                note.due = Some(due);
                note.tags = vec![MEETING_TAG.to_string()];
                let tag = source.trim();
                if !tag.is_empty() && tag != MEETING_TAG {
                    note.tags.push(tag.to_string());
                }
                note.extra.insert(ICS_UID.into(), PropertyValue::Text(key));
                note.extra.insert(ICS_SEQ.into(), PropertyValue::Int(r.seq.unwrap_or(0).into()));
                note.extra.insert(ICS_SOURCE.into(), PropertyValue::Text(source.to_string()));
                set_text(&mut note, LOCATION, r.location.as_deref());
                set_text(&mut note, ORGANIZER, r.organizer.as_deref());
                set_text(&mut note, ICS_TIME_ZONE, r.tzid.as_deref());
                plan.create.push(note);
                plan.report.created += 1;
            }
        }
    }
    // **An occurrence the series no longer has is marked, never deleted.** Only for a series read in
    // full this time (its rule was expanded), only within the window that expansion covered, and only
    // for a date still ahead — so a feed that briefly omits a series, or a meeting already held, is
    // never touched.
    let to = today.checked_add(time::Duration::days(HORIZON_DAYS)).unwrap_or(today);
    for (key, note) in existing {
        let Some((uid, day)) = key.rsplit_once('#') else { continue };
        let Some(keys) = produced.get(uid) else { continue };
        let Ok(day) = day.parse::<Stamp>() else { continue };
        let already = matches!(note.get(ICS_STATUS), PropertyValue::Text(s) if s == "cancelled");
        if keys.contains(key) || already || day.date < today || day.date > to {
            continue;
        }
        let mut note = note.clone();
        note.extra.insert(ICS_STATUS.into(), PropertyValue::Text("cancelled".into()));
        if !note.tags.iter().any(|t| t == CANCELLED_TAG) {
            note.tags.push(CANCELLED_TAG.into());
        }
        plan.report.cancelled += 1;
        plan.update.push(note);
    }
    plan
}

/// `DTSTART`/`DTEND` → the note's `start`/`due`.
///
/// With an end, the meeting is a bar from start to end. Without one it is a single marker on `due` —
/// the agenda's rule — so `start` stays empty rather than duplicating `due`.
///
/// **An all-day `DTEND` is exclusive** (RFC 5545 §3.6.1): a one-day event on the 23rd ends on the 24th.
/// Taken literally, every all-day event would spill into the next day, so a timeless end after a
/// timeless start is pulled back one day.
fn span(start: Stamp, end: Option<Stamp>) -> (Option<Stamp>, Stamp) {
    let Some(mut end) = end else {
        return (None, start);
    };
    if !start.has_time() && !end.has_time() && end.date > start.date {
        end = Stamp::day(end.date.previous_day().unwrap_or(end.date));
    }
    if end < start {
        return (None, start);
    }
    (Some(start), end)
}

fn set_text(note: &mut Object, key: &str, value: Option<&str>) {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => {
            note.extra.insert(key.into(), PropertyValue::Text(v.to_string()));
        }
        None => {
            note.extra.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::parse_ics;
    use time::macros::{date, time};

    const TODAY: Date = date!(2026 - 10 - 09);

    fn ics(seq: u32, start: &str, end: &str, extra: &str) -> String {
        format!(
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:abc@google.com\nSEQUENCE:{seq}\nSUMMARY:Lab meeting\n\
             DTSTART:{start}\nDTEND:{end}\nLOCATION:COM1 #02-10\n{extra}END:VEVENT\nEND:VCALENDAR\n"
        )
    }

    fn first_read() -> Object {
        let recs = parse_ics(&ics(0, "20261012T100000", "20261012T110000", ""), None);
        let plan = reconcile(&recs, &HashMap::new(), "work", TODAY);
        assert_eq!(plan.report.created, 1);
        plan.create.into_iter().next().unwrap()
    }

    #[test]
    fn a_new_invite_becomes_a_meeting_note_with_an_empty_body() {
        let note = first_read();
        assert_eq!(note.title.as_deref(), Some("Lab meeting"));
        assert_eq!(note.start, Some(Stamp::at(date!(2026 - 10 - 12), time!(10:00))));
        assert_eq!(note.due, Some(Stamp::at(date!(2026 - 10 - 12), time!(11:00))));
        assert_eq!(note.tags, vec!["meeting".to_string(), "work".to_string()]);
        assert_eq!(note.get(ICS_UID), PropertyValue::Text("abc@google.com".into()));
        assert_eq!(note.get(LOCATION), PropertyValue::Text("COM1 #02-10".into()));
        assert!(note.body.is_empty(), "the body belongs to the person, never to the feed");
    }

    #[test]
    fn reading_the_same_feed_twice_writes_nothing_the_second_time() {
        let note = first_read();
        let existing = index([&note]);
        let recs = parse_ics(&ics(0, "20261012T100000", "20261012T110000", ""), None);
        let plan = reconcile(&recs, &existing, "work", TODAY);
        assert!(plan.create.is_empty() && plan.update.is_empty());
        assert_eq!(plan.report.unchanged, 1);
    }

    #[test]
    fn a_sequence_bump_moves_the_times_and_leaves_the_title_and_body_alone() {
        let mut note = first_read();
        note.title = Some("Lab meeting — my agenda".into());
        note.body = "- ask about the GPU budget\n".into();
        let existing = index([&note]);
        let recs = parse_ics(&ics(1, "20261015T140000", "20261015T150000", ""), None);
        let plan = reconcile(&recs, &existing, "work", TODAY);
        assert_eq!(plan.report.moved, 1);
        let moved = &plan.update[0];
        assert_eq!(moved.id, note.id, "the same note is updated, not a second one created");
        assert_eq!(moved.start, Some(Stamp::at(date!(2026 - 10 - 15), time!(14:00))));
        assert_eq!(moved.due, Some(Stamp::at(date!(2026 - 10 - 15), time!(15:00))));
        assert_eq!(moved.title.as_deref(), Some("Lab meeting — my agenda"));
        assert_eq!(moved.body, "- ask about the GPU budget\n");
        assert_eq!(moved.get(ICS_SEQ), PropertyValue::Int(1));
    }

    #[test]
    fn a_changed_time_without_a_sequence_bump_is_not_applied() {
        // The person may have moved the note themselves; only the publisher's own signal overrides it.
        let note = first_read();
        let existing = index([&note]);
        let recs = parse_ics(&ics(0, "20261015T140000", "20261015T150000", ""), None);
        let plan = reconcile(&recs, &existing, "work", TODAY);
        assert!(plan.update.is_empty());
    }

    #[test]
    fn cancelled_marks_the_note_and_never_deletes_it() {
        let note = first_read();
        let existing = index([&note]);
        let recs =
            parse_ics(&ics(2, "20261012T100000", "20261012T110000", "STATUS:CANCELLED\n"), None);
        let plan = reconcile(&recs, &existing, "work", TODAY);
        assert_eq!(plan.report.cancelled, 1);
        let marked = &plan.update[0];
        assert_eq!(marked.id, note.id);
        assert!(marked.tags.iter().any(|t| t == CANCELLED_TAG));
        assert_eq!(marked.get(ICS_STATUS), PropertyValue::Text("cancelled".into()));
        // There is no deletion list in a `Plan` at all — the type is the guarantee.
    }

    #[test]
    fn a_cancelled_event_nobody_has_a_note_for_is_not_created() {
        let recs =
            parse_ics(&ics(2, "20261012T100000", "20261012T110000", "STATUS:CANCELLED\n"), None);
        let plan = reconcile(&recs, &HashMap::new(), "work", TODAY);
        assert!(plan.create.is_empty());
    }

    #[test]
    fn a_past_event_is_counted_and_not_created() {
        let recs = parse_ics(&ics(0, "20261001T100000", "20261001T110000", ""), None);
        let plan = reconcile(&recs, &HashMap::new(), "work", TODAY);
        assert!(plan.create.is_empty());
        assert_eq!(plan.report.past, 1);
    }

    #[test]
    fn an_event_without_a_uid_is_counted_and_not_created() {
        let text = "BEGIN:VEVENT\nSUMMARY:x\nDTSTART:20261012T100000\nEND:VEVENT\n";
        let plan = reconcile(&parse_ics(text, None), &HashMap::new(), "work", TODAY);
        assert!(plan.create.is_empty());
        assert_eq!(plan.report.no_id, 1);
    }

    #[test]
    fn an_all_day_event_ends_on_its_own_day_not_the_next() {
        let text = "BEGIN:VEVENT\nUID:d\nSUMMARY:Retreat\nDTSTART;VALUE=DATE:20261020\n\
                    DTEND;VALUE=DATE:20261021\nEND:VEVENT\n";
        let plan = reconcile(&parse_ics(text, None), &HashMap::new(), "work", TODAY);
        let note = &plan.create[0];
        assert_eq!(note.start, Some(Stamp::day(date!(2026 - 10 - 20))));
        assert_eq!(note.due, Some(Stamp::day(date!(2026 - 10 - 20))));
    }

    const WEEKLY: &str =
        "BEGIN:VEVENT\nUID:r\nSEQUENCE:0\nSUMMARY:Weekly\nDTSTART:20261013T090000\n\
                          DTEND:20261013T100000\nRRULE:FREQ=WEEKLY\nEND:VEVENT\n";

    fn days(plan: &Plan) -> Vec<String> {
        let mut d: Vec<String> = plan.create.iter().map(|n| n.start.unwrap().to_string()).collect();
        d.sort();
        d
    }

    #[test]
    fn a_weekly_series_becomes_one_note_per_week_over_the_window() {
        let plan = reconcile(&parse_ics(WEEKLY, None), &HashMap::new(), "work", TODAY);
        // Tuesdays from the 13th to 60 days after the 9th (8 December): nine of them.
        assert_eq!(plan.report.created, 9, "{:?}", days(&plan));
        assert_eq!(plan.report.series, 1);
        assert_eq!(plan.report.repeating, 0);
        assert_eq!(days(&plan)[0], "2026-10-13T09:00");
        assert_eq!(days(&plan)[8], "2026-12-08T09:00");
        let first = &plan
            .create
            .iter()
            .find(|n| n.start.unwrap().to_string() == "2026-10-13T09:00")
            .unwrap();
        assert_eq!(
            first.due.unwrap().to_string(),
            "2026-10-13T10:00",
            "each keeps the series' length"
        );
        assert_eq!(first.get(ICS_UID), PropertyValue::Text("r#2026-10-13".into()));
    }

    #[test]
    fn a_series_that_began_long_ago_still_shows_its_coming_weeks() {
        // The Phase 1 gap: only the first date was read, it was past, so the meeting showed nothing.
        let text =
            WEEKLY.replace("20261013T09", "20240102T09").replace("20261013T10", "20240102T10");
        let plan = reconcile(&parse_ics(&text, None), &HashMap::new(), "work", TODAY);
        assert!(plan.report.created >= 8, "{:?}", days(&plan));
        assert!(
            days(&plan).iter().all(|d| d.as_str() >= "2026-10-09"),
            "nothing in the past is created"
        );
    }

    #[test]
    fn a_moved_occurrence_replaces_the_one_it_overrides_and_an_exdate_removes_one() {
        let text = WEEKLY
            .replace("RRULE:FREQ=WEEKLY\n", "RRULE:FREQ=WEEKLY\nEXDATE:20261027T090000\n")
            + "BEGIN:VEVENT\nUID:r\nRECURRENCE-ID:20261020T090000\nSUMMARY:Weekly\n\
               DTSTART:20261021T140000\nDTEND:20261021T150000\nEND:VEVENT\n";
        let plan = reconcile(&parse_ics(&text, None), &HashMap::new(), "work", TODAY);
        let d = days(&plan);
        assert!(d.contains(&"2026-10-21T14:00".to_string()), "the moved one is there: {d:?}");
        assert!(!d.contains(&"2026-10-20T09:00".to_string()), "and not where it was: {d:?}");
        assert!(!d.contains(&"2026-10-27T09:00".to_string()), "the excluded week is gone: {d:?}");
        assert_eq!(plan.report.created, 8);
    }

    #[test]
    fn a_rescheduled_series_marks_the_dates_it_no_longer_has_and_deletes_nothing() {
        let first = reconcile(&parse_ics(WEEKLY, None), &HashMap::new(), "work", TODAY);
        let existing = index(first.create.iter());
        // Moved from Tuesdays to Wednesdays.
        let moved = WEEKLY.replace("SEQUENCE:0", "SEQUENCE:1").replace("20261013T", "20261014T");
        let plan = reconcile(&parse_ics(&moved, None), &existing, "work", TODAY);
        assert_eq!(plan.report.cancelled, 9, "every Tuesday is marked");
        assert!(plan.update.iter().all(|n| n.tags.iter().any(|t| t == CANCELLED_TAG)));
        assert_eq!(plan.report.created, 8, "and the Wednesdays appear");
    }

    #[test]
    fn a_meeting_already_held_is_never_marked_even_if_the_series_changed() {
        let first = reconcile(&parse_ics(WEEKLY, None), &HashMap::new(), "work", TODAY);
        let existing = index(first.create.iter());
        // A week later, the series moves to Wednesdays. The Tuesday of the 13th has happened.
        let later = date!(2026 - 10 - 16);
        let moved = WEEKLY.replace("SEQUENCE:0", "SEQUENCE:1").replace("20261013T", "20261014T");
        let plan = reconcile(&parse_ics(&moved, None), &existing, "work", later);
        assert!(
            plan.update
                .iter()
                .all(|n| n.get(ICS_UID) != PropertyValue::Text("r#2026-10-13".into())),
            "a meeting that took place stays as it was"
        );
    }

    #[test]
    fn a_series_missing_from_this_read_marks_nothing() {
        // A feed that briefly omits a series is not the organiser cancelling it.
        let first = reconcile(&parse_ics(WEEKLY, None), &HashMap::new(), "work", TODAY);
        let existing = index(first.create.iter());
        let other = "BEGIN:VEVENT\nUID:x\nSUMMARY:Other\nDTSTART:20261012T100000\nEND:VEVENT\n";
        let plan = reconcile(&parse_ics(other, None), &existing, "work", TODAY);
        assert_eq!(plan.report.cancelled, 0);
    }

    #[test]
    fn re_reading_a_series_writes_nothing() {
        let first = reconcile(&parse_ics(WEEKLY, None), &HashMap::new(), "work", TODAY);
        let existing = index(first.create.iter());
        let plan = reconcile(&parse_ics(WEEKLY, None), &existing, "work", TODAY);
        assert!(plan.create.is_empty() && plan.update.is_empty(), "{:?}", plan.report);
        assert_eq!(plan.report.unchanged, 9);
    }

    #[test]
    fn a_rule_it_cannot_read_shows_the_first_date_and_is_counted() {
        let text = WEEKLY.replace("FREQ=WEEKLY", "FREQ=MONTHLY;BYDAY=MO;BYSETPOS=-1");
        let plan = reconcile(&parse_ics(&text, None), &HashMap::new(), "work", TODAY);
        assert_eq!(plan.report.repeating, 1);
        assert_eq!(plan.report.series, 0);
        assert_eq!(plan.report.created, 1);
    }

    #[test]
    fn a_foreign_zone_is_carried_onto_the_note_so_it_does_not_pass_as_local_time() {
        let text = "BEGIN:VEVENT\nUID:z\nSUMMARY:Call\nDTSTART;TZID=Europe/Berlin:20261012T100000\nEND:VEVENT\n";
        let plan = reconcile(&parse_ics(text, None), &HashMap::new(), "work", TODAY);
        assert_eq!(plan.create[0].get(ICS_TIME_ZONE), PropertyValue::Text("Europe/Berlin".into()));
    }
}
