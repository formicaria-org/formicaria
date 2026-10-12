//! Diets and training regimes to follow: a library of sourced options, a person's own measurements,
//! and how a week measures up to what was chosen.
//!
//! `decisions.md#data` and `#ui`, 2026-10-12, *a diet or a regime is chosen from a library and
//! followed as a program note*. Four things live here, each behind one `dispatch` arm, so the UI,
//! `fm-cli`, curl and the assistant reach them the same way and none of it needs the assistant:
//!
//! - **The library** (`shipped/plans.json`): what can be followed. Carried in the app like the
//!   shipped views, never written into a vault. **Every number in it names its source** (the
//!   2026-10-11 ruling *nothing is invented*); the register is `sources-2026-10-11.md`.
//! - **The profile**: a person's sex, year of birth, height, weight and activity level, a note in
//!   their vault found by its `profile` property. Formulas run on it only when asked.
//! - **A preview**: the library's option with its numbers filled in for this person, which the
//!   person may change before following — that is the fine-tuning, and "My own" is the same form
//!   with nothing filled in.
//! - **Progress**: the week holding a day, measured against the plan in force.
//!
//! **Following a plan is a note.** It is an ordinary `program: food` or `program: training` note —
//! the kind `log.rs` already reads targets from — that also names the library option (`plan`) and
//! carries every bound it holds the person to (`p_<check>_min`, `p_<check>_max`). So a plan is
//! readable without the app, a change of numbers is a new note and leaves earlier weeks measured
//! against what they had, and stopping is giving the note a last day.

use crate::dto::ObjectMeta;
use crate::log::{PLANNED, PROGRAM};
use crate::thread::{is_message, is_proposal, is_template, plan_base, LOG};
use fm_core::{Store, StoreError};
use fm_model::{Object, PropertyValue};
use fm_query::{Query, SortKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::{format_description::well_known::Iso8601, Date, Duration};

/// The library, as shipped.
const LIBRARY: &str = include_str!("shipped/plans.json");

/// The note carrying this is the person's own measurements.
pub const PROFILE: &str = "profile";
/// A program note names the library option it follows under this key.
pub const PLAN: &str = "plan";
/// A day the person says they kept to their plan is an entry of this log.
pub const KEPT: &str = "kept";
/// The log a weight is written down in.
pub const BODY: &str = "body";
/// What an activity entry counts as: `aerobic`, `strength`, `balance` or `flexibility`. Not
/// `kind`, which every note already answers with what it is.
pub const COUNTS_AS: &str = "counts_as";
/// The energy need a plan was set from, kept on its note so a fasting day is measured against
/// the figure the person saw and not one recomputed later.
pub const ENERGY: &str = "p_energy";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Source {
    pub by: String,
    pub title: String,
    #[serde(default)]
    pub url: String,
}

/// Where to see how something is done: a page or a video by a known body. A link and a title
/// only; nothing of theirs is carried in the app. `seen` is the day it was last seen to work.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Guide {
    pub label: String,
    pub by: String,
    pub url: String,
    pub seen: String,
}

