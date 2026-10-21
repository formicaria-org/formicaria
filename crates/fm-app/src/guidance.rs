//! **Guidance**: the Health group's cards, and how the diary measures up to the ones a person
//! follows (`decisions.md#ui` `#data`, 2026-10-22, *a fifth group, Health*; every quote's source is
//! in `health-plan-2026-10-22.md` §5.3).
//!
//! The cards keep the shape of Food's and Fitness's (`plan.rs`): one per question, in the source's
//! own words, badged by what they rest on, with what they do not say. They are kept apart from
//! `plans.json` because what they count is different: not a day's nutrients or a week's sessions,
//! but **days of the diary** — in a week, in a month, in a row, since it was started. The counting
//! is `fm_query::days`'s.
//!
//! **A check shows the person's count beside a named body's number**, never a diagnosis word and
//! never a target worked out here. **Following a card is a note**, as in Fitness: `program: health`
//! and `plan: <card>`, from its `start`; stopping gives it a `due`. A card that asks the person to
//! sort their trackers (which painkillers are which) keeps their answer on that note
//! (`p_<choice>: key,key`), so the class of a medicine is theirs and never read off its name.

use crate::health::{parse_key, series, Tracker, HEALTH};
use crate::log::PROGRAM;
use crate::plan::{Source, PLAN};
use crate::thread::{is_message, is_proposal, is_template};
use fm_core::{Store, StoreError};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::days::{self, Series};
use fm_query::{Filter, Predicate, Query, SortKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::{Date, Duration};

const CARDS: &str = include_str!("shipped/health-cards.json");

/// A sentence a card carries, and whose it is. `nhs` marks the NHS website's own words, carried
/// under the Open Government Licence and credited as it asks.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Said {
    pub by: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nhs: bool,
}

/// Something a card asks the person to sort their own trackers into.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

/// **A closed set of ways to count**, each one arm of [`measure`]:
/// - `kept`: days since the diary was started (its first recorded day), today included;
/// - `week_days`: days of this week (from Monday) with any of the keys at 1 or more;
/// - `month_days`: the same, from the first of this month;
/// - `days_over`: days of this week with any of the keys above `value`;
/// - `at_least`: days of this week with any of the keys at `value` or more;
/// - `run`: the most days in a row with any of the keys at 1 or more, in the last `window` days;
///
/// and on the cycle (`cycle.rs`), each over the last six months as FIGO's metrics are:
/// - `cycle_lengths`: the lengths of the cycles that ended, beside `min`–`max`;
/// - `bleeding_days`: the latest period's days of bleeding in a row;
/// - `cycle_spread`: the longest cycle less the shortest, beside a number that depends on age
///   (`max_by_age`), and compared only when the person's year of birth is known;
/// - `spotting_days`: days of spotting with no flow;
/// - `heavy_days`: days of the latest period at flow 3;
/// - `around_period`: of the last three periods, how many had a headache on days −2 to +3.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Check {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    /// Keys the person chose for this card, by the id of the [`Choice`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys_from: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<i64>,
    /// The body's number: at least this…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    /// …at most this…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// …or the number at or above which the body speaks up (NICE's *"on 10 days per month or
    /// more"*), which is not the same as a most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_or_over: Option<i64>,
    /// A most that depends on age: `[from, to, most]`, ages inclusive (FIGO's regularity).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub max_by_age: Vec<[i64; 3]>,
    /// Whose number it is, in the source's words.
    pub basis: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Card {
    pub id: String,
    /// `diary`, `nose`, `daily`, `cycle`; and `own` for a card the person wrote.
    pub shelf: String,
    pub name: String,
    /// `guideline`, `consensus`, `review`, `definition`; `own` for a card the person wrote.
    pub rests_on: String,
    pub says: Vec<Said>,
    /// Shown first, before anything else on the card.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urgent: Vec<Said>,
    /// Where reputable sources disagree, said with both names.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub disagree: String,
    pub sources: Vec<Source>,
    /// The trackers following it brings into the person's Health template when missing.
    #[serde(default)]
    pub trackers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choose: Vec<Choice>,
    #[serde(default)]
    pub checks: Vec<Check>,
    /// What the card does not claim, said so nobody reads it in.
    #[serde(default)]
    pub not_said: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Library {
    nhs_credit: String,
    cards: Vec<Card>,
}

fn library() -> &'static Library {
    static LIB: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    LIB.get_or_init(|| serde_json::from_str(CARDS).expect("the shipped health cards parse"))
}

