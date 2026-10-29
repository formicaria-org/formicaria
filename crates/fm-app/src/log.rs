//! Logged entries — a meal, a set, a weighing — and what a day of them comes to.
//!
//! `decisions.md#data` and `#seams`, 2026-10-11. An entry is a note carrying `log: <kind>`, a day
//! in `start`, and whole numbers whose unit is in the key (`kcal`, `protein_mg`). These are the
//! deterministic tools over them: each is a function here behind one `dispatch` arm, so the UI,
//! `fm-cli`, curl and the assistant all reach it the same way, and none of it needs the assistant.
//!
//! **A day is asked for by a view's name.** `views::totals_named` turns the name into a
//! [`TotalsDef`] on this side of the wire, so no query and no property list ever crosses it — the
//! same rule `run_view` keeps.

use crate::dto::ObjectMeta;
use crate::thread::{
    is_log, is_message, is_proposal, is_template, log_base, log_with_keyed_base,
    log_with_programs_base, LOG,
};
use crate::views::TotalsDef;
use fm_core::edit::apply_property;
use fm_core::{Store, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Query, SortKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::{format_description::well_known::Iso8601, Date, OffsetDateTime};

/// An entry that is meant, not yet eaten or lifted, carries this. It is in a day's rows — that is
/// where it gets confirmed — and in none of its sums. Confirming clears the key.
pub const PLANNED: &str = "planned";
/// A note carrying `program: <kind>` holds that log's targets, from its `start` on.
pub const PROGRAM: &str = "program";

/// More days than this in one ask is a mistake, not a chart.
const MAX_DAYS: i64 = 400;
/// More notes than this in one write is a mistake, not a meal.
const MAX_DRAFTS: usize = 200;

fn is_planned(o: &Object) -> bool {
    !matches!(
        o.get(PLANNED),
        PropertyValue::Null | PropertyValue::Bool(false) | PropertyValue::Int(0)
    )
}

/// A program note of this log: `program: food` is the food log's.
fn is_log_of_program(o: &Object, kind: &str) -> bool {
    matches!(o.get(PROGRAM), PropertyValue::Text(k) if k == kind)
}

fn is_log_of(o: &Object, kind: &str) -> bool {
    o.get(LOG) == PropertyValue::Text(kind.to_string())
}

fn parse_day(which: &str, s: &str) -> Result<Date, StoreError> {
    Date::parse(s, &Iso8601::DATE)
        .map_err(|_| StoreError::Parse(format!("{which} must be a day like 2026-07-20, not '{s}'")))
}
fn day_string(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

// ── a span of days ───────────────────────────────────────────────────────────────────────────────

/// A sum, and how complete it is.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MetricSum {
    pub sum: i64,
    /// How many of the day's entries carried a whole number for this value.
    pub n: usize,
    /// How many carried something that is not one (`12.5`). Reported, never added, never dropped.
    #[serde(skip_serializing_if = "is_zero")]
    pub odd: usize,
    /// The least and the most the sum could be, when some entry's amount is a range
    /// ([`AMOUNT_MIN`]). Absent when every amount is one number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<i64>,
}

/// How much of a food an entry is, in grams — and, when the person only knows it roughly, the
/// least and the most it could be (`decisions.md#data`, 2026-10-12, *an amount may be a range*).
/// **Every value of a food scales with its amount**, so the two bounds are all an entry has to
/// carry: a value's least is the value × least ÷ amount, and no nutrient is stored twice.
pub const AMOUNT: &str = "amount_g";
pub const AMOUNT_MIN: &str = "amount_min_g";
pub const AMOUNT_MAX: &str = "amount_max_g";

/// `(amount, least, most)` when an entry's amount is a range that holds its amount.
fn range_of(o: &Object) -> Option<(i64, i64, i64)> {
    let int = |k: &str| match o.get(k) {
        PropertyValue::Int(v) if v > 0 => Some(v),
        _ => None,
    };
    let (amount, min, max) = (int(AMOUNT)?, int(AMOUNT_MIN)?, int(AMOUNT_MAX)?);
    (min <= amount && amount <= max && min < max).then_some((amount, min, max))
}
fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, Serialize)]
pub struct DayTotals {
    pub day: String,
    /// How many entries the day has, planned ones left out.
    pub of: usize,
    pub sums: BTreeMap<String, MetricSum>,
    /// The targets in force that day, by metric key. Empty when no program covers it.
    pub targets: BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<ObjectMeta>>,
    /// When the day's entries were, for those that carry a time: how many separate occasions,
    /// and the first and the last (`HH:MM`). `None` when no counted entry has a time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<DayTimes>,
}

