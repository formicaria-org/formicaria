//! **The Health diary**: what a person tracks, and their days of it (`decisions.md#ui` `#data`,
//! 2026-10-22, *a fifth group, Health*; the design and its sources: `health-plan-2026-10-22.md`).
//!
//! An entry is a day note carrying `log: health`, dated by day in `start`. **A tracker is a key,
//! and the end of the key says what kind it is** (`headache_0to3`, `bedding_washed_yes`,
//! `sleep_min`): the unit-in-the-key rule of 2026-10-11, one step further. Nothing about a tracker is
//! stored anywhere but its key, so the file reads without the app.
//!
//! **What a person tracks is their Health template**: a note tagged `template` with `group:
//! Health`, carrying their keys. Adding a tracker writes a key there. That is editing a note,
//! which a person can already do by hand, and not authoring a view.
//!
//! This module counts nothing itself: [`series`] reads the days, and `fm_query::days` does the
//! arithmetic.

use crate::thread::{is_template, log_base, LOG};
use fm_core::{Store, StoreError};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::days::Series;
use fm_query::{Filter, Predicate, Query};
use serde::Serialize;
use std::collections::BTreeMap;
use time::Date;

/// The `log` kind of a Health entry.
pub const HEALTH: &str = "health";
/// The group a Health template names, as its views name it.
pub const GROUP_NAME: &str = "Health";
/// The key a template names its group by (`ui/src/lib/templates.ts`'s `GROUP`).
const GROUP: &str = "group";

/// **What kind a tracker is, read off the end of its key.** A closed set: a suffix outside it is
/// not a tracker, so a stray property on a template never becomes a control on the diary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TrackerKind {
    /// `_0to3`: IHS's *"0 = no headache; 1 = mild headache; 2 = moderate headache; 3 = severe
    /// headache"* (Diener 2019), used for any severity.
    Severity,
    /// `_0to10`: an 11-point scale, shown as numbers only — its usual end anchors are unread.
    Rating,
    /// `_0to100`: how much it bothered, from *"not at all bothersome"* to *"extremely
    /// bothersome"* (adapted from Sousa-Pinto 2021, CC BY 4.0).
    Bother,
    /// `_yes`: 1 or 0.
    YesNo,
    /// `_count` or `_doses`: how many times.
    Count,
    /// A unit: how much, in that unit.
    Amount { unit: &'static str },
    /// `_score`: a total the person got on a questionnaire elsewhere. Never its items.
    Score,
}

/// The units a tracker may be measured in. **Grows only with a source for the unit**; a key ending
/// in anything else is not a tracker.
/// `c10` and `mmoll10` are tenths, of a degree Celsius and of a millimole per litre: a temperature
/// of 38.1 °C is `381`, a glucose of 5.4 mmol/L is `54`, so the number stays whole.
const UNITS: &[&str] = &["min", "ml", "mg", "g", "mmhg", "pct", "c", "c10", "mmoll10"];

impl TrackerKind {
    /// The least and the most a recorded value may be, where the kind bounds it.
    pub fn bounds(&self) -> Option<(i64, i64)> {
        match self {
            TrackerKind::Severity => Some((0, 3)),
            TrackerKind::Rating => Some((0, 10)),
            TrackerKind::Bother => Some((0, 100)),
            TrackerKind::YesNo => Some((0, 1)),
            TrackerKind::Count => Some((0, i64::MAX)),
            TrackerKind::Amount { .. } | TrackerKind::Score => None,
        }
    }

    /// Whether `v` may be recorded for this kind.
    pub fn accepts(&self, v: i64) -> bool {
        self.bounds().is_none_or(|(lo, hi)| (lo..=hi).contains(&v))
    }