/// The value a following note carries in `program`.
pub const PROGRAM_HEALTH: &str = HEALTH;
/// **A card the person writes** (*Yours*, as in Food and Fitness): a following note with `plan:
/// own`, its title the card's name, its body the person's own words, and one check of their own —
/// `own_key` (a tracker), `own_count` (one of [`OWN_KINDS`]), `own_value` (the number a day is
/// measured against, for `days_over` and `at_least`), `own_window` (for `run`), and `own_min` or
/// `own_max`. Its number is theirs, so it needs no source; it is badged *Yours* and says so.
pub const OWN: &str = "own";
/// The ways an own card may count: the diary's own, not the cycle's.
pub const OWN_KINDS: [&str; 5] = ["week_days", "month_days", "days_over", "at_least", "run"];

fn own_card(o: &Object) -> Card {
    let text = |k: &str| match o.get(k) {
        PropertyValue::Text(t) => t.trim().to_string(),
        _ => String::new(),
    };
    let int = |k: &str| match o.get(k) {
        PropertyValue::Int(v) => Some(v),
        _ => None,
    };
    let (key, kind) = (text("own_key"), text("own_count"));
    let stem = parse_key(&key).map(|(stem, _)| crate::health::label_of(stem));
    let value = int("own_value");
    let window = int("own_window").filter(|w| *w > 0).unwrap_or(30);
    let check = stem.as_ref().filter(|_| OWN_KINDS.contains(&kind.as_str())).map(|what| {
        let label = match kind.as_str() {
            "week_days" => format!("Days this week with {what}"),
            "month_days" => format!("Days this month with {what}"),
            "days_over" => format!("Days this week with {what} above {}", value.unwrap_or(0)),
            "at_least" => format!("Days this week with {what} at {} or more", value.unwrap_or(1)),
            _ => format!("Most days in a row with {what}, last {window} days"),
        };
        Check {
            id: OWN.into(),
            label,
            kind: kind.clone(),
            keys: vec![key.clone()],
            keys_from: Vec::new(),
            value,
            window: (kind == "run").then_some(window),
            min: int("own_min"),
            max: int("own_max").filter(|_| int("own_min").is_none()),
            at_or_over: None,
            max_by_age: Vec::new(),
            basis: "Your own number".into(),
        }
    });
    let mut not_said =
        vec!["This card is yours: its number is your own, not a body's.".to_string()];
    if check.is_none() {
        not_said
            .push("Its count could not be read: choose a tracker and how it is counted.".into());
    }
    let body = o.body.trim();
    Card {
        id: format!("{OWN}:{}", o.id),
        shelf: OWN.into(),
        name: o
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Your own card".into()),
        rests_on: OWN.into(),
        says: if body.is_empty() {
            Vec::new()
        } else {
            vec![Said { by: "You".into(), text: body.to_string(), nhs: false }]
        },
        urgent: Vec::new(),
        disagree: String::new(),
        sources: Vec::new(),
        trackers: stem.map(|_| vec![key]).unwrap_or_default(),
        choose: Vec::new(),
        checks: check.into_iter().collect(),
        not_said,
    }
}

/// What the person chose for a card: keys, by the id of the choice.
type Chosen = BTreeMap<String, Vec<String>>;
/// A chosen set of keys is kept on the following note under `p_<choice>`.
const CHOSEN: &str = "p_";

