//! **Fitness → Coach**: the plan followed for a goal, its own calendar, progress per sport, and
//! matching a watch's session to a planned one (`decisions.md` 2026-11-20, *Fitness gets a Coach
//! tab; the app still chooses no loads*; the coach plan's Spec K and S.5).
//!
//! Everything here reads notes, except [`accept_match`], which the person asks for. **Nothing here
//! steps a plan forward, writes a planned session or chooses a number**: the person puts a week on
//! the calendar (`create_notes`), presses *Next week* (`set_properties` on the program note) and
//! moves, skips or ticks a session (`set_properties`) — the commands every other tracker writes with.
//!
//! The words it reads:
//! - a **program note** `program: coach`, `plan: <library id>`, `question: goal`, `goal`,
//!   `event_date` (optional) and `week` (the plan's week the person is on). Stopping gives it a `due`.
//! - a **planned session**: a `log: workout` entry with `planned: 1`, `start`, and three links —
//!   `plan` (the program note's id), `plan_week` and `session` (the template's id). Done, it keeps
//!   what was planned as `planned_<key>` ([`PLANNED_KEYS`]); skipped, it keeps `planned` and gains
//!   `skipped`.
//! - an **imported session**: `source_key` beginning `fit:` (`watch.rs`).
//!
//! The Agenda, the board and `notes_base` are not touched: training is drawn on the Coach's own
//! calendar only.

use crate::commands::DeletedNote;
use crate::dto::ObjectMeta;
use crate::log::{PLANNED, PROGRAM};
use crate::plan::{Plan, COUNTS_AS, PLAN};
use crate::thread::{coach_base, is_message, is_proposal, is_template, LOG};
use fm_core::import::SOURCE_KEY;
use fm_core::{Store, StoreError};
use fm_model::{Id, Object, PropertyValue};
use fm_query::{Query, SortKey};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use time::{format_description::well_known::Iso8601, Date, Duration, OffsetDateTime};

/// A program note of the Coach carries `program: coach`.
pub const COACH: &str = "coach";
/// The plan's week a program note is on — moved only when the person presses *Next week*.
pub const WEEK: &str = "week";
/// What a program note trains for, and the day it is for (optional).
pub const GOAL: &str = "goal";
pub const EVENT_DATE: &str = "event_date";
/// A planned session's links back to its plan: the week of the plan and the session template.
pub const PLAN_WEEK: &str = "plan_week";
pub const SESSION: &str = "session";
/// A planned session the person skipped: it stays planned, and is counted as not done.
pub const SKIPPED: &str = "skipped";
/// The figures a planned entry keeps as `planned_<key>` once it is done, so planned and done can be
/// laid side by side (Spec K.3). The same list as `ui/src/lib/training/coach.ts`'s.
pub const PLANNED_KEYS: [&str; 5] = ["minutes", "distance_m", "reps", "load_g", "effort"];

/// More days than this on one calendar is a mistake: a month view is six weeks.
const MAX_DAYS: i64 = 62;
/// More weeks of progress than this is a mistake.
const MAX_WEEKS: i64 = 26;

fn parse_day(which: &str, s: &str) -> Result<Date, StoreError> {
    Date::parse(s, &Iso8601::DATE)
        .map_err(|_| StoreError::Parse(format!("{which} must be a day like 2026-07-20, not '{s}'")))
}
fn day_string(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}
fn text(o: &Object, key: &str) -> String {
    match o.get(key) {
        PropertyValue::Text(s) => s,
        _ => String::new(),
    }
}
fn int(o: &Object, key: &str) -> Option<i64> {
    match o.get(key) {
        PropertyValue::Int(v) => Some(v),
        PropertyValue::Text(s) => s.trim().parse().ok(),
        _ => None,
    }
}
fn truthy(o: &Object, key: &str) -> bool {
    !matches!(o.get(key), PropertyValue::Null | PropertyValue::Bool(false) | PropertyValue::Int(0))
        && !matches!(o.get(key), PropertyValue::Text(s) if s.trim().is_empty())
}
fn is_planned(o: &Object) -> bool {
    truthy(o, PLANNED)
}
fn is_log(o: &Object, kind: &str) -> bool {
    text(o, LOG) == kind
}
fn hidden(o: &Object) -> bool {
    is_template(o) || is_message(o) || is_proposal(o)
}
/// The day an entry is on: a watch's own day where the import wrote one, else its `start`.
fn day_of(o: &Object) -> Option<Date> {
    match o.get("day") {
        PropertyValue::Stamp(s) => Some(s.date),
        PropertyValue::Text(s) => Date::parse(s.get(..10).unwrap_or(""), &Iso8601::DATE).ok(),
        _ => None,
    }
    .or(o.start.map(|s| s.date))
}
/// An activity entry (a run, a ride, a session) — not one set of an exercise.
fn is_activity(o: &Object) -> bool {
    !text(o, "activity").is_empty()
}
/// What an entry counts as, as `plan.rs` reads it: a set is strengthening; an activity says.
fn counts_as(o: &Object) -> String {
    let kind = text(o, COUNTS_AS);
    if !kind.is_empty() {
        kind
    } else if int(o, "reps").is_some() || !text(o, "exercise").is_empty() {
        "strength".into()
    } else {
        "aerobic".into()
    }
}
fn is_imported(o: &Object) -> bool {
    let key = text(o, SOURCE_KEY);
    key.starts_with("fit:") && !key.contains(":set:")
}