    /// How two entries of one day become the day's value, when a day has two notes (two devices,
    /// before a merge settles them). **A severity is the day's peak** (IHS records peak severity),
    /// as are a rating, a bother and a yes; a count or an amount is added up; a score is the
    /// higher, since two scores on one day are a mistake either way.
    fn combine(&self, a: i64, b: i64) -> i64 {
        match self {
            TrackerKind::Count | TrackerKind::Amount { .. } => a.saturating_add(b),
            _ => a.max(b),
        }
    }
}

/// The part of a key before its kind's suffix, and the kind. `None` when the key is not a tracker.
pub fn parse_key(key: &str) -> Option<(&str, TrackerKind)> {
    let fixed: &[(&str, TrackerKind)] = &[
        ("_0to3", TrackerKind::Severity),
        ("_0to10", TrackerKind::Rating),
        ("_0to100", TrackerKind::Bother),
        ("_yes", TrackerKind::YesNo),
        ("_count", TrackerKind::Count),
        ("_doses", TrackerKind::Count),
        ("_score", TrackerKind::Score),
    ];
    let named = |stem: &str| fm_model::parse_log_kind(stem).is_some() && !stem.ends_with('_');
    for (suffix, kind) in fixed {
        if let Some(stem) = key.strip_suffix(suffix).filter(|s| named(s)) {
            return Some((stem, kind.clone()));
        }
    }
    let (stem, unit) = key.rsplit_once('_')?;
    let unit = UNITS.iter().find(|u| **u == unit)?;
    named(stem).then_some((stem, TrackerKind::Amount { unit }))
}

/// What a tracker is called on the screen: its key's stem, with spaces for underscores and a
/// capital first letter (`bedding_washed_yes` → *Bedding washed*). A name in another script
/// needs a label that is not derived from the key; that is open (`health-plan-2026-10-22.md` §7).
pub fn label_of(stem: &str) -> String {
    let words = stem.replace('_', " ");
    let mut c = words.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// One thing a person tracks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tracker {
    pub key: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: TrackerKind,
}

fn is_health_template(o: &Object) -> bool {
    o.kind == Kind::Note && is_template(o) && o.get(GROUP) == PropertyValue::Text(GROUP_NAME.into())
}

/// **What this person tracks**: every tracker key on their Health templates, once each, in key
/// order. A key on a template that is not a tracker (a `status`, a stray word) is left out.
pub fn trackers(store: &dyn Store) -> Result<Vec<Tracker>, StoreError> {
    Ok(of_templates(&templates(store)?))
}

/// The person's Health templates, oldest first.
fn templates(store: &dyn Store) -> Result<Vec<Object>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::TagsAll(vec![crate::thread::TEMPLATE_TAG.into()])),
        sort: vec![fm_query::SortKey::asc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.into_iter().filter(is_health_template).collect())
}

fn of_templates(templates: &[Object]) -> Vec<Tracker> {
    let mut found: BTreeMap<String, Tracker> = BTreeMap::new();
    for o in templates {
        for key in o.extra.keys() {
            if let Some((stem, kind)) = parse_key(key) {
                found.entry(key.clone()).or_insert_with(|| Tracker {
                    key: key.clone(),
                    label: label_of(stem),
                    kind,
                });
            }
        }
    }
    found.into_values().collect()
}

/// **Each tracker's recorded days in `from..=to`.** A day is recorded for a tracker when one of its
/// Health notes carries a whole number for the key that the kind accepts. A value out of bounds or
/// not a whole number is not recorded, and is counted in `odd` so a screen can say so rather than
/// drop it silently (the rule `MetricSum::odd` keeps for sums).
pub fn series(
    store: &dyn Store,
    trackers: &[Tracker],
    from: Date,
    to: Date,
) -> Result<ByTracker, StoreError> {
    Ok(read(store, trackers, from, to)?.0)
}

/// The Health notes of each day in `from..=to`, oldest first within a day.
pub type Notes = BTreeMap<Date, Vec<Object>>;