/// One check, counted.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Counted {
    pub id: String,
    pub label: String,
    pub basis: String,
    pub value: i64,
    /// The days it was counted over, and how many of them had anything recorded on its keys.
    pub of: usize,
    pub recorded: usize,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_or_over: Option<i64>,
    /// Whether the count is within the body's number, when it gives one. Shown beside the number,
    /// never as a word about the person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<bool>,
    /// The check reads keys the person was asked to choose, and none are chosen.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unchosen: bool,
    /// The numbers behind the count, where there are several: cycle lengths.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<i64>,
    /// The cycles counted over, each with why it is left out when it is.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cycles: Vec<crate::cycle::Cycle>,
    /// Headache days outside every period's window (`around_period`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside: Option<usize>,
    /// Why the count is not set beside the number, when it is not.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub not_compared: String,
}

/// A card being followed, and its checks on the day asked about.
#[derive(Debug, Serialize)]
pub struct Followed {
    pub card: String,
    /// The note that follows it: stopping gives it a last day.
    pub note: String,
    pub since: String,
    /// What the person chose for it, by choice id.
    pub chosen: BTreeMap<String, Vec<String>>,
    pub checks: Vec<Counted>,
}

#[derive(Debug, Serialize)]
pub struct Guidance {
    /// The credit the NHS website asks for beside its words.
    pub nhs_credit: String,
    pub cards: Vec<Card>,
    pub followed: Vec<Followed>,
}

fn day_string(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

fn parse_day(s: &str) -> Result<Date, StoreError> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DATE)
        .map_err(|_| StoreError::Parse(format!("day must be like 2026-07-20, not '{s}'")))
}

/// The notes following a card on `day`: `program: health` with a `plan`, started on or before it
/// and not yet past its `due`. Where two follow one card, the later start wins.
fn following(store: &dyn Store, day: Date) -> Result<Vec<Object>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Prop {
            key: PROGRAM.into(),
            op: fm_query::Op::Eq,
            value: PropertyValue::Text(PROGRAM_HEALTH.into()),
        }),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut by_card: BTreeMap<String, Object> = BTreeMap::new();
    for o in store.query(&q)?.rows {
        if o.kind != Kind::Note || is_template(&o) || is_message(&o) || is_proposal(&o) {
            continue;
        }
        let PropertyValue::Text(card) = o.get(PLAN) else { continue };
        let start = o.start.map(|s| s.date);
        if start.is_some_and(|s| s > day) || o.due.is_some_and(|d| d.date < day) {
            continue;
        }
        // Each card the person wrote is its own card; a library card is followed once.
        let card = if card == OWN { format!("{OWN}:{}", o.id) } else { card };
        let later = by_card.get(&card).is_none_or(|had| had.start.map(|s| s.date) <= start);
        if later {
            by_card.insert(card, o);
        }
    }
    Ok(by_card.into_values().collect())
}

fn chosen_of(note: &Object, card: &Card) -> BTreeMap<String, Vec<String>> {
    card.choose
        .iter()
        .map(|c| {
            let keys = match note.get(&format!("{CHOSEN}{}", c.id)) {
                PropertyValue::Text(t) => t
                    .split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| parse_key(k).is_some())
                    .collect(),
                _ => Vec::new(),
            };
            (c.id.clone(), keys)
        })
        .collect()
}

/// The keys a check reads: its own, and those chosen for it.
fn keys_of(check: &Check, chosen: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut keys = check.keys.clone();
    for from in &check.keys_from {
        keys.extend(chosen.get(from).cloned().unwrap_or_default());
    }
    keys.sort();
    keys.dedup();
    keys
}

/// Several trackers as one: a day's value is the most any of them reached, and a day is recorded
/// when any of them was. *"Any painkiller"* is a day with any of them at 1 or more.
fn merged(all: &BTreeMap<String, (Series, usize)>, keys: &[String]) -> Series {
    let mut out = Series::new();
    for k in keys {
        if let Some((s, _)) = all.get(k) {
            for (d, v) in s {
                out.entry(*d).and_modify(|x| *x = (*x).max(*v)).or_insert(*v);
            }
        }
    }
    out
}

fn monday_of(d: Date) -> Date {
    days::week_of(d)
}