/// The times of a day's entries.
///
/// **An occasion is a run of entries less than 15 minutes apart** — the rule of Gill and Panda
/// (*Cell Metab* 2015;22:789), one of several in use (`sources-2026-10-11.md` §6c). It is a count
/// of what happened; no body gives a number to reach.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DayTimes {
    pub occasions: usize,
    pub first: String,
    pub last: String,
}

/// Minutes apart below which two entries are one occasion.
pub const OCCASION_GAP_MIN: i64 = 15;

/// The minute of the day an entry was at, when it carries a time.
pub fn minute_of(o: &Object) -> Option<i64> {
    o.start.and_then(|s| s.time).map(|t| i64::from(t.hour()) * 60 + i64::from(t.minute()))
}

fn day_times(counted: &[Object]) -> Option<DayTimes> {
    let mut at: Vec<i64> = counted.iter().filter_map(minute_of).collect();
    at.sort_unstable();
    let (first, last) = (*at.first()?, *at.last()?);
    let occasions = 1 + at.windows(2).filter(|w| w[1] - w[0] >= OCCASION_GAP_MIN).count();
    let clock = |m: i64| format!("{:02}:{:02}", m / 60, m % 60);
    Some(DayTimes { occasions, first: clock(first), last: clock(last) })
}

#[derive(Debug, Serialize)]
pub struct LogDays {
    pub days: Vec<DayTotals>,
}