/// One thing a plan holds a person to, and how it is measured.
///
/// `kind` is one of a closed set, each measured by one arm of [`measure`]:
/// - `sum`: a value's total for a day (`key`), in the key's own unit.
/// - `share`: the part of a day's energy that came from `key`, in percent.
/// - `low_days`: how many days of the week came to `pct` percent of the energy need or less.
/// - `minutes`: the week's activity, in moderate minutes; a vigorous minute counts as two.
/// - `days`: how many days of the week had activity `of` a kind.
/// - `sets`: how many sets the week had in `regions`.
/// - `region_days`: on how many days each of `regions` was trained — the least of them.
/// - `named_days`: how many days of the week had an activity logged under one of `names` — a
///   protocol known by its name (the Norwegian 4×4) is followed by doing it and saying so.
/// - `kept`: how many days of the week the person said they kept to it.
/// - `fast`: the minutes without eating that ended on a day — from the last entry with energy
///   the day before to the first that day, by the times the entries carry.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Check {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kcal_per_g: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pct: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub of: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    /// For `named_days`: the activities, by the name they are logged under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// The bounds the source gives, when they are the same for everybody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// …and when they depend on the person: by sex, by being under 25, by body weight, or as a
    /// share of the energy estimate. Read by [`preview`], never by [`measure`].
    #[serde(default, skip_serializing)]
    min_male: Option<i64>,
    #[serde(default, skip_serializing)]
    min_female: Option<i64>,
    #[serde(default, skip_serializing)]
    min_young: Option<i64>,
    #[serde(default, skip_serializing)]
    min_mg_per_kg: Option<i64>,
    #[serde(default, skip_serializing)]
    max_energy_pct: Option<i64>,
    /// Whose number it is, in the source's words.
    pub basis: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Plan {
    pub id: String,
    /// `food` or `fitness`.
    pub group: String,
    pub name: String,
    /// What kind of source it rests on: `guideline`, `position`, `review`, `definition`,
    /// `described` (a diet written down by its body that the app cannot measure) or `own`.
    pub rests_on: String,
    pub summary: String,
    pub sources: Vec<Source>,
    /// The profile fields its numbers are worked out from.
    #[serde(default)]
    pub needs: Vec<String>,
    pub checks: Vec<Check>,
    /// What the body's own text asks for, where the app can only show it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub describes: Vec<String>,
    /// What following it asks for that the app cannot measure, and why.
    #[serde(default)]
    pub unmeasured: Vec<String>,
    /// A statement by a body about who this is not for, quoted instead of screening anybody.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub caution: String,
    #[serde(default)]
    pub guides: Vec<Guide>,
}

/// An activity a known body names, and what it counts as.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Activity {
    pub name: String,
    pub kind: String,
    pub by: String,
}

/// A week laid out day by day, as a public body publishes it — a place to start, word for word.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExampleWeeks {
    pub by: String,
    pub title: String,
    pub url: String,
    pub seen: String,
    pub examples: Vec<ExampleWeek>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExampleWeek {
    pub name: String,
    pub about: String,
    /// Sunday to Saturday, as the source lists them; an empty day is a day with nothing.
    pub days: Vec<String>,
    pub total: String,
}

#[derive(Debug, Deserialize)]
struct Library {
    #[serde(default)]
    weeks: Vec<ExampleWeeks>,
    activities: Vec<Activity>,
    plans: Vec<Plan>,
}

fn library() -> &'static Library {
    static LIB: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    LIB.get_or_init(|| serde_json::from_str(LIBRARY).expect("the shipped plan library parses"))
}

/// The log a group's entries are in, and the value its program notes carry.
fn kinds_of(group: &str) -> Result<(&'static str, &'static str), StoreError> {
    match group {
        "food" => Ok(("food", "food")),
        "fitness" => Ok(("workout", "training")),
        other => Err(StoreError::Parse(format!("no such group of plans: '{other}'"))),
    }
}

#[derive(Debug, Serialize)]
pub struct LibraryAnswer {
    pub plans: Vec<Plan>,
    /// Activities a known body names, with what each counts as. Only for `fitness`.
    pub activities: Vec<Activity>,
    /// Weeks laid out day by day by a public body. Only for `fitness`.
    pub weeks: Vec<ExampleWeeks>,
}

/// What can be followed in a group, in the order it is offered.
pub fn plan_library(group: &str) -> Result<LibraryAnswer, StoreError> {
    kinds_of(group)?;
    let lib = library();
    Ok(LibraryAnswer {
        plans: lib.plans.iter().filter(|p| p.group == group).cloned().collect(),
        activities: if group == "fitness" { lib.activities.clone() } else { Vec::new() },
        weeks: if group == "fitness" { lib.weeks.clone() } else { Vec::new() },
    })
}

fn plan_named(id: &str) -> Result<&'static Plan, StoreError> {
    library()
        .plans
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| StoreError::Parse(format!("no plan called '{id}' in this version")))
}

// ── the profile, and the two formulas run on it ──────────────────────────────────────────────────

/// A person's measurements, as far as they have given them.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct Profile {
    /// The note they are kept in, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `female` or `male`: the two the equations are published for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex: Option<String>,
    /// The year of birth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub born: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_cm: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_g: Option<i64>,
    /// EFSA's activity level times ten: 14, 16, 18 or 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pal_x10: Option<i64>,
}