/// **Count one check on `day`.** The periods end on `day`: this week from its Monday, this month
/// from its first, a window of days, or since the diary began.
fn measure(check: &Check, s: &Series, keys_empty: bool, day: Date) -> Counted {
    let (from, value, of, recorded) = match check.kind.as_str() {
        "kept" => {
            let first = s.keys().next().copied().filter(|f| *f <= day);
            match first {
                Some(f) => {
                    let c = days::count(s, f, day, 0);
                    (f, c.of as i64, c.of, c.recorded)
                }
                None => (day, 0, 0, 0),
            }
        }
        "week_days" | "month_days" => {
            let from = if check.kind == "week_days" {
                monday_of(day)
            } else {
                day.replace_day(1).expect("the first of a month exists")
            };
            let c = days::count(s, from, day, 1);
            (from, c.at_least as i64, c.of, c.recorded)
        }
        "days_over" => {
            let from = monday_of(day);
            let c = days::count(s, from, day, check.value.unwrap_or(0).saturating_add(1));
            (from, c.at_least as i64, c.of, c.recorded)
        }
        "at_least" => {
            let from = monday_of(day);
            let c = days::count(s, from, day, check.value.unwrap_or(1));
            (from, c.at_least as i64, c.of, c.recorded)
        }
        "run" => {
            let from = day - Duration::days(check.window.unwrap_or(30).max(1) - 1);
            let c = days::count(s, from, day, 1);
            (from, days::longest_run(s, from, day, 1) as i64, c.of, c.recorded)
        }
        other => unreachable!("not a diary check: {other}"),
    };
    let within = if keys_empty {
        None
    } else if let Some(m) = check.min {
        Some(value >= m)
    } else if let Some(m) = check.max {
        Some(value <= m)
    } else {
        check.at_or_over.map(|m| value < m)
    };
    Counted {
        id: check.id.clone(),
        label: check.label.clone(),
        basis: check.basis.clone(),
        value,
        of,
        recorded,
        from: day_string(from),
        to: day_string(day),
        min: check.min,
        max: check.max,
        at_or_over: check.at_or_over,
        within,
        unchosen: keys_empty && !check.keys_from.is_empty(),
        values: Vec::new(),
        cycles: Vec::new(),
        outside: None,
        not_compared: String::new(),
    }
}

const CYCLE_KINDS: [&str; 6] = [
    "cycle_lengths",
    "bleeding_days",
    "cycle_spread",
    "spotting_days",
    "heavy_days",
    "around_period",
];

/// **Count one cycle check on `day`** (`cycle.rs` does the arithmetic). `age` is from the person's
/// profile, when they gave a year of birth.
fn measure_cycle(
    check: &Check,
    notes: &crate::health::Notes,
    headache: &Series,
    age: Option<i64>,
    day: Date,
) -> Counted {
    use crate::cycle;
    let since = day - Duration::days(cycle::SIX_MONTHS);
    let all = cycle::cycles(notes, day);
    let lengths = cycle::counted_lengths(&all);
    let mut c = Counted {
        id: check.id.clone(),
        label: check.label.clone(),
        basis: check.basis.clone(),
        value: 0,
        of: 0,
        recorded: 0,
        from: day_string(since),
        to: day_string(day),
        min: check.min,
        max: check.max,
        at_or_over: check.at_or_over,
        within: None,
        unchosen: false,
        values: Vec::new(),
        cycles: Vec::new(),
        outside: None,
        not_compared: String::new(),
    };
    match check.kind.as_str() {
        "cycle_lengths" => {
            c.value = lengths.len() as i64;
            c.of = all.len();
            c.recorded = lengths.len();
            if !lengths.is_empty() {
                let lo = check.min.unwrap_or(i64::MIN);
                let hi = check.max.unwrap_or(i64::MAX);
                c.within = Some(lengths.iter().all(|l| (lo..=hi).contains(l)));
            }
            c.values = lengths;
            c.cycles = all;
        }
        "cycle_spread" => {
            c.of = all.len();
            c.recorded = lengths.len();
            if lengths.len() >= 2 {
                c.value = lengths.iter().max().unwrap() - lengths.iter().min().unwrap();
                c.values = lengths;
                let most = age.and_then(|a| {
                    check
                        .max_by_age
                        .iter()
                        .find(|[from, to, _]| (*from..=*to).contains(&a))
                        .map(|r| r[2])
                });
                match (age, most) {
                    (None, _) => c.not_compared = "your year of birth is not in your profile".into(),
                    (Some(_), None) => {
                        c.not_compared = "FIGO gives no number for this age: \"below the age of 18 and after 45 years, ovulatory \
                             disorders are frequent and norms are not well defined\""
                            .into()
                    }
                    (Some(_), Some(m)) => {
                        c.max = Some(m);
                        c.within = Some(c.value <= m);
                    }
                }
            } else {
                c.not_compared = "fewer than two cycles counted".into();
            }
        }
        "bleeding_days" | "heavy_days" => {
            if let Some((first, run)) = cycle::latest_period(notes, day) {
                c.from = day_string(first);
                c.of = run as usize;
                c.recorded = run as usize;
                c.value = if check.kind == "bleeding_days" {
                    run
                } else {
                    cycle::heavy_days(notes, first, run)
                };
                if check.kind == "bleeding_days" {
                    c.within = check.max.map(|m| run <= m);
                }
            } else {
                c.not_compared = "no first day of a period recorded".into();
            }
        }
        "spotting_days" => {
            c.value = cycle::spotting_days(notes, day);
            c.of = (day - since).whole_days() as usize + 1;
        }
        "around_period" => {
            let (hits, of, outside) = cycle::around_periods(notes, headache, day, 3);
            c.value = hits as i64;
            c.of = of;
            c.recorded = of;
            c.outside = Some(outside);
        }
        other => unreachable!("not a cycle check: {other}"),
    }
    c
}

