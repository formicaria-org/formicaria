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
    is_log, is_message, is_proposal, is_template, log_base, log_with_programs_base, LOG,
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
    let q = Query {
        filter: log_with_programs_base(&def.log, PROGRAM),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut by_day: BTreeMap<Date, Vec<Object>> = BTreeMap::new();
    let mut programs = Vec::new();
    for o in store.query(&q)?.rows {
        if is_log(&o) {
            if let Some(s) = o.start.filter(|s| s.date >= from && s.date <= to) {
                by_day.entry(s.date).or_default().push(o);
            }
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
        let sums = keys
            .iter()
            .map(|k| {
                let t = tallied.tallies.get(k).cloned().unwrap_or_default();
                (k.clone(), MetricSum { sum: t.sum, n: t.n, odd: t.odd })
            })
            .collect();
        days.push(DayTotals {
            day: day_string(day),
            of: counted.len(),
            sums,
            targets: targets_on(&programs, day, &keys),
            rows: rows.then(|| all.iter().map(ObjectMeta::from).collect()),
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
}

/// The recipes kept in this log, by name, each with the sum of its ingredients.
///
/// A recipe is saved, not eaten: its notes carry `log` and no `start`, so they are in no day and
/// in no planning view. The recipe's note carries `servings`; an ingredient is an entry like any
/// other — a food, an amount, the values that amount had — that names the recipe instead of a day.
///
/// **The sum is of the ingredients as entered**, raw or cooked as the person chose them. Nothing
/// here knows what cooking does to a weight or to a vitamin, and nothing guesses
/// (`decisions.md#data`, 2026-10-11, *a recipe is the sum of what went in*).
pub fn log_recipes(store: &dyn Store, def: &TotalsDef) -> Result<Vec<Recipe>, StoreError> {
    let query = Query {
        filter: log_base(&def.log),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let mut heads: BTreeMap<String, (Object, i64)> = BTreeMap::new();
    let mut parts: BTreeMap<String, Vec<Object>> = BTreeMap::new();
    for o in store.query(&query)?.rows {
        if o.start.is_some() || is_template(&o) {
            continue;
        }
        let PropertyValue::Text(word) = o.get(RECIPE) else { continue };
        if word.is_empty() {
            continue;
        }
        match o.get(SERVINGS) {
            PropertyValue::Int(n) if n >= 1 => {
                heads.insert(word, (o, n));
            }
            _ => parts.entry(word).or_default().push(o),
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
                (k.clone(), MetricSum { sum: t.sum, n: t.n, odd: t.odd })
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
        });
    }
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
    let built: Vec<Object> = drafts.iter().map(|d| build(d, vault)).collect::<Result<_, _>>()?;
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
    let mut previous = BTreeMap::new();
    for (key, value) in props {
        let before = obj.get(key);
        previous.insert(key.clone(), (!before.is_null()).then(|| before.display()));
        apply_property(&mut obj, key, &as_written(key, value)?)?;
    }
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(previous)
}