/// **The Health notes of each day in `from..=to`** and nothing counted: for what is read off a
/// note other than a tracker's number (the cycle's context, a cycle marked as missed logging).
pub fn day_notes(store: &dyn Store, from: Date, to: Date) -> Result<Notes, StoreError> {
    Ok(read(store, &[], from, to)?.1)
}
/// Each tracker's recorded days, and how many values on its key could not be read.
type ByTracker = BTreeMap<String, (Series, usize)>;

/// **One pass over the vault** for both the series and the notes they came from.
fn read(
    store: &dyn Store,
    trackers: &[Tracker],
    from: Date,
    to: Date,
) -> Result<(ByTracker, Notes), StoreError> {
    let q = Query {
        filter: log_base(HEALTH),
        sort: vec![fm_query::SortKey::asc("created")],
        ..Default::default()
    };
    let mut out: ByTracker = trackers.iter().map(|t| (t.key.clone(), (Series::new(), 0))).collect();
    let mut notes = Notes::new();
    for o in store.query(&q)?.rows {
        if o.get(LOG) != PropertyValue::Text(HEALTH.into()) {
            continue;
        }
        let Some(day) = o.start.map(|s| s.date).filter(|d| *d >= from && *d <= to) else {
            continue;
        };
        tally(&mut out, trackers, &o, day);
        notes.entry(day).or_default().push(o);
    }
    Ok((out, notes))
}

fn tally(out: &mut ByTracker, trackers: &[Tracker], o: &Object, day: Date) {
    for t in trackers {
        let (s, odd) = out.get_mut(&t.key).expect("one entry per tracker");
        match o.get(&t.key) {
            PropertyValue::Null => {}
            PropertyValue::Int(v) if t.kind.accepts(v) => {
                let v = match s.get(&day) {
                    Some(had) => t.kind.combine(*had, v),
                    None => v,
                };
                s.insert(day, v);
            }
            _ => *odd += 1,
        }
    }
}

/// One day of the diary as a screen reads it.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HealthDay {
    pub day: String,
    /// The day's note, to write the next value into. The oldest when a day has two.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// What was recorded, by tracker key. A key absent here was not recorded that day.
    pub values: BTreeMap<String, i64>,
    /// **Written in later.** The note was made more than a day after the day it is about: filling
    /// in yesterday is ordinary, filling in last week is recall, and a doctor reading the month
    /// should be able to tell. Paper diaries are often written up afterwards (Stone 2002), and
    /// electronic diaries are preferred *"with time stamps (to reduce recall bias)"* (IHS,
    /// Tassorelli 2018). Shown, never locked.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub late: bool,
    /// The cycle's context, when the person set or cleared one on this day (`cycle.rs`): their own
    /// words, or `none` where one ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// **What fm already knows about the day, read and not typed again**: from Food's log the water
    /// (`water_mg`, foods and drinks alike), caffeine (`caffeine_ug`) and alcohol (`alcohol_mg`);
    /// from the body log the weight (`weight_g`, a mean) and steps. Only what was logged.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub elsewhere: BTreeMap<String, i64>,
}

/// The values read from the other logs, as one `totals` definition so `log::log_days` reads them
/// exactly as Food and its trends do.
fn elsewhere_def() -> crate::views::TotalsDef {
    use crate::views::{AddAction, Agg, Metric, Role, Span, TotalsDef};
    let m = |key: &str, agg: Agg, log: Option<&str>| Metric {
        key: key.into(),
        label: key.into(),
        role: Role::Detail,
        agg,
        log: log.map(str::to_string),
    };
    TotalsDef {
        log: "food".into(),
        add: AddAction::Food,
        span: Span::Day,
        metrics: vec![
            m("water_mg", Agg::Sum, None),
            m("caffeine_ug", Agg::Sum, None),
            m("alcohol_mg", Agg::Sum, None),
            m("weight_g", Agg::Mean, Some("body")),
            m("steps", Agg::Sum, Some("body")),
        ],
        slots: Vec::new(),
    }
}