/// Every day from `from` to `to`: what its entries come to, how complete that is, the targets in
/// force, and — when `rows` — the entries themselves, in slot order.
///
/// **One store query for the span, then the pure engine once per day.** The entries are bucketed
/// by the *day* of their `start`, so a hand-edited entry with a time on it still lands in its own
/// day; the totals are `fm_query`'s [`Tally`](fm_query::Tally), run over that day's counted rows.
pub fn log_days(
    store: &dyn Store,
    def: &TotalsDef,
    from: &str,
    to: &str,
    rows: bool,
) -> Result<LogDays, StoreError> {
    let from = parse_day("from", from)?;
    let to = parse_day("to", to)?;
    if to < from || (to - from).whole_days() >= MAX_DAYS {
        return Err(StoreError::Parse(format!(
            "from {} to {} is not a span of at most {MAX_DAYS} days",
            day_string(from),
            day_string(to)
        )));
    }

    // **One pass over the vault for the entries and the targets both.** Each is a whole-corpus
    // read — only `Kind` and text reach the index — and a day screen asks for this on every
    // change, so two passes was twice the cost of the screen (`tests/perf.rs`).
    // A value read from another log (a weight among the food trends) rides the same pass.
    let mut logs: Vec<&str> = vec![&def.log];
    for m in &def.metrics {
        if let Some(l) = m.log.as_deref().filter(|l| !logs.contains(l)) {
            logs.push(l);
        }
    }
    let q = Query {
        filter: if logs.len() == 1 {
            log_with_programs_base(&def.log, PROGRAM)
        } else {
            crate::thread::plan_base(&logs, &[PROGRAM])
        },
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut by_day: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
    let mut beside: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
    let mut programs = Vec::new();
    for o in store.query(&q)?.rows {
        if is_log(&o) {
            if let Some(s) = o.start.filter(|s| s.date >= from && s.date <= to) {
                let own = is_log_of(&o, &def.log);
                if own { &mut by_day } else { &mut beside }.entry(s.date).or_default().push(o);
            }
        } else if !is_log_of_program(&o, &def.log) {
            // Another group's program: its targets are not this log's.
        } else if !is_message(&o) && !is_proposal(&o) && !is_template(&o) {
            // A program is something you plan: an ordinary note, never a hidden one.
            programs.push(o);
        }
    }

    let keys: Vec<String> = def.metrics.iter().map(|m| m.key.clone()).collect();
    let slot_at = |o: &Object| {
        let slot = match o.get("slot") {
            PropertyValue::Text(s) => s,
            _ => String::new(),
        };
        def.slots.iter().position(|s| s.value == slot).unwrap_or(usize::MAX)
    };

    let mut days = Vec::new();
    let mut day = from;
    loop {
        let mut all = by_day.remove(&day).unwrap_or_default();
        // Stable: within a slot the entries keep the order they were made in.
        all.sort_by_key(slot_at);
        let counted: Vec<Object> = all.iter().filter(|o| !is_planned(o)).cloned().collect();
        let tallied = fm_query::run(&Query { tally: keys.clone(), ..Default::default() }, &counted);
        let mut sums: BTreeMap<String, MetricSum> = keys
            .iter()
            .map(|k| {
                let t = tallied.tallies.get(k).cloned().unwrap_or_default();
                (k.clone(), MetricSum { sum: t.sum, n: t.n, odd: t.odd, low: None, high: None })
            })
            .collect();
        if counted.iter().any(|o| range_of(o).is_some()) {
            for (key, got) in sums.iter_mut() {
                let (mut low, mut high) = (0i64, 0i64);
                for o in &counted {
                    let PropertyValue::Int(v) = o.get(key) else { continue };
                    match range_of(o) {
                        Some((amount, min, max)) => {
                            low = low.saturating_add(v.saturating_mul(min) / amount);
                            high = high.saturating_add(v.saturating_mul(max) / amount);
                        }
                        None => {
                            low = low.saturating_add(v);
                            high = high.saturating_add(v);
                        }
                    }
                }
                (got.low, got.high) = (Some(low), Some(high));
            }
        }
        let others = beside.remove(&day).unwrap_or_default();
        for m in &def.metrics {
            let Some(l) = m.log.as_deref().filter(|l| *l != def.log) else { continue };
            let theirs: Vec<Object> =
                others.iter().filter(|o| is_log_of(o, l) && !is_planned(o)).cloned().collect();
            let t =
                fm_query::run(&Query { tally: vec![m.key.clone()], ..Default::default() }, &theirs)
                    .tallies
                    .get(&m.key)
                    .cloned()
                    .unwrap_or_default();
            sums.insert(
                m.key.clone(),
                MetricSum { sum: t.sum, n: t.n, odd: t.odd, low: None, high: None },
            );
        }
        days.push(DayTotals {
            day: day_string(day),
            of: counted.len(),
            sums,
            targets: targets_on(&programs, day, &keys),
            rows: rows.then(|| all.iter().map(ObjectMeta::from).collect()),
            times: day_times(&counted),
        });
        if day >= to {
            break;
        }
        day = day.next_day().ok_or_else(|| StoreError::Parse("the calendar ended".into()))?;
    }
    Ok(LogDays { days })
}

// The notes that hold a log's targets are ordinary notes — a program is something you plan, so it
// is on the agenda like anything else with a `start` — found by **property**, never by a tag
// alone, which would let a training program supply food targets.

/// The targets in force on `day`: from the program with the latest `start` on or before it — and,
/// if it has a `due`, not yet past that — then the one changed most recently. A program with no `start` is in force from the beginning, and
/// loses to any that has one.
fn targets_on(programs: &[Object], day: Date, keys: &[String]) -> BTreeMap<String, i64> {
    let Some(p) = programs
        .iter()
        .filter(|p| p.start.is_none_or(|s| s.date <= day))
        // A program with an end is over the day after it: `due` is its last day.
        .filter(|p| p.due.is_none_or(|d| d.date >= day))
        .max_by_key(|p| (p.start.map(|s| s.date), p.updated))
    else {
        return BTreeMap::new();
    };
    keys.iter()
        .filter_map(|k| match p.get(k) {
            PropertyValue::Int(v) => Some((k.clone(), v)),
            _ => None,
        })
        .collect()
}

// ── what was logged before ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RecentItem {
    pub title: String,
    /// The entry's properties as last logged — its amount and its values, ready to log again.
    pub props: BTreeMap<String, serde_json::Value>,
    pub count: usize,
    pub last: String,
}