/// Copy the planned figures of an entry into `planned_<key>`, once: what was planned is kept when
/// the done figures are written over it. A key already kept is left as it is.
pub fn keep_planned(o: &mut Object) {
    for key in PLANNED_KEYS {
        if let PropertyValue::Int(v) = o.get(key) {
            let kept = format!("planned_{key}");
            if o.get(&kept).is_null() {
                o.extra.insert(kept, PropertyValue::Int(v));
            }
        }
    }
}

// ── the plan followed, and the calendar ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Followed {
    pub note: ObjectMeta,
    /// The library's plan, when this version carries it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    /// The plan's week the person is on (1 when the note says nothing).
    pub week: i64,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CoachDay {
    pub day: String,
    /// Every entry of the training log on the day: planned, done and skipped.
    pub training: Vec<ObjectMeta>,
    /// The Menu's planned meals (`log: food` with `planned`).
    pub meals: Vec<ObjectMeta>,
    /// Meetings the read-only calendar sync wrote.
    pub meetings: Vec<ObjectMeta>,
}

#[derive(Debug, Serialize)]
pub struct CoachAnswer {
    /// The Coach plans in force on `day`, the latest first. One is followed at a time; an older one
    /// still in force is listed so it can be stopped.
    pub followed: Vec<Followed>,
    pub days: Vec<CoachDay>,
}

/// The Coach's plan in force on `day` and its calendar from `from` to `to`: one pass over the vault.
pub fn coach(
    store: &dyn Store,
    day: &str,
    from: &str,
    to: &str,
) -> Result<CoachAnswer, StoreError> {
    let on = parse_day("day", day)?;
    let from = parse_day("from", from)?;
    let to = parse_day("to", to)?;
    if to < from || (to - from).whole_days() >= MAX_DAYS {
        return Err(StoreError::Parse(format!(
            "from {} to {} is not a span of at most {MAX_DAYS} days",
            day_string(from),
            day_string(to)
        )));
    }
    let q =
        Query { filter: coach_base(), sort: vec![SortKey::asc("created")], ..Default::default() };
    let mut programs: Vec<Object> = Vec::new();
    let mut by_day: BTreeMap<Date, CoachDay> = BTreeMap::new();
    let mut d = from;
    loop {
        by_day.insert(
            d,
            CoachDay {
                day: day_string(d),
                training: Vec::new(),
                meals: Vec::new(),
                meetings: Vec::new(),
            },
        );
        if d >= to {
            break;
        }
        d = d.next_day().ok_or_else(|| StoreError::Parse("the calendar ended".into()))?;
    }
    for o in store.query(&q)?.rows {
        if hidden(&o) {
            continue;
        }
        if text(&o, LOG).is_empty() && text(&o, PROGRAM) == COACH {
            let started = o.start.is_none_or(|s| s.date <= on);
            let ended = o.due.is_some_and(|s| s.date < on);
            if started && !ended && !text(&o, PLAN).is_empty() {
                programs.push(o);
            }
            continue;
        }
        let Some(at) = o.start.map(|s| s.date) else { continue };
        let Some(slot) = by_day.get_mut(&at) else { continue };
        if is_log(&o, "workout") {
            slot.training.push(ObjectMeta::from(&o));
        } else if is_log(&o, "food") {
            if is_planned(&o) {
                slot.meals.push(ObjectMeta::from(&o));
            }
        } else if crate::kinds::kind_of(&o) == "meeting" {
            slot.meetings.push(ObjectMeta::from(&o));
        }
    }
    // The latest start first; a note with no start loses to any that has one.
    programs.sort_by_key(|p| std::cmp::Reverse((p.start.map(|s| s.date), p.created)));
    let followed = programs
        .iter()
        .map(|p| Followed {
            note: ObjectMeta::from(p),
            plan: crate::plan::plan_by_id(&text(p, PLAN)).cloned(),
            week: int(p, WEEK).unwrap_or(1).max(1),
            goal: text(p, GOAL),
            event_date: match p.get(EVENT_DATE) {
                PropertyValue::Null => None,
                PropertyValue::Stamp(s) => Some(day_string(s.date)),
                v => Some(v.display()).filter(|s| !s.is_empty()),
            },
        })
        .collect();
    Ok(CoachAnswer { followed, days: by_day.into_values().collect() })
}