/// The days a check could look back over: the longest is *since the diary began*, read over the
/// most a span may be (`health_days`'s 400).
const LOOK_BACK: i64 = 399;

/// **The cards, and the ones followed, measured on `day`.**
pub fn health_guidance(store: &dyn Store, day: &str) -> Result<Guidance, StoreError> {
    let day = parse_day(day)?;
    let lib = library();
    let mut followed = Vec::new();
    let notes = following(store, day)?;
    // Every key any followed check reads, read in one pass.
    let mut wanted: Vec<(Object, std::borrow::Cow<'static, Card>, Chosen)> = Vec::new();
    let mut all_keys: Vec<String> = Vec::new();
    let mut own_cards: Vec<Card> = Vec::new();
    for n in notes {
        let PropertyValue::Text(id) = n.get(PLAN) else { continue };
        let card = if id == OWN {
            let c = own_card(&n);
            own_cards.push(c.clone());
            std::borrow::Cow::Owned(c)
        } else {
            let Some(card) = lib.cards.iter().find(|c| c.id == id) else { continue };
            std::borrow::Cow::Borrowed(card)
        };
        let chosen = chosen_of(&n, &card);
        all_keys.extend(card.trackers.iter().cloned());
        for c in &card.checks {
            all_keys.extend(keys_of(c, &chosen));
        }
        wanted.push((n, card, chosen));
    }
    all_keys.sort();
    all_keys.dedup();
    let trackers: Vec<Tracker> = all_keys
        .iter()
        .filter_map(|k| {
            parse_key(k).map(|(stem, kind)| Tracker {
                key: k.clone(),
                label: crate::health::label_of(stem),
                kind,
            })
        })
        .collect();
    let all = series(store, &trackers, day - Duration::days(LOOK_BACK), day)?;
    // The cycle reads days of notes, and the person's age; only when a followed card asks.
    let cyclic = wanted
        .iter()
        .any(|(_, card, _)| card.checks.iter().any(|c| CYCLE_KINDS.contains(&c.kind.as_str())));
    let (notes, age) = if cyclic {
        let notes = crate::health::day_notes(store, day - Duration::days(LOOK_BACK), day)?;
        let born = crate::plan::plan_profile(store)?.born;
        (notes, born.map(|b| i64::from(day.year()) - b))
    } else {
        (Default::default(), None)
    };
    for (note, card, chosen) in wanted {
        let checks = card
            .checks
            .iter()
            .map(|c| {
                if CYCLE_KINDS.contains(&c.kind.as_str()) {
                    let headache = merged(&all, &keys_of(c, &chosen));
                    return measure_cycle(c, &notes, &headache, age, day);
                }
                // `kept` reads the card's own trackers: the diary it asks for.
                let keys = if c.kind == "kept" && c.keys.is_empty() {
                    card.trackers.clone()
                } else {
                    keys_of(c, &chosen)
                };
                measure(c, &merged(&all, &keys), keys.is_empty(), day)
            })
            .collect();
        followed.push(Followed {
            card: card.id.clone(),
            note: note.id.to_string(),
            since: note.start.map(|s| day_string(s.date)).unwrap_or_default(),
            chosen,
            checks,
        });
    }
    let mut cards = lib.cards.clone();
    cards.extend(own_cards);
    Ok(Guidance { nhs_credit: lib.nhs_credit.clone(), cards, followed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::MemoryStore;
    use fm_model::Stamp;
    use time::Month;

    const KINDS: [&str; 12] = [
        "kept",
        "week_days",
        "month_days",
        "days_over",
        "at_least",
        "run",
        "cycle_lengths",
        "bleeding_days",
        "cycle_spread",
        "spotting_days",
        "heavy_days",
        "around_period",
    ];

    #[test]
    fn every_card_parses_and_every_check_is_of_a_known_kind_with_a_basis() {
        let lib = library();
        assert!(lib.nhs_credit.contains("Open Government Licence v3.0"));
        assert!(lib.cards.len() >= 10);
        for card in &lib.cards {
            assert!(
                ["diary", "nose", "daily", "cycle"].contains(&card.shelf.as_str()),
                "{}",
                card.id
            );
            assert!(
                ["guideline", "consensus", "review", "definition"]
                    .contains(&card.rests_on.as_str()),
                "{}",
                card.id
            );
            assert!(!card.says.is_empty() && !card.sources.is_empty(), "{}", card.id);
            assert!(card.sources.iter().all(|s| s.url.starts_with("https://")), "{}", card.id);
            for k in &card.trackers {
                assert!(parse_key(k).is_some(), "{}: {k} is not a tracker", card.id);
            }
            for c in &card.checks {
                assert!(KINDS.contains(&c.kind.as_str()), "{}: {}", card.id, c.kind);
                assert!(!c.basis.is_empty(), "{}: {}", card.id, c.id);
                for k in &c.keys {
                    assert!(parse_key(k).is_some(), "{}: {k}", card.id);
                }
                for from in &c.keys_from {
                    assert!(card.choose.iter().any(|x| &x.id == from), "{}: {from}", card.id);
                }
            }
        }
    }

    #[test]
    fn no_card_names_a_condition_as_the_persons() {
        for card in &library().cards {
            for c in &card.checks {
                let l = c.label.to_lowercase();
                for word in ["overuse", "migraine", "chronic", "you have", "diagnos"] {
                    assert!(!l.contains(word), "{}: '{}' in a check's label", card.id, word);
                }
            }
        }
    }

    fn day(m: Month, d: u8) -> Date {
        Date::from_calendar_date(2026, m, d).unwrap()
    }

    fn put(s: &mut MemoryStore, f: impl FnOnce(&mut Object)) {
        let mut o = Object::new(Kind::Note, "");
        f(&mut o);
        s.put(&o).unwrap();
    }
    fn entry(s: &mut MemoryStore, d: Date, kv: &[(&str, i64)]) {
        put(s, |o| {
            o.extra.insert("log".into(), PropertyValue::Text(HEALTH.into()));
            o.start = Some(Stamp::day(d));
            for (k, v) in kv {
                o.extra.insert((*k).into(), PropertyValue::Int(*v));
            }
        });
    }
    fn follow(s: &mut MemoryStore, card: &str, from: Date, extra: &[(&str, &str)]) {
        put(s, |o| {
            o.extra.insert(PROGRAM.into(), PropertyValue::Text(PROGRAM_HEALTH.into()));
            o.extra.insert(PLAN.into(), PropertyValue::Text(card.into()));
            o.start = Some(Stamp::day(from));
            for (k, v) in extra {
                o.extra.insert((*k).into(), PropertyValue::Text((*v).into()));
            }
        });
    }

    #[test]
    fn a_followed_card_counts_the_week_the_month_and_keeps_the_two_classes_apart() {
        let mut s = MemoryStore::new();
        // Thursday 22 October 2026; its week began on Monday 19.
        let today = day(Month::October, 22);
        entry(&mut s, day(Month::October, 2), &[("paracetamol_doses", 2)]);
        entry(&mut s, day(Month::October, 19), &[("paracetamol_doses", 1)]);
        entry(&mut s, day(Month::October, 20), &[("sumatriptan_doses", 1)]);
        entry(&mut s, day(Month::October, 21), &[("paracetamol_doses", 0)]);
        follow(
            &mut s,
            "painkiller-days",
            day(Month::October, 1),
            &[("p_simple", "paracetamol_doses"), ("p_specific", "sumatriptan_doses")],
        );
        let g = health_guidance(&s, "2026-10-22").unwrap();
        assert_eq!(g.followed.len(), 1);
        let f = &g.followed[0];
        let by = |id: &str| f.checks.iter().find(|c| c.id == id).unwrap();
        // Any painkiller this week: the 19th and the 20th, beside the NHS's 2.
        let w = by("week");
        assert_eq!((w.value, w.of, w.recorded, w.within), (2, 4, 3, Some(true)));
        assert_eq!(w.from, "2026-10-19");
        // The month, each class on its own: never added together.
        assert_eq!((by("simple").value, by("simple").at_or_over), (2, Some(15)));
        assert_eq!(by("specific").value, 1);
        assert_eq!(by("specific").within, Some(true));
        let _ = today;
    }

    #[test]
    fn a_card_whose_keys_were_not_chosen_says_so_and_counts_nothing() {
        let mut s = MemoryStore::new();
        follow(&mut s, "painkiller-days", day(Month::October, 1), &[]);
        let g = health_guidance(&s, "2026-10-22").unwrap();
        let c = &g.followed[0].checks[0];
        assert!(c.unchosen && c.within.is_none() && c.value == 0);
    }

    #[test]
    fn days_kept_run_from_the_first_recorded_day_and_a_run_ends_on_a_missing_day() {
        let mut s = MemoryStore::new();
        entry(&mut s, day(Month::October, 1), &[("headache_0to3", 0)]);
        entry(&mut s, day(Month::October, 5), &[("headache_0to3", 2)]);
        follow(&mut s, "headache-diary", day(Month::October, 1), &[]);
        for (d, v) in [
            (10, 1),
            (11, 1),
            (12, 1),
            (14, 1),
            (15, 1),
            (16, 1),
            (17, 1),
            (18, 1),
            (19, 1),
            (20, 1),
        ] {
            entry(&mut s, day(Month::October, d), &[("decongestant_spray_yes", v)]);
        }
        follow(&mut s, "hay-fever", day(Month::October, 1), &[]);
        let g = health_guidance(&s, "2026-10-22").unwrap();
        let kept = g.followed.iter().find(|f| f.card == "headache-diary").unwrap();
        let k = &kept.checks[0];
        assert_eq!((k.value, k.recorded, k.min, k.within), (22, 2, Some(56), Some(false)));
        let run = g.followed.iter().find(|f| f.card == "hay-fever").unwrap();
        // The 13th was not recorded, so the 14th starts again: seven in a row, beside the NHS's 5.
        assert_eq!((run.checks[0].value, run.checks[0].within), (7, Some(false)));
    }

    #[test]
    fn a_card_stopped_or_not_yet_started_is_not_followed() {
        let mut s = MemoryStore::new();
        follow(&mut s, "sleep", day(Month::November, 1), &[]);
        put(&mut s, |o| {
            o.extra.insert(PROGRAM.into(), PropertyValue::Text(PROGRAM_HEALTH.into()));
            o.extra.insert(PLAN.into(), PropertyValue::Text("caffeine".into()));
            o.start = Some(Stamp::day(day(Month::October, 1)));
            o.due = Some(Stamp::day(day(Month::October, 10)));
        });
        assert!(health_guidance(&s, "2026-10-22").unwrap().followed.is_empty());
    }

    #[test]
    fn your_cycle_counts_lengths_beside_each_body_and_compares_the_spread_only_with_an_age() {
        let mut s = MemoryStore::new();
        for (m, d) in
            [(Month::July, 1), (Month::July, 29), (Month::August, 30), (Month::September, 28)]
        {
            entry(&mut s, day(m, d), &[("period_start_yes", 1), ("period_flow_0to3", 2)]);
        }
        follow(&mut s, "your-cycle", day(Month::July, 1), &[]);
        let g = health_guidance(&s, "2026-10-22").unwrap();
        let by = |g: &Guidance, id: &str| {
            let f = &g.followed[0];
            let c = f.checks.iter().find(|c| c.id == id).unwrap();
            (c.value, c.values.clone(), c.within, c.max, c.not_compared.clone())
        };
        // 28, 32 and 29: within FIGO's 24–38 and within the NHS's 21–35, each said on its own.
        assert_eq!(by(&g, "lengths_figo").1, vec![28, 32, 29]);
        assert_eq!(by(&g, "lengths_figo").2, Some(true));
        assert_eq!(by(&g, "lengths_nhs").2, Some(true));
        // A spread of 4, not compared until a year of birth is known.
        let spread = by(&g, "spread");
        assert_eq!((spread.0, spread.2), (4, None));
        assert!(spread.4.contains("year of birth"));
        put(&mut s, |o| {
            o.extra.insert("profile".into(), PropertyValue::Bool(true));
            o.extra.insert("born".into(), PropertyValue::Int(1995));
        });
        let g = health_guidance(&s, "2026-10-22").unwrap();
        // 31 in 2026: FIGO's 26–41 band, at most 7.
        let spread = by(&g, "spread");
        assert_eq!((spread.2, spread.3), (Some(true), Some(7)));
    }

    #[test]
    fn a_card_the_person_writes_counts_their_tracker_against_their_own_number() {
        let mut s = MemoryStore::new();
        for d in [19, 20, 21] {
            entry(&mut s, day(Month::October, d), &[("bedding_washed_yes", i64::from(d == 20))]);
        }
        put(&mut s, |o| {
            o.title = Some("Wash the bedding".into());
            o.body = "My allergist said once a week.".into();
            o.extra.insert(PROGRAM.into(), PropertyValue::Text(PROGRAM_HEALTH.into()));
            o.extra.insert(PLAN.into(), PropertyValue::Text(OWN.into()));
            o.extra.insert("own_key".into(), PropertyValue::Text("bedding_washed_yes".into()));
            o.extra.insert("own_count".into(), PropertyValue::Text("week_days".into()));
            o.extra.insert("own_min".into(), PropertyValue::Int(1));
            o.start = Some(Stamp::day(day(Month::October, 1)));
        });
        // A second one, whose count was never set: listed, and says it could not be read.
        put(&mut s, |o| {
            o.extra.insert(PROGRAM.into(), PropertyValue::Text(PROGRAM_HEALTH.into()));
            o.extra.insert(PLAN.into(), PropertyValue::Text(OWN.into()));
            o.start = Some(Stamp::day(day(Month::October, 1)));
        });
        let g = health_guidance(&s, "2026-10-22").unwrap();
        let own: Vec<&Card> = g.cards.iter().filter(|c| c.shelf == OWN).collect();
        assert_eq!(own.len(), 2, "each card the person wrote is a card of its own");
        let mine = own.iter().find(|c| c.name == "Wash the bedding").unwrap();
        assert_eq!((mine.rests_on.as_str(), mine.says[0].by.as_str()), ("own", "You"));
        assert!(mine.sources.is_empty());
        let f = g.followed.iter().find(|f| f.card == mine.id).unwrap();
        let c = &f.checks[0];
        assert_eq!(c.label, "Days this week with Bedding washed");
        assert_eq!(
            (c.value, c.min, c.within, c.basis.as_str()),
            (1, Some(1), Some(true), "Your own number")
        );
        let blank = own.iter().find(|c| c.name == "Your own card").unwrap();
        assert!(
            blank.checks.is_empty()
                && blank.not_said.iter().any(|n| n.contains("could not be read"))
        );
    }
}