/// What this log holds already, one line per distinct thing: most often logged **in this slot**
/// first, then most often logged at all, then most recent. Each carries the properties it had the
/// last time, so logging it again needs no typing.
///
/// Two entries are the same thing when they name the same `source`, or failing that have the same
/// title. Planned entries and saved ones (no `start`) are not something that happened and are
/// left out.
pub fn log_recent(
    store: &dyn Store,
    def: &TotalsDef,
    slot: &str,
    q: &str,
    limit: usize,
) -> Result<Vec<RecentItem>, StoreError> {
    struct Seen {
        item: RecentItem,
        in_slot: usize,
        at: fm_model::Stamp,
    }
    let needle = q.trim().to_lowercase();
    let query = Query {
        filter: log_base(&def.log),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut seen: BTreeMap<String, Seen> = BTreeMap::new();
    for o in store.query(&query)?.rows {
        let Some(start) = o.start else { continue };
        if is_planned(&o) {
            continue;
        }
        let title = o.title.clone().unwrap_or_default();
        if !needle.is_empty() && !title.to_lowercase().contains(&needle) {
            continue;
        }
        let key = match o.get("source") {
            PropertyValue::Text(s) if !s.is_empty() => s,
            _ => title.to_lowercase(),
        };
        let here = !slot.is_empty() && o.get("slot") == PropertyValue::Text(slot.to_string());
        let props = || ObjectMeta::from(&o).props.into_iter().collect();
        match seen.get_mut(&key) {
            None => {
                let item = RecentItem { title, props: props(), count: 1, last: start.to_string() };
                seen.insert(key, Seen { item, in_slot: usize::from(here), at: start });
            }
            Some(s) => {
                s.item.count += 1;
                s.in_slot += usize::from(here);
                // Rows arrive oldest first, so `>=` leaves the newest of a day in place.
                if start >= s.at {
                    s.at = start;
                    s.item.last = start.to_string();
                    s.item.title = title;
                    s.item.props = props();
                }
            }
        }
    }
    let mut out: Vec<Seen> = seen.into_values().collect();
    out.sort_by(|a, b| {
        b.in_slot.cmp(&a.in_slot).then(b.item.count.cmp(&a.item.count)).then(b.at.cmp(&a.at))
    });
    Ok(out.into_iter().take(limit.max(1)).map(|s| s.item).collect())
}

// ── recipes ──────────────────────────────────────────────────────────────────────────────────────

/// A recipe's notes share one word under this key: the recipe itself and each of its ingredients.
pub const RECIPE: &str = "recipe";
/// On a recipe's own note: the carried recipe it was made from, e.g. `nhlbi/<id>`.
pub const FROM: &str = "from";
/// The note that carries this is the recipe; the others under the same word are what goes in it.
pub const SERVINGS: &str = "servings";

/// A recipe, and what its ingredients come to.
#[derive(Debug, Serialize)]
pub struct Recipe {
    /// The word its notes share. An entry logged from it records `recipe/<this>` as its `source`.
    pub recipe: String,
    /// The recipe's own note.
    pub id: String,
    pub title: String,
    /// How many servings the whole of it makes. A person says; nothing estimates it.
    pub servings: i64,
    /// How many ingredients it has.
    pub of: usize,
    /// The whole recipe: every whole-number value any ingredient carries, added up, with how many
    /// ingredients carried it — so a total missing an ingredient's value says so.
    pub sums: BTreeMap<String, MetricSum>,
    pub ingredients: Vec<ObjectMeta>,
    /// The carried recipe it was made from (e.g. `nhlbi/<id>`), when it was — so the screen
    /// can set the book's lines beside it, each to be matched to a food (2026-10-18).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// The recipes kept in this log, by name, each with the sum of its ingredients.
///
/// **The recipe's own note is an ordinary note** — tagged, visible in the Library, made from a
/// template if a person likes — carrying `servings` and a word under `recipe`
/// (`decisions.md#ui`, 2026-10-12). **An ingredient is an entry like any other** — a food, an
/// amount, the values that amount had — that carries `log`, names the recipe by that word instead
/// of a day, and so is in no day and in no planning view. A recipe's note made before that
/// ruling carries `log` too, and is read the same.
///
/// **The sum is of the ingredients as entered**, raw or cooked as the person chose them. Nothing
/// here knows what cooking does to a weight or to a vitamin, and nothing guesses
/// (`decisions.md#data`, 2026-10-11, *a recipe is the sum of what went in*).
pub fn log_recipes(store: &dyn Store, def: &TotalsDef) -> Result<Vec<Recipe>, StoreError> {
    let query = Query {
        filter: log_with_keyed_base(&def.log, RECIPE),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut heads: BTreeMap<String, (Object, i64)> = BTreeMap::new();
    let mut parts: BTreeMap<String, Vec<Object>> = BTreeMap::new();
    for o in store.query(&query)?.rows {
        if o.start.is_some() || is_template(&o) || is_message(&o) || is_proposal(&o) {
            continue;
        }
        let PropertyValue::Text(word) = o.get(RECIPE) else { continue };
        if word.is_empty() {
            continue;
        }
        // An entry of another log is not this one's; a note with no `log` at all is a recipe's
        // own note, and recipes are the food log's.
        let logged = !o.get(LOG).is_null();
        if logged && !is_log_of(&o, &def.log) {
            continue;
        }
        match o.get(SERVINGS) {
            PropertyValue::Int(n) if n >= 1 => {
                heads.insert(word, (o, n));
            }
            _ if logged => parts.entry(word).or_default().push(o),
            // A note that names a recipe and is neither it nor an ingredient of it: not ours.
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (word, (head, servings)) in heads {
        let ingredients = parts.remove(&word).unwrap_or_default();
        let mut keys: Vec<String> = Vec::new();
        for o in &ingredients {
            for (k, v) in &o.extra {
                if matches!(v, PropertyValue::Int(_)) && !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
        keys.sort();
        let tallied =
            fm_query::run(&Query { tally: keys.clone(), ..Default::default() }, &ingredients);
        let sums = keys
            .iter()
            .map(|k| {
                let t = tallied.tallies.get(k).cloned().unwrap_or_default();
                (k.clone(), MetricSum { sum: t.sum, n: t.n, odd: t.odd, low: None, high: None })
            })
            .collect();
        out.push(Recipe {
            recipe: word,
            id: head.id.to_string(),
            title: head.title.clone().unwrap_or_default(),
            servings,
            of: ingredients.len(),
            sums,
            ingredients: ingredients.iter().map(ObjectMeta::from).collect(),
            from: match head.get(FROM) {
                PropertyValue::Text(s) if !s.is_empty() => Some(s),
                _ => None,
            },
        });
    }
    out.sort_by_key(|r| r.title.to_lowercase());
    Ok(out)
}

// ── packaged foods ───────────────────────────────────────────────────────────────────────────────

/// The note carrying this is a packaged food, written down from its label.
pub const PRODUCT: &str = "product";

/// A packaged food: what its label says a hundred grams hold.
#[derive(Debug, Serialize)]
pub struct Product {
    pub id: String,
    pub title: String,
    /// The number under its barcode, when the person gave it.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub barcode: String,
    /// What the pack calls a serving, in grams, when it says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_g: Option<i64>,
    /// The same for a food whose label is by volume, in millilitres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_ml: Option<i64>,
    /// `ml` when the label's figures are per 100 ml and the food is measured in millilitres
    /// (`decisions.md#data`, 2026-10-13); absent for grams. No density is assumed anywhere.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
    /// Every whole-number value the note carries whose key ends in a unit, and `kcal`: per 100
    /// of the food's unit.
    pub per100: BTreeMap<String, i64>,
}

/// The packaged foods a person has written down from their labels, by name.
///
/// **A product is an ordinary note** (`decisions.md#data`, 2026-10-12, *a packaged food is a note
/// read off its label*): its `product` property says so, its values are the label's per 100 g
/// under the keys every logged food uses, and its ingredient list is its text. It is in no log
/// and in no day; logging some of it makes an entry that carries its own numbers, as for a food
/// from the list.
pub fn log_products(store: &dyn Store) -> Result<Vec<Product>, StoreError> {
    let query = Query {
        filter: crate::thread::plan_base(&[], &[PRODUCT]),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut out: Vec<Product> = store
        .query(&query)?
        .rows
        .iter()
        .filter(|o| o.get(LOG).is_null() && !o.get(PRODUCT).is_null())
        .filter(|o| !is_template(o) && !is_message(o) && !is_proposal(o))
        .map(|o| Product {
            id: o.id.to_string(),
            title: o.title.clone().unwrap_or_default(),
            barcode: match o.get("barcode") {
                PropertyValue::Text(s) => s,
                PropertyValue::Int(n) => n.to_string(),
                _ => String::new(),
            },
            serving_g: match o.get("serving_g") {
                PropertyValue::Int(n) if n > 0 => Some(n),
                _ => None,
            },
            serving_ml: match o.get("serving_ml") {
                PropertyValue::Int(n) if n > 0 => Some(n),
                _ => None,
            },
            // Only the one word is a unit; anything else written there is not believed.
            unit: match o.get("unit") {
                PropertyValue::Text(s) if s == "ml" => s,
                _ => String::new(),
            },
            per100: o
                .extra
                .iter()
                .filter(|(k, _)| k.as_str() == "kcal" || k.ends_with("_mg") || k.ends_with("_ug"))
                .filter_map(|(k, v)| match v {
                    PropertyValue::Int(n) => Some((k.clone(), *n)),
                    _ => None,
                })
                .collect(),
        })
        .collect();
    out.sort_by_key(|p| p.title.to_lowercase());
    Ok(out)
}

// ── recipes carried in the app ───────────────────────────────────────────────────────────────────

/// The recipes that come with the app, as their source published them.
///
/// **Carried, never written into a vault, and never changed** (`decisions.md#data`, 2026-10-12,
/// *recipes from outside are carried word for word*): the U.S. National Heart, Lung, and Blood
/// Institute's, which it says are in the public domain and asks not to be altered. Each has its
/// own ingredient lines and steps, and **the values per serving are the Institute's own** — the
/// app works none out, since the amounts are in cups and spoons and tie to no food table.
/// `ci/pack-recipes.py` makes the file.
pub const RECIPES_NHLBI: &str = include_str!("shipped/recipes-nhlbi.json");

/// **Italian Wikibooks' recipes, the Italian ones** (`decisions.md#data`, 2026-10-18, *a modern
/// Italian collection is carried*): 291 pages of *Libro di cucina* filed under an Italian region,
/// word for word, under **CC BY-SA 4.0** — a separate work in its own file, credited per recipe by
/// its page and its history, its licence named in the file and in `THIRD-PARTY.md`. No values:
/// the pages give none, so a recipe of theirs counts once it is made one's own and matched to foods.
/// `ci/pack-wikibooks.py` makes the file.
pub const RECIPES_WIKIBOOKS: &str = include_str!("shipped/recipes-wikibooks.json");

/// **Every collection carried in the app, in the order they are offered.** Each file says what it
/// is called — its `key` (a logged serving's source is `<key>/<id>`) and its `label` — so nothing
/// here or in the UI names one (2026-10-18, *one recipe search: every source equal*).
const CARRIED: &[&str] = &[RECIPES_NHLBI, RECIPES_WIKIBOOKS];

/// Every carried collection, as its file holds it.
pub fn recipe_libraries() -> Result<Vec<serde_json::Value>, StoreError> {
    CARRIED
        .iter()
        .map(|text| {
            serde_json::from_str(text)
                .map_err(|e| StoreError::Parse(format!("the carried recipes do not read: {e}")))
        })
        .collect()
}

/// One carried collection by its `key`; nothing named is the first.
pub fn recipe_library(key: &str) -> Result<serde_json::Value, StoreError> {
    let all = recipe_libraries()?;
    let found = all.iter().position(|l| l["key"].as_str() == Some(key)).unwrap_or(0);
    Ok(all.into_iter().nth(found).unwrap_or_default())
}

// ── how an exercise is done ──────────────────────────────────────────────────────────────────────

/// How each exercise is done, as its source wrote it, and where a film of it can be watched.
///
/// **Carried word for word, and never written into a vault** (`decisions.md#data`, 2026-10-12, *how
/// an exercise is done is its source's own words*): the NHS website's exercises that need no
/// equipment, under the Open Government Licence, and the U.S. Army's drills (ATP 7-22.02,
/// approved for public release), which also cover a bar, free weights and machines.
/// `ci/pack-exercises.py` makes the file. The only fields that are the app's are where a card is
/// filed: `equipment` and `area`.
pub const EXERCISES: &str = include_str!("shipped/exercises.json");
/// Links to films of the same exercises: a known body's own channel, or — where none was found,
/// and said to be so — a trainer's. Links only; each was seen to exist on the day in `seen`.
pub const EXERCISE_VIDEOS: &str = include_str!("shipped/exercise-videos.json");
/// What to mind when lifting, and where else an exercise is shown done
/// (`decisions.md#data`, 2026-10-18, *a lifting card has the NIA's safety tips, and links*): the
/// National Institute on Aging's Safety Tips box, whole and word for word, for the cards that need
/// a bar, free weights or a machine; and per card, links to pages elsewhere — their titles only.
/// `ci/pack-exercise-advice.py` makes the file.
pub const EXERCISE_ADVICE: &str = include_str!("shipped/exercise-advice.json");

pub fn exercise_library() -> Result<serde_json::Value, StoreError> {
    let read = |what: &str, text: &str| {
        serde_json::from_str::<serde_json::Value>(text)
            .map_err(|e| StoreError::Parse(format!("the carried {what} do not read: {e}")))
    };
    let mut lib = read("exercises", EXERCISES)?;
    let films = read("exercise films", EXERCISE_VIDEOS)?;
    lib["videos"] = films["videos"].clone();
    lib["videos_seen"] = films["seen"].clone();
    let advice = read("exercise advice", EXERCISE_ADVICE)?;
    lib["safely"] = advice["safely"].clone();
    lib["links"] = advice["links"].clone();
    lib["links_seen"] = advice["seen"].clone();
    Ok(lib)
}

// ── what is in the kitchen ───────────────────────────────────────────────────────────────────────

/// The note carrying this is something the person has at home.
pub const KITCHEN: &str = "kitchen";

#[derive(Debug, Serialize)]
pub struct KitchenItem {
    pub id: String,
    pub title: String,
}

/// What the person says they have at home, by name.
///
/// **Have or have not, and no amount** (`decisions.md#data`, 2026-10-12, *the kitchen is a list of
/// names*): an amount is true for a day and then a chore. An item is an ordinary note carrying
/// `kitchen` — a packaged food's own note may carry it too — and is matched to what a menu needs
/// by its name.
pub fn log_kitchen(store: &dyn Store) -> Result<Vec<KitchenItem>, StoreError> {
    let query = Query {
        filter: crate::thread::plan_base(&[], &[KITCHEN]),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut out: Vec<KitchenItem> = store
        .query(&query)?
        .rows
        .iter()
        .filter(|o| o.get(LOG).is_null() && !o.get(KITCHEN).is_null())
        .filter(|o| !is_template(o) && !is_message(o) && !is_proposal(o))
        .map(|o| KitchenItem { id: o.id.to_string(), title: o.title.clone().unwrap_or_default() })
        .collect();
    out.sort_by_key(|k| k.title.to_lowercase());
    Ok(out)
}

// ── routines ─────────────────────────────────────────────────────────────────────────────────────

/// A routine's notes share one word under this key: the routine itself and each of its sets.
pub const ROUTINE: &str = "routine";
/// The days of the week a routine is meant for, on its own note: `mon,thu`.
pub const WEEKDAYS: &str = "weekdays";

/// A session kept under a name, to plan again on the days it is meant for.
#[derive(Debug, Serialize)]
pub struct Routine {
    /// The word its notes share.
    pub routine: String,
    /// The routine's own note.
    pub id: String,
    pub title: String,
    /// `mon` … `sun`, as written on the note; empty when it is meant for no day in particular.
    pub weekdays: Vec<String>,
    /// Its sets, in the order they were made.
    pub sets: Vec<ObjectMeta>,
}

/// The routines kept in this log, by name.
///
/// **Stored as a recipe is** (`decisions.md#data`, 2026-10-12, *a routine is a session kept under
/// a name*): the routine's own note is an ordinary note carrying a word under `routine` and the
/// weekdays it is meant for; each of its sets is an entry of the log that names the routine by
/// that word **instead of a day**, and so is in no day and counts towards nothing until it is
/// planned onto one.
pub fn log_routines(store: &dyn Store, def: &TotalsDef) -> Result<Vec<Routine>, StoreError> {
    let query = Query {
        filter: log_with_keyed_base(&def.log, ROUTINE),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut heads: BTreeMap<String, Object> = BTreeMap::new();
    let mut parts: BTreeMap<String, Vec<Object>> = BTreeMap::new();
    for o in store.query(&query)?.rows {
        if o.start.is_some() || is_template(&o) || is_message(&o) || is_proposal(&o) {
            continue;
        }
        let PropertyValue::Text(word) = o.get(ROUTINE) else { continue };
        if word.is_empty() {
            continue;
        }
        if o.get(LOG).is_null() {
            heads.insert(word, o);
        } else if is_log_of(&o, &def.log) {
            parts.entry(word).or_default().push(o);
        }
    }
    let mut out: Vec<Routine> = heads
        .into_iter()
        .map(|(word, head)| {
            let weekdays = match head.get(WEEKDAYS) {
                PropertyValue::Text(s) => s
                    .split(',')
                    .map(|d| d.trim().to_lowercase())
                    .filter(|d| !d.is_empty())
                    .collect(),
                _ => Vec::new(),
            };
            Routine {
                sets: parts
                    .remove(&word)
                    .unwrap_or_default()
                    .iter()
                    .map(ObjectMeta::from)
                    .collect(),
                routine: word,
                id: head.id.to_string(),
                title: head.title.clone().unwrap_or_default(),
                weekdays,
            }
        })
        .collect();
    out.sort_by_key(|r| r.title.to_lowercase());
    Ok(out)
}

// ── writing ──────────────────────────────────────────────────────────────────────────────────────

/// A note to make. Everything but the title is optional.
#[derive(Debug, Deserialize)]
pub struct NoteDraft {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

/// A JSON value as the text `apply_property` types — so a number sent by the UI is typed exactly
/// as the same number written in a file would be. **A number must be whole**: `12.5` would be kept
/// as text and silently left out of every total.
fn as_written(key: &str, v: &serde_json::Value) -> Result<String, StoreError> {
    use serde_json::Value;
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) if n.is_i64() || n.is_u64() => Ok(n.to_string()),
        Value::Number(n) => Err(StoreError::Parse(format!(
            "`{key}` must be a whole number, not {n} — use a smaller unit"
        ))),
        Value::Null => Ok(String::new()),
        _ => Err(StoreError::Parse(format!("`{key}` must be text, a whole number or true/false"))),
    }
}

fn build(draft: &NoteDraft, vault: &str) -> Result<Object, StoreError> {
    let mut obj = Object::new(Kind::Note, draft.body.clone());
    obj.vault = vault.to_string();
    obj.title = Some(draft.title.trim().to_string()).filter(|t| !t.is_empty());
    obj.tags = draft.tags.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    if !draft.start.trim().is_empty() {
        apply_property(&mut obj, "start", &draft.start)?;
    }
    for (key, value) in &draft.props {
        let text = as_written(key, value)?;
        if key == LOG {
            // The one place `log` is set: `apply_property` refuses it everywhere else, which is
            // what stops a board drag hiding a note. Shape-tested here for the same reason the
            // exclusion is.
            let kind = fm_model::parse_log_kind(text.trim()).ok_or_else(|| {
                StoreError::Parse(format!(
                    "`log` must be one short lower-case word, like `food` — not '{text}'"
                ))
            })?;
            obj.extra.insert(LOG.to_string(), PropertyValue::Text(kind.to_string()));
        } else {
            apply_property(&mut obj, key, &text)?;
        }
    }
    Ok(obj)
}

/// Make several notes in one call — each built whole and written once, title, tags, day and
/// properties together.
///
/// It is plural because the things that need it are: copying yesterday's breakfast, logging a
/// saved meal, starting a session. Each is several notes and one thing a person did.
///
/// **Every draft is checked before any note is written**, so a refused value writes nothing. The
/// writes themselves are one after another under one lock; a disk failing half-way would leave the
/// first of them behind, and the error says how many.
/// **An entry of the food list keeps its food and its amount, and no copy of their numbers**
/// (`decisions.md#data`, 2026-10-29, *every view sums from the food list*): every view works them
/// out from the list when it reads the entry (`packs::resolve`), so a copy could only go stale —
/// as choline did, for eggs logged after the list had it. Whatever a client sends, numbers are not
/// written for such an entry; a packet from its label, a recipe's serving or a food typed in by hand
/// keeps its own.
pub fn food_and_amount_only(o: &mut Object) {
    let PropertyValue::Text(src) = o.get("source") else { return };
    if !src.starts_with("foods@") || !matches!(o.get("amount_g"), PropertyValue::Int(_)) {
        return;
    }
    o.extra.retain(|k, _| !is_amount_of_a_nutrient(k) && k != "borrowed");
}

/// `kcal`, or a key in a nutrient's unit (`protein_mg`, `iron_ug`, `vitamin_d_ng`) — never the
/// amount eaten.
pub fn is_amount_of_a_nutrient(key: &str) -> bool {
    key == "kcal"
        || (!key.starts_with("amount_")
            && (key.ends_with("_mg") || key.ends_with("_ug") || key.ends_with("_ng")))
}

pub fn create_notes(
    store: &mut dyn Store,
    vault: &str,
    drafts: &[NoteDraft],
) -> Result<Vec<ObjectMeta>, StoreError> {
    if drafts.len() > MAX_DRAFTS {
        return Err(StoreError::Parse(format!(
            "{} notes in one write is more than the {MAX_DRAFTS} allowed",
            drafts.len()
        )));
    }
    let mut built: Vec<Object> =
        drafts.iter().map(|d| build(d, vault)).collect::<Result<_, _>>()?;
    built.iter_mut().for_each(food_and_amount_only);
    for (i, obj) in built.iter().enumerate() {
        store.put(obj).map_err(|e| {
            StoreError::Io(format!("{e} — {i} of {} had been written", built.len()))
        })?;
    }
    Ok(built.iter().map(ObjectMeta::from).collect())
}

/// Set several properties of one note in one write, answering what each was (`None` when it had
/// none) in the form this reads back — so undoing is calling it again with the answer.
///
/// An empty value removes a property. Every key goes through `apply_property`, so everything it
/// refuses is refused here too, `log` included; and nothing is written unless all of them pass.
pub fn set_properties(
    store: &mut dyn Store,
    id: &str,
    props: &BTreeMap<String, serde_json::Value>,
) -> Result<BTreeMap<String, Option<String>>, StoreError> {
    let parsed: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(parsed)?.ok_or(StoreError::NotFound(parsed))?;
    let unchanged = obj.clone();
    let mut previous = BTreeMap::new();
    for (key, value) in props {
        let before = obj.get(key);
        previous.insert(key.clone(), (!before.is_null()).then(|| before.display()));
        apply_property(&mut obj, key, &as_written(key, value)?)?;
    }
    food_and_amount_only(&mut obj);
    // Values the note already holds are not a write (`commands::update_body` says why).
    if obj == unchanged {
        return Ok(previous);
    }
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(previous)
}