// ── progress: planned and done, per week and per sport ────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct SportWeek {
    /// fm's one-word sport (`run`, `ride`, `strength`…), or an activity's own name when it has none.
    pub sport: String,
    /// Sessions that were planned (still planned, skipped, or done from a plan).
    pub planned: i64,
    /// Sessions done (planned or not).
    pub done: i64,
    pub skipped: i64,
    pub planned_minutes: i64,
    pub done_minutes: i64,
    /// Distance done, in metres.
    pub distance_m: i64,
}

#[derive(Debug, Serialize)]
pub struct WeekProgress {
    pub from: String,
    pub to: String,
    pub sports: Vec<SportWeek>,
}

/// The sport an entry is of: its own `sport`, else the one the shipped sport table gives its
/// activity's name (`Running` → `run`), else the activity's name.
fn sport_of(o: &Object, by_activity: &HashMap<String, String>) -> String {
    let own = text(o, "sport");
    if !own.is_empty() {
        return own;
    }
    let activity = text(o, "activity");
    if activity.is_empty() {
        return "strength".into();
    }
    by_activity.get(&activity.to_lowercase()).cloned().unwrap_or(activity)
}

/// Each of the `weeks` weeks up to the one holding `day`, oldest first: per sport, sessions planned
/// and done, minutes planned and done, and distance done. **Measured facts only** — no load, no
/// zone, no line across sports. A day's sets of exercises are one strength session.
pub fn coach_progress(
    store: &dyn Store,
    day: &str,
    weeks: i64,
) -> Result<Vec<WeekProgress>, StoreError> {
    let day = parse_day("day", day)?;
    if !(1..=MAX_WEEKS).contains(&weeks) {
        return Err(StoreError::Parse(format!("weeks must be 1 to {MAX_WEEKS}")));
    }
    let monday = day - Duration::days(i64::from(day.weekday().number_days_from_monday()));
    let first = monday - Duration::days(7 * (weeks - 1));
    let last = monday + Duration::days(6);
    let by_activity: HashMap<String, String> = crate::watch::sport_rows()
        .into_iter()
        .map(|r| (r.activity.to_lowercase(), r.sport))
        .collect();
    let q = Query {
        filter: crate::thread::plan_base(&["workout"], &[]),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    // (week index, sport) → the tally; and per day, whether sets were done or planned there.
    let mut tally: BTreeMap<(i64, String), SportWeek> = BTreeMap::new();
    let mut sets: BTreeMap<Date, (bool, bool)> = BTreeMap::new();
    let mut strength_days: BTreeMap<Date, (bool, bool)> = BTreeMap::new();
    for o in store.query(&q)?.rows {
        if hidden(&o) {
            continue;
        }
        let Some(at) = o.start.map(|s| s.date) else { continue };
        if at < first || at > last {
            continue;
        }
        let planned_now = is_planned(&o);
        let was_planned = planned_now
            || !text(&o, PLAN).is_empty()
            || PLANNED_KEYS.iter().any(|k| !o.get(&format!("planned_{k}")).is_null());
        if !is_activity(&o) {
            let e = sets.entry(at).or_default();
            e.0 |= !planned_now;
            e.1 |= was_planned;
            continue;
        }
        let sport = sport_of(&o, &by_activity);
        if sport == "strength" {
            let e = strength_days.entry(at).or_default();
            e.0 |= !planned_now;
            e.1 |= was_planned;
        }
        let w = (at - first).whole_days() / 7;
        let t = tally
            .entry((w, sport.clone()))
            .or_insert_with(|| SportWeek { sport, ..Default::default() });
        if was_planned {
            t.planned += 1;
            t.planned_minutes += if planned_now {
                int(&o, "minutes").unwrap_or(0)
            } else {
                int(&o, "planned_minutes").unwrap_or(0)
            };
        }
        if planned_now && truthy(&o, SKIPPED) {
            t.skipped += 1;
        }
        if !planned_now {
            t.done += 1;
            t.done_minutes += int(&o, "minutes").unwrap_or(0);
            t.distance_m += int(&o, "distance_m").unwrap_or(0);
        }
    }
    // A day of sets is a strength session — unless a strength activity already stands for it.
    for (at, (done, planned)) in sets {
        let has = strength_days.get(&at).copied().unwrap_or_default();
        let w = (at - first).whole_days() / 7;
        let t = tally
            .entry((w, "strength".into()))
            .or_insert_with(|| SportWeek { sport: "strength".into(), ..Default::default() });
        if done && !has.0 {
            t.done += 1;
        }
        if planned && !has.1 {
            t.planned += 1;
        }
    }
    Ok((0..weeks)
        .map(|w| {
            let from = first + Duration::days(7 * w);
            WeekProgress {
                from: day_string(from),
                to: day_string(from + Duration::days(6)),
                sports: tally
                    .iter()
                    .filter(|((i, _), _)| *i == w)
                    .map(|(_, t)| t.clone())
                    .collect(),
            }
        })
        .collect())
}

// ── a watch's session, matched to a planned one (Spec S.5) ───────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct Proposal {
    pub imported: ObjectMeta,
    pub planned: ObjectMeta,
}