impl Profile {
    fn of(o: &Object) -> Profile {
        let int = |k: &str| match o.get(k) {
            PropertyValue::Int(v) => Some(v),
            _ => None,
        };
        Profile {
            id: Some(o.id.to_string()),
            sex: match o.get("sex") {
                PropertyValue::Text(s) if s == "female" || s == "male" => Some(s),
                _ => None,
            },
            born: int("born"),
            height_cm: int("height_cm"),
            weight_g: int("weight_g"),
            pal_x10: int("pal_x10"),
        }
    }
    fn has(&self, field: &str) -> bool {
        match field {
            "sex" => self.sex.is_some(),
            "born" => self.born.is_some(),
            "height_cm" => self.height_cm.is_some(),
            "weight_g" => self.weight_g.is_some(),
            "pal_x10" => self.pal_x10.is_some(),
            _ => true,
        }
    }
    fn age_in(&self, year: i32) -> Option<i64> {
        self.born.map(|b| i64::from(year) - b)
    }
}

/// The person's measurements: the profile note changed most recently, or an empty profile.
///
/// **The weight is the last one written down.** A weight is logged as an entry of the body log, so
/// it can be a trend; the latest of those is the person's weight, and the profile note's own
/// figure stands only until there is one.
pub fn plan_profile(store: &dyn Store) -> Result<Profile, StoreError> {
    let q = Query { filter: plan_base(&[BODY], &[PROFILE]), ..Default::default() };
    let rows = store.query(&q)?.rows;
    let weighed = |o: &&Object| matches!(o.get("log"), PropertyValue::Text(k) if k == BODY);
    let mut profile = rows
        .iter()
        .filter(|o| !weighed(o) && !o.get(PROFILE).is_null())
        .filter(|o| !is_template(o) && !is_message(o) && !is_proposal(o))
        .max_by_key(|o| o.updated)
        .map(Profile::of)
        .unwrap_or_default();
    let last = rows
        .iter()
        .filter(weighed)
        .filter_map(|o| match (o.start, o.get("weight_g")) {
            (Some(s), PropertyValue::Int(g)) if g > 0 => Some(((s.date, o.updated), g)),
            _ => None,
        })
        .max_by_key(|(at, _)| *at);
    if let Some((_, g)) = last {
        profile.weight_g = Some(g);
    }
    Ok(profile)
}

/// Basal metabolic rate in kcal a day, by Henry's equations for adults (Henry 2005, *Public Health
/// Nutr* 8(7A):1133–52, Table 15 — weight in kg, height in metres; the equations EFSA adopted in
/// 2013). `None` under 18: the adult rows are the ones read and re-read at the source.
///
/// In whole grams and centimetres so nothing is rounded before the end: the published
/// coefficients are scaled by 1000 and the sum divided once.
pub fn henry_bmr(sex: &str, age: i64, weight_g: i64, height_cm: i64) -> Option<i64> {
    // (per kg, per metre, constant), each ×1000 where the paper gives decimals.
    let (w, h, c): (i64, i64, i64) = match (sex, age) {
        (_, a) if a < 18 => return None,
        ("male", a) if a < 30 => (14_400, 313_000, 113_000),
        ("male", a) if a < 60 => (11_400, 541_000, -137_000),
        ("male", _) => (11_400, 541_000, -256_000),
        ("female", a) if a < 30 => (10_400, 615_000, -282_000),
        ("female", a) if a < 60 => (8_180, 502_000, -11_600),
        ("female", _) => (8_520, 421_000, 10_700),
        _ => return None,
    };
    // w × kg + h × m + c, with kg = g/1000 and m = cm/100, all over the 1000 the terms carry.
    let thousandths = w * weight_g / 1000 + h * height_cm / 100 + c;
    Some((thousandths + 500) / 1000)
}