/// Each day's values from the other logs, by day.
pub fn elsewhere(
    store: &dyn Store,
    from: &str,
    to: &str,
) -> Result<BTreeMap<String, BTreeMap<String, i64>>, StoreError> {
    let def = elsewhere_def();
    let days = crate::log::log_days(store, &def, from, to, false)?;
    Ok(days
        .days
        .into_iter()
        .map(|d| {
            let values = def
                .metrics
                .iter()
                .filter_map(|m| {
                    let s = d.sums.get(&m.key)?;
                    (s.n > 0).then(|| {
                        let v = if m.agg == crate::views::Agg::Mean {
                            s.sum / s.n as i64
                        } else {
                            s.sum
                        };
                        (m.key.clone(), v)
                    })
                })
                .collect();
            (d.day, values)
        })
        .collect())
}

/// What a span holds for one tracker: the days it covers, the days recorded, and the days at or
/// above 1. For a severity, also the days at or above 2, which IHS calls *moderate or severe*.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HealthCount {
    pub of: usize,
    pub recorded: usize,
    pub days: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderate_or_severe: Option<usize>,
    /// Values on the tracker's key that are not a whole number its kind accepts. Not recorded,
    /// and said here so they are not lost without a word.
    #[serde(skip_serializing_if = "is_zero")]
    pub odd: usize,
}
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// One ISO week of one tracker, for Over time: the days of it inside the span, the days recorded,
/// the days at or above 1, and the recorded values added up. **A mean is `sum / recorded`**, worked
/// out where it is shown; a week with three recorded days says so, and is not read as seven.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HealthWeek {
    /// The Monday it starts on.
    pub week: String,
    pub of: usize,
    pub recorded: usize,
    pub days: usize,
    pub sum: i64,
}

#[derive(Debug, Serialize)]
pub struct HealthDays {
    /// The Health template a new tracker is written into: the oldest, when there are several.
    /// `None` until the person tracks something.
    pub template: Option<String>,
    pub trackers: Vec<Tracker>,
    /// Every day of the span, in order, recorded or not.
    pub days: Vec<HealthDay>,
    pub counts: BTreeMap<String, HealthCount>,
    /// Per tracker, its weeks in order, when they were asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weeks: Option<BTreeMap<String, Vec<HealthWeek>>>,
}

/// More days than this in one ask is a mistake, not a month.
const MAX_DAYS: i64 = 400;