#[derive(Debug, Serialize)]
pub struct Unmatched {
    pub imported: ObjectMeta,
    /// Planned entries it could be: several, or none.
    pub candidates: Vec<ObjectMeta>,
}

#[derive(Debug, Serialize)]
pub struct Matches {
    /// Exactly one planned entry fits: *"This was your planned …?"*.
    pub proposals: Vec<Proposal>,
    /// Several fit — or, when a day was asked about, none: listed, nothing proposed.
    pub unmatched: Vec<Unmatched>,
}

/// For each imported activity without a `plan` link (on `day`, when one is given): the planned
/// entries on its day that count as the same. **Never acts**: one fit is a proposal, the person
/// accepts it ([`accept_match`]); several are listed. A planned entry two imports could both be is
/// no proposal for either.
pub fn match_planned(store: &dyn Store, day: Option<&str>) -> Result<Matches, StoreError> {
    let only = day.filter(|d| !d.is_empty()).map(|d| parse_day("day", d)).transpose()?;
    let q = Query {
        filter: crate::thread::plan_base(&["workout"], &[]),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let rows: Vec<Object> = store.query(&q)?.rows.into_iter().filter(|o| !hidden(o)).collect();
    let planned: Vec<&Object> = rows
        .iter()
        .filter(|o| is_planned(o) && is_activity(o) && !truthy(o, SKIPPED) && o.start.is_some())
        .collect();
    let mut found: Vec<(&Object, Vec<&Object>)> = Vec::new();
    for o in rows.iter().filter(|o| is_imported(o) && text(o, PLAN).is_empty() && !is_planned(o)) {
        let Some(at) = day_of(o) else { continue };
        if only.is_some_and(|d| d != at) {
            continue;
        }
        let kind = counts_as(o);
        let fits: Vec<&Object> = planned
            .iter()
            .copied()
            .filter(|p| p.start.is_some_and(|s| s.date == at) && counts_as(p) == kind)
            .collect();
        found.push((o, fits));
    }
    // How many imports each planned entry is the one fit of.
    let mut claimed: HashMap<Id, usize> = HashMap::new();
    for (_, fits) in &found {
        if let [one] = fits.as_slice() {
            *claimed.entry(one.id).or_default() += 1;
        }
    }
    let mut out = Matches { proposals: Vec::new(), unmatched: Vec::new() };
    for (o, fits) in found {
        match fits.as_slice() {
            [one] if claimed.get(&one.id) == Some(&1) => out
                .proposals
                .push(Proposal { imported: ObjectMeta::from(o), planned: ObjectMeta::from(*one) }),
            [] if only.is_none() => {}
            _ => out.unmatched.push(Unmatched {
                imported: ObjectMeta::from(o),
                candidates: fits.iter().map(|p| ObjectMeta::from(*p)).collect(),
            }),
        }
    }
    Ok(out)
}

/// What [`accept_match`] changed, so the person can take it back: the planned note's values before
/// (`None` where it had none — the same answer `set_properties` gives), its text before, and the
/// imported note as it was.
#[derive(Debug, Serialize)]
pub struct Folded {
    pub planned: String,
    pub vault: String,
    pub previous: BTreeMap<String, Option<String>>,
    pub body_before: String,
    pub removed: DeletedNote,
    /// Whether the change was recorded as one step in the vault's history.
    pub recorded: bool,
}

/// Keys of an imported note that are not carried onto the planned one: what makes it an entry of
/// a plan stays the planned note's own.
const NOT_CARRIED: [&str; 4] = [PLANNED, PLAN, PLAN_WEEK, SESSION];

/// **The person said yes**: the planned note takes the imported session's done values (keeping its
/// planned ones as `planned_*`), stops being planned, and the imported note is folded into it — its
/// `source_key`, `source_fit`, file and body tables move across, and the imported note is removed.
/// One note per session. The caller records the change as one step.
pub fn accept_match(
    store: &mut dyn Store,
    planned_id: &str,
    imported_id: &str,
) -> Result<Folded, StoreError> {
    let parse = |id: &str| -> Result<Id, StoreError> {
        id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))
    };
    let (pid, iid) = (parse(planned_id)?, parse(imported_id)?);
    let mut planned = store.get(pid)?.ok_or(StoreError::NotFound(pid))?;
    let imported = store.get(iid)?.ok_or(StoreError::NotFound(iid))?;
    if !is_log(&planned, "workout") || !is_planned(&planned) {
        return Err(StoreError::Parse("that entry is not a planned session any more".into()));
    }
    if !is_imported(&imported) || !text(&imported, PLAN).is_empty() {
        return Err(StoreError::Parse("that entry is not a session read from a watch".into()));
    }
    if planned.vault != imported.vault {
        return Err(StoreError::Parse("the two entries are in different vaults".into()));
    }
    let before = planned.clone();
    let mut previous: BTreeMap<String, Option<String>> = BTreeMap::new();
    let note = |o: &Object, key: &str, previous: &mut BTreeMap<String, Option<String>>| {
        if !previous.contains_key(key) {
            let v = o.get(key);
            previous.insert(key.to_string(), (!v.is_null()).then(|| v.display()));
        }
    };
    keep_planned(&mut planned);
    for (k, v) in &imported.extra {
        if NOT_CARRIED.contains(&k.as_str()) || k.starts_with("planned_") {
            continue;
        }
        note(&before, k, &mut previous);
        planned.extra.insert(k.clone(), v.clone());
    }
    for k in PLANNED_KEYS {
        let kept = format!("planned_{k}");
        if before.get(&kept).is_null() && !planned.get(&kept).is_null() {
            note(&before, &kept, &mut previous);
        }
    }
    note(&before, PLANNED, &mut previous);
    planned.extra.remove(PLANNED);
    if imported.start.is_some() && imported.start != before.start {
        previous.insert("start".into(), before.start.map(|s| s.to_string()));
        planned.start = imported.start;
    }
    if !imported.body.trim().is_empty() {
        planned.body = if before.body.trim().is_empty() {
            imported.body.clone()
        } else {
            format!("{}\n\n{}", before.body.trim_end(), imported.body.trim())
        };
    }
    for a in &imported.assets {
        if !planned.assets.contains(a) {
            planned.assets.push(a.clone());
        }
    }
    planned.updated = OffsetDateTime::now_utc();
    store.put(&planned)?;
    let removed = crate::commands::delete_keeping(store, imported_id)?;
    Ok(Folded {
        planned: planned_id.to_string(),
        vault: planned.vault.clone(),
        previous,
        body_before: before.body,
        removed,
        recorded: false,
    })
}