/// A day's energy need in kcal: Henry's basal rate times EFSA's activity level (1.4, 1.6, 1.8 or
/// 2.0 — "a rough indication", in EFSA's words). `None` unless all five measurements are there.
pub fn energy_need(p: &Profile, year: i32) -> Option<i64> {
    let pal = p.pal_x10.filter(|x| [14, 16, 18, 20].contains(x))?;
    let bmr = henry_bmr(p.sex.as_deref()?, p.age_in(year)?, p.weight_g?, p.height_cm?)?;
    Some((bmr * pal + 5) / 10)
}

// ── a plan with its numbers filled in ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct Bound {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct Preview {
    pub plan: Plan,
    pub profile: Profile,
    /// The profile fields this plan needs and does not have yet.
    pub missing: Vec<String>,
    /// The energy estimate, when the plan uses one and the profile allows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy: Option<i64>,
    /// Each check's bounds as the source gives them for this person. A person may change any of
    /// them before following.
    pub bounds: BTreeMap<String, Bound>,
}

fn bounds_for(check: &Check, p: &Profile, year: i32, energy: Option<i64>) -> Bound {
    let mut b = Bound { min: check.min, max: check.max };
    match p.sex.as_deref() {
        Some("male") if check.min_male.is_some() => b.min = check.min_male,
        Some("female") if check.min_female.is_some() => b.min = check.min_female,
        _ => {}
    }
    if check.min_male.is_some() && p.sex.is_none() {
        b.min = None;
    }
    if let (Some(young), Some(age)) = (check.min_young, p.age_in(year)) {
        if (18..25).contains(&age) {
            b.min = Some(young);
        }
    }
    if let Some(per_kg) = check.min_mg_per_kg {
        b.min = p.weight_g.map(|g| per_kg * g / 1000);
    }
    if let Some(pct) = check.max_energy_pct {
        b.max = energy.map(|e| e * pct / 100);
    }
    b
}

/// A library option with its numbers worked out for the person whose profile is in the vault.
pub fn plan_preview(store: &dyn Store, id: &str, today: &str) -> Result<Preview, StoreError> {
    let plan = plan_named(id)?;
    let profile = plan_profile(store)?;
    let year = parse_day("day", today)?.year();
    let missing: Vec<String> = plan.needs.iter().filter(|f| !profile.has(f)).cloned().collect();
    let uses_energy = plan.needs.iter().any(|f| f == "pal_x10");
    let energy = if uses_energy { energy_need(&profile, year) } else { None };
    let bounds =
        plan.checks.iter().map(|c| (c.id.clone(), bounds_for(c, &profile, year, energy))).collect();
    Ok(Preview { plan: plan.clone(), profile, missing, energy, bounds })
}

// ── a week, measured ─────────────────────────────────────────────────────────────────────────────