fn parse_day(which: &str, s: &str) -> Result<Date, StoreError> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DATE)
        .map_err(|_| StoreError::Parse(format!("{which} must be a day like 2026-07-20, not '{s}'")))
}
fn day_string(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// **The diary over a span**: what the person tracks, every day with what was recorded on it, and
/// the counts the month shows. The counting is `fm_query::days`'s; nothing here labels a day.
pub fn health_days(
    store: &dyn Store,
    from: &str,
    to: &str,
    by_week: bool,
) -> Result<HealthDays, StoreError> {
    let (from, to) = (parse_day("from", from)?, parse_day("to", to)?);
    if to < from || (to - from).whole_days() >= MAX_DAYS {
        return Err(StoreError::Parse(format!(
            "from {} to {} is not a span of at most {MAX_DAYS} days",
            day_string(from),
            day_string(to)
        )));
    }
    let templates = templates(store)?;
    let trackers = of_templates(&templates);
    let template = templates.first().map(|o| o.id.to_string());
    let (series, mut notes) = read(store, &trackers, from, to)?;
    let mut others = elsewhere(store, &day_string(from), &day_string(to))?;
    let mut days = Vec::new();
    let mut d = from;
    loop {
        let here = notes.remove(&d).unwrap_or_default();
        let values = trackers
            .iter()
            .filter_map(|t| series[&t.key].0.get(&d).map(|v| (t.key.clone(), *v)))
            .collect();
        let late = here.first().is_some_and(|o| o.created.date() > d + time::Duration::days(1));
        let context = here.iter().find_map(|o| match o.get(crate::cycle::CONTEXT) {
            PropertyValue::Text(t) if !t.trim().is_empty() => Some(t.trim().to_string()),
            _ => None,
        });
        days.push(HealthDay {
            day: day_string(d),
            id: here.first().map(|o| o.id.to_string()),
            values,
            late,
            context,
            elsewhere: others.remove(&day_string(d)).unwrap_or_default(),
        });
        match d.next_day() {
            Some(n) if n <= to => d = n,
            _ => break,
        }
    }
    let counts = trackers
        .iter()
        .map(|t| {
            let (s, odd) = &series[&t.key];
            let c = fm_query::days::count(s, from, to, 1);
            let strong = (t.kind == TrackerKind::Severity)
                .then(|| fm_query::days::count(s, from, to, 2).at_least);
            let count = HealthCount {
                of: c.of,
                recorded: c.recorded,
                days: c.at_least,
                moderate_or_severe: strong,
                odd: *odd,
            };
            (t.key.clone(), count)
        })
        .collect();
    let weeks = by_week.then(|| {
        trackers
            .iter()
            .map(|t| {
                let w = fm_query::days::per_week(&series[&t.key].0, from, to, 1)
                    .into_iter()
                    .map(|(monday, c)| HealthWeek {
                        week: day_string(monday),
                        of: c.of,
                        recorded: c.recorded,
                        days: c.at_least,
                        sum: c.sum,
                    })
                    .collect();
                (t.key.clone(), w)
            })
            .collect()
    });
    Ok(HealthDays { template, trackers, days, counts, weeks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::MemoryStore;
    use fm_model::Stamp;
    use time::Month;

    fn day(d: u8) -> Date {
        Date::from_calendar_date(2026, Month::October, d).unwrap()
    }

    #[test]
    fn a_keys_suffix_says_what_kind_of_tracker_it_is() {
        assert_eq!(parse_key("headache_0to3"), Some(("headache", TrackerKind::Severity)));
        assert_eq!(parse_key("pain_0to10"), Some(("pain", TrackerKind::Rating)));
        assert_eq!(parse_key("nose_0to100"), Some(("nose", TrackerKind::Bother)));
        assert_eq!(parse_key("bedding_washed_yes"), Some(("bedding_washed", TrackerKind::YesNo)));
        assert_eq!(parse_key("ibuprofen_doses"), Some(("ibuprofen", TrackerKind::Count)));
        assert_eq!(parse_key("nose_rinse_count"), Some(("nose_rinse", TrackerKind::Count)));
        assert_eq!(parse_key("sleep_min"), Some(("sleep", TrackerKind::Amount { unit: "min" })));
        assert_eq!(
            parse_key("bp_sys_mmhg"),
            Some(("bp_sys", TrackerKind::Amount { unit: "mmhg" }))
        );
        assert_eq!(parse_key("snot22_score"), Some(("snot22", TrackerKind::Score)));
    }

    #[test]
    fn a_key_with_no_known_suffix_or_no_name_is_not_a_tracker() {
        for k in ["status", "headache", "mood_0to5", "weight_kg", "_yes", "x__yes", "Head_0to3"] {
            assert_eq!(parse_key(k), None, "{k}");
        }
    }

    #[test]
    fn a_label_is_the_keys_name_in_words() {
        assert_eq!(label_of("bedding_washed"), "Bedding washed");
        assert_eq!(label_of("snot22"), "Snot22");
    }

    #[test]
    fn a_kind_bounds_what_may_be_recorded() {
        assert!(TrackerKind::Severity.accepts(3) && !TrackerKind::Severity.accepts(4));
        assert!(!TrackerKind::YesNo.accepts(-1));
        assert!(TrackerKind::Amount { unit: "min" }.accepts(1_000));
    }

    fn note(body: &str, f: impl FnOnce(&mut Object)) -> Object {
        let mut o = Object::new(Kind::Note, body);
        f(&mut o);
        o
    }

    fn entry(body: &str, d: u8, kv: &[(&str, PropertyValue)]) -> Object {
        note(body, |o| {
            o.extra.insert(LOG.into(), PropertyValue::Text(HEALTH.into()));
            o.start = Some(Stamp::day(day(d)));
            // Written on its own day, as a diary is kept.
            o.created = time::OffsetDateTime::new_utc(day(d), time::Time::MIDNIGHT);
            for (k, v) in kv {
                o.extra.insert((*k).into(), v.clone());
            }
        })
    }

    fn store(objects: Vec<Object>) -> MemoryStore {
        let mut s = MemoryStore::new();
        for o in objects {
            s.put(&o).unwrap();
        }
        s
    }

    #[test]
    fn what_a_person_tracks_is_the_tracker_keys_on_their_health_templates() {
        let template = note("t1", |o| {
            o.tags = vec!["template".into()];
            o.extra.insert(GROUP.into(), PropertyValue::Text(GROUP_NAME.into()));
            o.extra.insert("headache_0to3".into(), PropertyValue::Int(0));
            o.extra.insert("bedding_washed_yes".into(), PropertyValue::Int(0));
            o.extra.insert("not_a_tracker".into(), PropertyValue::Int(0));
        });
        // A Food template's keys are not Health trackers, nor is an ordinary note's.
        let food = note("t2", |o| {
            o.tags = vec!["template".into()];
            o.extra.insert(GROUP.into(), PropertyValue::Text("Food".into()));
            o.extra.insert("coffee_yes".into(), PropertyValue::Int(0));
        });
        let plain = note("n1", |o| {
            o.extra.insert(GROUP.into(), PropertyValue::Text(GROUP_NAME.into()));
            o.extra.insert("tea_yes".into(), PropertyValue::Int(0));
        });
        let s = store(vec![template, food, plain]);
        let keys: Vec<String> = trackers(&s).unwrap().into_iter().map(|t| t.key).collect();
        assert_eq!(keys, vec!["bedding_washed_yes", "headache_0to3"]);
    }

    #[test]
    fn a_day_is_recorded_only_where_a_value_was_written_and_a_bad_value_is_counted_not_dropped() {
        let ts = vec![
            Tracker {
                key: "headache_0to3".into(),
                label: "Headache".into(),
                kind: TrackerKind::Severity,
            },
            Tracker {
                key: "ibuprofen_doses".into(),
                label: "Ibuprofen".into(),
                kind: TrackerKind::Count,
            },
        ];
        let s = store(vec![
            entry("e1", 1, &[("headache_0to3", PropertyValue::Int(0))]),
            entry(
                "e2",
                2,
                &[
                    ("headache_0to3", PropertyValue::Int(2)),
                    ("ibuprofen_doses", PropertyValue::Int(1)),
                ],
            ),
            // A second note on the same day, from another device: the peak and the sum.
            entry(
                "e3",
                2,
                &[
                    ("headache_0to3", PropertyValue::Int(3)),
                    ("ibuprofen_doses", PropertyValue::Int(1)),
                ],
            ),
            entry("e4", 3, &[("headache_0to3", PropertyValue::Int(7))]),
            entry("e5", 4, &[("headache_0to3", PropertyValue::Text("bad".into()))]),
            // Outside the span.
            entry("e6", 20, &[("headache_0to3", PropertyValue::Int(1))]),
        ]);
        let got = series(&s, &ts, day(1), day(10)).unwrap();
        let (headache, odd) = &got["headache_0to3"];
        assert_eq!(
            headache.iter().map(|(d, v)| (d.day(), *v)).collect::<Vec<_>>(),
            vec![(1, 0), (2, 3)]
        );
        assert_eq!(*odd, 2, "7 is out of 0–3, and 'bad' is not a number");
        assert_eq!(got["ibuprofen_doses"].0[&day(2)], 2);
        let c = fm_query::days::count(headache, day(1), day(10), 1);
        assert_eq!((c.of, c.recorded, c.at_least), (10, 2, 1));
    }

    #[test]
    fn the_diary_over_a_span_says_every_day_and_counts_without_labelling() {
        let template = note("t", |o| {
            o.tags = vec!["template".into()];
            o.extra.insert(GROUP.into(), PropertyValue::Text(GROUP_NAME.into()));
            o.extra.insert("headache_0to3".into(), PropertyValue::Int(0));
            o.extra.insert("bedding_washed_yes".into(), PropertyValue::Int(0));
        });
        let late = {
            let mut o = entry("e3", 3, &[("headache_0to3", PropertyValue::Int(1))]);
            o.created = time::OffsetDateTime::new_utc(day(9), time::Time::MIDNIGHT);
            o
        };
        let s = store(vec![
            template,
            entry(
                "e1",
                1,
                &[
                    ("headache_0to3", PropertyValue::Int(2)),
                    ("bedding_washed_yes", PropertyValue::Int(1)),
                ],
            ),
            entry("e2", 2, &[("headache_0to3", PropertyValue::Int(0))]),
            late,
        ]);
        let got = health_days(&s, "2026-10-01", "2026-10-05", true).unwrap();
        assert_eq!(got.trackers.len(), 2);
        assert!(got.template.is_some());
        assert_eq!(got.days.len(), 5);
        assert!(got.days[0].id.is_some() && got.days[3].id.is_none());
        assert_eq!(got.days[0].values["bedding_washed_yes"], 1);
        assert!(got.days[3].values.is_empty(), "a day with no note records nothing");
        assert!(got.days[2].late && !got.days[0].late);
        let h = &got.counts["headache_0to3"];
        assert_eq!((h.of, h.recorded, h.days, h.moderate_or_severe), (5, 3, 2, Some(1)));
        assert_eq!(got.counts["bedding_washed_yes"].moderate_or_severe, None);
        // 2026-10-01 is a Thursday: the span is two part weeks, and each says how many days it had.
        let weeks = &got.weeks.as_ref().unwrap()["headache_0to3"];
        assert_eq!(
            weeks
                .iter()
                .map(|w| (w.week.as_str(), w.of, w.recorded, w.days, w.sum))
                .collect::<Vec<_>>(),
            vec![("2026-09-28", 4, 3, 2, 3), ("2026-10-05", 1, 0, 0, 0)]
        );
        assert!(health_days(&s, "2026-10-01", "2026-10-05", false).unwrap().weeks.is_none());
        assert!(health_days(&s, "2026-10-05", "2026-10-01", false).is_err());
    }

    #[test]
    fn what_food_and_the_body_log_hold_is_read_into_the_days_not_typed_again() {
        let logged = |log: &str, d: u8, kv: &[(&str, i64)]| {
            note("", |o| {
                o.extra.insert(LOG.into(), PropertyValue::Text(log.into()));
                o.start = Some(Stamp::day(day(d)));
                for (k, v) in kv {
                    o.extra.insert((*k).into(), PropertyValue::Int(*v));
                }
            })
        };
        let s = store(vec![
            logged("food", 2, &[("water_mg", 1_500_000), ("caffeine_ug", 95_000)]),
            logged("food", 2, &[("water_mg", 250_000)]),
            logged("body", 2, &[("weight_g", 71_000)]),
            logged("body", 2, &[("weight_g", 71_400)]),
        ]);
        let got = health_days(&s, "2026-10-01", "2026-10-03", false).unwrap();
        assert!(got.days[0].elsewhere.is_empty());
        let e = &got.days[1].elsewhere;
        assert_eq!(e["water_mg"], 1_750_000, "added up, foods and drinks alike");
        assert_eq!(e["caffeine_ug"], 95_000);
        assert_eq!(e["weight_g"], 71_200, "a weight is a mean, never a sum");
        assert!(!e.contains_key("alcohol_mg"), "nothing logged is not a zero");
    }
}