fn parse_day(which: &str, s: &str) -> Result<Date, StoreError> {
    Date::parse(s, &Iso8601::DATE)
        .map_err(|_| StoreError::Parse(format!("{which} must be a day like 2026-07-20, not '{s}'")))
}
fn day_string(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CheckProgress {
    pub id: String,
    pub label: String,
    pub kind: String,
    /// What the numbers are in: a value's own key (`fiber_mg`), `percent`, `minutes`, `days` or
    /// `sets`.
    pub unit: String,
    /// `day` for a check asked of every day, `week` for one asked of the week as a whole.
    pub per: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// The day asked about (for `day`) or the week so far (for `week`). `None` when there is
    /// nothing to measure: a day with nothing logged is not a day of zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
    /// `met`, `under`, `over`, or `none` when there is nothing to measure.
    pub state: String,
    /// Monday to Sunday: the value each day had, for a `day` check; for a `week` check, what each
    /// day added. `None` where a day had nothing.
    pub days: Vec<Option<i64>>,
    pub basis: String,
}

#[derive(Debug, Serialize)]
pub struct Progress {
    /// The plan in force on the day, or `None` when the person follows none in this group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<Plan>,
    /// The note that says so: its id, its title, the day it was followed from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<ObjectMeta>,
    /// A program note names a plan this version does not have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown: Option<String>,
    /// The Monday and the Sunday of the week measured.
    pub from: String,
    pub to: String,
    pub day: String,
    pub checks: Vec<CheckProgress>,
    /// The note that says the person kept to it on `day`, when they said so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept_today: Option<String>,
}

fn int_of(o: &Object, key: &str) -> Option<i64> {
    match o.get(key) {
        PropertyValue::Int(v) => Some(v),
        _ => None,
    }
}
fn text_of(o: &Object, key: &str) -> String {
    match o.get(key) {
        PropertyValue::Text(s) => s,
        _ => String::new(),
    }
}
fn is_planned(o: &Object) -> bool {
    !matches!(
        o.get(PLANNED),
        PropertyValue::Null | PropertyValue::Bool(false) | PropertyValue::Int(0)
    )
}

/// What an entry of the training log counts as: a set of an exercise is strengthening; an
/// activity is what its `counts_as` says.
fn counts_as(o: &Object) -> String {
    let kind = text_of(o, COUNTS_AS);
    if !kind.is_empty() {
        kind
    } else if int_of(o, "reps").is_some() || !text_of(o, "exercise").is_empty() {
        "strength".into()
    } else {
        "aerobic".into()
    }
}

/// Moderate minutes an activity entry is worth. WHO (2020) puts moderate intensity at "a 5 or 6"
/// and vigorous at "a 7 or 8" on a scale of 0–10, and lets a vigorous minute stand for two
/// moderate ones (150 moderate or 75 vigorous). Effort under 5 is lighter than either and counts
/// for nothing here; above 8 is counted as vigorous.
fn moderate_minutes(o: &Object) -> i64 {
    let minutes = int_of(o, "minutes").unwrap_or(0).max(0);
    match int_of(o, "effort") {
        Some(e) if e >= 7 => minutes * 2,
        Some(e) if e >= 5 => minutes,
        _ => 0,
    }
}

struct Week<'a> {
    days: [Date; 7],
    /// The counted entries of the group's log, by day of the week.
    entries: [Vec<&'a Object>; 7],
    kept: [bool; 7],
    /// The counted entries of the day before the week's Monday: a night's fast starts there.
    eve: Vec<&'a Object>,
    /// Up to which day of the week there is anything to say (the day asked about).
    upto: usize,
}

fn measure(check: &Check, bound: Bound, energy: Option<i64>, w: &Week) -> CheckProgress {
    let sum_key = |day: usize, key: &str| -> Option<i64> {
        let got: Vec<i64> = w.entries[day].iter().filter_map(|o| int_of(o, key)).collect();
        (!w.entries[day].is_empty()).then(|| got.iter().sum())
    };
    let (unit, per, days): (String, &str, Vec<Option<i64>>) = match check.kind.as_str() {
        "sum" => (check.key.clone(), "day", (0..7).map(|d| sum_key(d, &check.key)).collect()),
        "share" => {
            let factor = check.kcal_per_g.unwrap_or(0);
            let days = (0..7)
                .map(|d| {
                    let kcal = sum_key(d, "kcal").filter(|k| *k > 0)?;
                    let mg = sum_key(d, &check.key)?;
                    // mg × kcal/g ÷ 1000 is kcal from this value; over the day's kcal, in percent,
                    // rounded to the nearest whole.
                    Some((mg * factor / 10 + kcal / 2) / kcal)
                })
                .collect();
            ("percent".into(), "day", days)
        }
        "low_days" => {
            let limit = energy.map(|e| e * check.pct.unwrap_or(0) / 100);
            let days = (0..7)
                .map(|d| {
                    let kcal = sum_key(d, "kcal")?;
                    Some(i64::from(limit.is_some_and(|l| kcal <= l)))
                })
                .collect();
            ("days".into(), "week", days)
        }
        "minutes" => (
            "minutes".into(),
            "week",
            (0..7)
                .map(|d| {
                    let m: i64 = w.entries[d].iter().map(|o| moderate_minutes(o)).sum();
                    (m > 0).then_some(m)
                })
                .collect(),
        ),
        "days" => (
            "days".into(),
            "week",
            (0..7)
                .map(|d| {
                    let any =
                        w.entries[d].iter().any(|o| check.of == "any" || counts_as(o) == check.of);
                    any.then_some(1)
                })
                .collect(),
        ),
        "sets" => (
            "sets".into(),
            "week",
            (0..7)
                .map(|d| {
                    let n = w.entries[d]
                        .iter()
                        .filter(|o| check.regions.contains(&text_of(o, "region")))
                        .count() as i64;
                    (n > 0).then_some(n)
                })
                .collect(),
        ),
        "region_days" => (
            "days".into(),
            "week",
            // What a day adds is not a number here (the value is the least over the regions), so
            // a day is marked when it trained any of them.
            (0..7)
                .map(|d| {
                    w.entries[d]
                        .iter()
                        .any(|o| check.regions.contains(&text_of(o, "region")))
                        .then_some(1)
                })
                .collect(),
        ),
        "named_days" => (
            "days".into(),
            "week",
            (0..7)
                .map(|d| {
                    let named = |o: &&Object| {
                        let said = [text_of(o, "activity"), o.title.clone().unwrap_or_default()];
                        check.names.iter().any(|n| said.iter().any(|s| s.eq_ignore_ascii_case(n)))
                    };
                    w.entries[d].iter().any(named).then_some(1)
                })
                .collect(),
        ),
        "kept" => ("days".into(), "week", w.kept.iter().map(|k| k.then_some(1)).collect()),
        // The minutes from the last thing eaten the day before to the first thing eaten that
        // day. Only entries with a time, and with energy: "calorie containing (≥5 kcal)" is what
        // an eating event is in Gill and Panda 2015, and the consensus definition counts "food
        // intake and the consumption of caloric beverages". A day or an eve with no such entry
        // has nothing to measure.
        "fast" => {
            let eats = |list: &[&Object]| -> Vec<i64> {
                list.iter()
                    .filter(|o| int_of(o, "kcal").is_some_and(|k| k >= 5))
                    .filter_map(|o| crate::log::minute_of(o))
                    .collect()
            };
            let days = (0..7)
                .map(|d| {
                    let before = if d == 0 { eats(&w.eve) } else { eats(&w.entries[d - 1]) };
                    let last = before.into_iter().max()?;
                    let first = eats(&w.entries[d]).into_iter().min()?;
                    Some(24 * 60 - last + first)
                })
                .collect();
            ("duration".into(), "day", days)
        }
        _ => (String::new(), "week", vec![None; 7]),
    };

    let value = if per == "day" {
        days[w.upto]
    } else if check.kind == "region_days" {
        check
            .regions
            .iter()
            .map(|r| {
                (0..7).filter(|d| w.entries[*d].iter().any(|o| &text_of(o, "region") == r)).count()
                    as i64
            })
            .min()
    } else {
        // Up to the day asked about, as the week's own doc says: a day after it is not yet part
        // of "so far", whatever has been logged on it.
        Some(days[..=w.upto].iter().flatten().sum())
    };
    let state = match value {
        None => "none",
        Some(v) if bound.max.is_some_and(|m| v > m) => "over",
        Some(v) if bound.min.is_some_and(|m| v < m) => "under",
        Some(_) => "met",
    };
    CheckProgress {
        id: check.id.clone(),
        label: check.label.clone(),
        kind: check.kind.clone(),
        unit,
        per: per.into(),
        min: bound.min,
        max: bound.max,
        value,
        state: state.into(),
        days,
        basis: check.basis.clone(),
    }
}

/// The week holding `day`, measured against the plan in force on it.
///
/// **The plan in force** is the program note of this group that names a plan, has started, has
/// not ended, and started latest — the rule `log.rs` uses for targets, asked of the notes that
/// carry `plan`. **Only the bounds on that note are measured**: a check the person left empty is
/// not held against them, whatever the library says.
///
/// **A week is Monday to Sunday**, measured up to the day asked about. Planned entries are in
/// none of it.
pub fn plan_progress(store: &dyn Store, group: &str, day: &str) -> Result<Progress, StoreError> {
    progress(store, group, day, false)
}

/// The same, **counting what is planned as if it had happened** — what a day on the menu would
/// come to (`decisions.md#ui`, 2026-10-12, *a planned day is held against the diet too*). Never
/// what a person is told they did.
pub fn plan_outlook(store: &dyn Store, group: &str, day: &str) -> Result<Progress, StoreError> {
    progress(store, group, day, true)
}

fn progress(
    store: &dyn Store,
    group: &str,
    day: &str,
    with_planned: bool,
) -> Result<Progress, StoreError> {
    let (log, program) = kinds_of(group)?;
    let day = parse_day("day", day)?;
    let monday = day - Duration::days(i64::from(day.weekday().number_days_from_monday()));
    let sunday = monday + Duration::days(6);
    let days: [Date; 7] = std::array::from_fn(|i| monday + Duration::days(i as i64));

    let q = Query {
        filter: plan_base(&[log, KEPT], &[PROGRAM]),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let rows = store.query(&q)?.rows;
    let mut programs: Vec<&Object> = Vec::new();
    let mut entries: [Vec<&Object>; 7] = Default::default();
    let mut kept_notes: Vec<&Object> = Vec::new();
    let mut eve: Vec<&Object> = Vec::new();
    let sunday_before = monday - Duration::days(1);
    for o in &rows {
        if is_template(o) || is_message(o) || is_proposal(o) {
            continue;
        }
        let kind = text_of(o, LOG);
        if kind.is_empty() {
            if text_of(o, PROGRAM) == program && !text_of(o, PLAN).is_empty() {
                programs.push(o);
            }
            continue;
        }
        if o.start.is_some_and(|s| s.date == sunday_before)
            && kind == log
            && (with_planned || !is_planned(o))
        {
            eve.push(o);
        }
        let Some(at) = o.start.map(|s| s.date).filter(|d| *d >= monday && *d <= sunday) else {
            continue;
        };
        let i = (at - monday).whole_days() as usize;
        if kind == KEPT {
            kept_notes.push(o);
        } else if kind == log && (with_planned || !is_planned(o)) {
            entries[i].push(o);
        }
    }

    let in_force = programs
        .iter()
        .filter(|p| p.start.is_none_or(|s| s.date <= day))
        .filter(|p| p.due.is_none_or(|d| d.date >= day))
        .max_by_key(|p| (p.start.map(|s| s.date), p.updated))
        .copied();

    let mut out = Progress {
        plan: None,
        note: in_force.map(ObjectMeta::from),
        unknown: None,
        from: day_string(monday),
        to: day_string(sunday),
        day: day_string(day),
        checks: Vec::new(),
        kept_today: None,
    };
    let Some(note) = in_force else { return Ok(out) };
    let id = text_of(note, PLAN);
    let Ok(plan) = plan_named(&id) else {
        out.unknown = Some(id);
        return Ok(out);
    };

    let mut kept = [false; 7];
    for k in kept_notes.iter().filter(|k| text_of(k, PLAN) == id) {
        let at = k.start.expect("filtered to the week above").date;
        kept[(at - monday).whole_days() as usize] = true;
        if at == day {
            out.kept_today = Some(k.id.to_string());
        }
    }
    let week = Week { days, entries, kept, eve, upto: (day - monday).whole_days() as usize };
    let _ = &week.days;
    let energy = int_of(note, ENERGY);
    out.checks = plan
        .checks
        .iter()
        .filter_map(|c| {
            let bound = Bound {
                min: int_of(note, &format!("p_{}_min", c.id)),
                max: int_of(note, &format!("p_{}_max", c.id)),
            };
            (bound.min.is_some() || bound.max.is_some()).then(|| measure(c, bound, energy, &week))
        })
        .collect();
    out.plan = Some(plan.clone());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers a screen shows a person about themselves, checked against the paper's own
    /// arithmetic: a man of 35, 80 kg and 1.80 m is `11.4×80 + 541×1.8 − 137`.
    #[test]
    fn henrys_equations_are_the_published_ones() {
        assert_eq!(henry_bmr("male", 35, 80_000, 180), Some(1749)); // 912 + 973.8 − 137 = 1748.8
        assert_eq!(henry_bmr("male", 25, 70_000, 175), Some(1669)); // 1008 + 547.75 + 113
        assert_eq!(henry_bmr("male", 70, 70_000, 170), Some(1462)); // 798 + 919.7 − 256
        assert_eq!(henry_bmr("female", 25, 60_000, 165), Some(1357)); // 624 + 1014.75 − 282
        assert_eq!(henry_bmr("female", 45, 65_000, 165), Some(1348)); // 531.7 + 828.3 − 11.6
        assert_eq!(henry_bmr("female", 65, 65_000, 160), Some(1238)); // 553.8 + 673.6 + 10.7
    }

    #[test]
    fn nobody_under_eighteen_and_nobody_unnamed_gets_an_estimate() {
        assert_eq!(henry_bmr("male", 17, 60_000, 170), None);
        assert_eq!(henry_bmr("other", 30, 60_000, 170), None);
        let p = Profile {
            sex: Some("male".into()),
            born: Some(1991),
            height_cm: Some(180),
            weight_g: Some(80_000),
            pal_x10: Some(16),
            ..Default::default()
        };
        assert_eq!(energy_need(&p, 2026), Some(2798)); // 1749 × 1.6
        assert_eq!(energy_need(&Profile { pal_x10: Some(15), ..p.clone() }, 2026), None);
        assert_eq!(energy_need(&Profile { weight_g: None, ..p }, 2026), None);
    }

    /// The library is data, and data can be wrong in ways code cannot: every option must say
    /// where it comes from, and every check must be of a kind `measure` knows.
    #[test]
    fn every_option_names_a_source_and_every_check_is_measurable() {
        let lib = library();
        assert!(lib.plans.len() >= 15);
        let mut ids = std::collections::BTreeSet::new();
        for p in &lib.plans {
            assert!(ids.insert(&p.id), "{} is listed twice", p.id);
            assert!(["food", "fitness"].contains(&p.group.as_str()), "{}", p.id);
            if p.rests_on != "own" {
                assert!(!p.sources.is_empty(), "{} names no source", p.id);
                for c in &p.checks {
                    assert!(c.basis.len() > 8, "{}/{} does not say whose number it is", p.id, c.id);
                }
            }
            let mut checks = std::collections::BTreeSet::new();
            for c in &p.checks {
                assert!(checks.insert(&c.id), "{}/{} is listed twice", p.id, c.id);
                assert!(
                    [
                        "sum",
                        "share",
                        "low_days",
                        "minutes",
                        "days",
                        "sets",
                        "region_days",
                        "kept",
                        "fast",
                        "named_days",
                    ]
                    .contains(&c.kind.as_str()),
                    "{}/{} is of a kind nothing measures: {}",
                    p.id,
                    c.id,
                    c.kind
                );
                assert!(
                    c.id.chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'),
                    "{}/{} cannot be part of a property key",
                    p.id,
                    c.id
                );
            }
            for g in &p.guides {
                assert!(g.url.starts_with("https://"), "{}: {}", p.id, g.url);
                assert_eq!(g.seen.len(), 10, "{}: a guide says when it was last seen", p.id);
            }
        }
        assert!(plan_library("food").unwrap().plans.iter().any(|p| p.rests_on == "own"));
        assert!(plan_library("fitness").unwrap().plans.iter().any(|p| p.rests_on == "own"));
        assert!(plan_library("music").is_err());
    }

    #[test]
    fn a_source_s_number_becomes_this_person_s_number() {
        let efsa = plan_named("efsa-reference").unwrap();
        let check = |id: &str| efsa.checks.iter().find(|c| c.id == id).unwrap();
        let woman = Profile {
            sex: Some("female".into()),
            born: Some(2004),
            weight_g: Some(60_000),
            ..Default::default()
        };
        // 0.83 g for each kg: 49.8 g.
        assert_eq!(bounds_for(check("protein"), &woman, 2026, None).min, Some(49_800));
        assert_eq!(bounds_for(check("iron"), &woman, 2026, None).min, Some(16_000));
        // 22 years old: the higher calcium figure EFSA gives for 18–24.
        assert_eq!(bounds_for(check("calcium"), &woman, 2026, None).min, Some(1_000_000));
        let nobody = Profile::default();
        assert_eq!(bounds_for(check("iron"), &nobody, 2026, None).min, None, "no sex, no number");
        assert_eq!(bounds_for(check("protein"), &nobody, 2026, None).min, None);
        assert_eq!(bounds_for(check("fibre"), &nobody, 2026, None).min, Some(25_000));
    }
}
