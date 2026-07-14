//! Pure query engine: filter, sort, and *generic* group-by. Depends only on
//! `fm-model`. It has no dependency on `rusqlite`, `std::fs`, or any path type —
//! seam 1 is a compile-time guarantee, not a convention.
//!
//! The load-bearing trick: **full-text search is just another predicate**
//! ([`Predicate::Text`]). Here it is a substring scan; `FileStore` implements the
//! same contract with SQLite FTS5. Both satisfy one interface, so search is not
//! special and storage stays swappable.

use fm_model::{Kind, Object, PropertyValue};
use indexmap::IndexMap;
use time::Date;

/// A full query: what to keep, how to order, whether to group, and a page window.
#[derive(Clone, Debug, Default)]
pub struct Query {
    pub filter: Filter,
    pub sort: Vec<SortKey>,
    /// Group rows by *any* property name. The renderer receives opaque groups
    /// and never learns what the property is — that is the whole thesis.
    pub group_by: Option<String>,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Conjunction (AND) of predicates.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub all: Vec<Predicate>,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }
    /// Builder sugar: `Filter::new().and(p1).and(p2)`.
    pub fn and(mut self, p: Predicate) -> Self {
        self.all.push(p);
        self
    }
}

#[derive(Clone, Debug)]
pub enum Predicate {
    /// `type ∈ {..}`
    Kind(Vec<Kind>),
    /// A comparison against any property (well-known or custom).
    Prop { key: String, op: Op, value: PropertyValue },
    /// `tags ⊇ set` (every listed tag present).
    TagsAll(Vec<String>),
    /// `tags ∩ set ≠ ∅` (any listed tag present).
    TagsAny(Vec<String>),
    /// A date property within an inclusive `[from, to]` window (open-ended if None).
    DateRange { key: String, from: Option<Date>, to: Option<Date> },
    /// Full-text predicate. Substring scan here; FTS5 in `FileStore`.
    Text(String),
    Not(Box<Predicate>),
    /// Disjunction (OR) of predicates.
    Any(Vec<Predicate>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    /// Property is present (non-null). `value` is ignored.
    Exists,
}

#[derive(Clone, Debug)]
pub struct SortKey {
    pub key: String,
    pub dir: Dir,
}

impl SortKey {
    pub fn asc(key: impl Into<String>) -> Self {
        SortKey { key: key.into(), dir: Dir::Asc }
    }
    pub fn desc(key: impl Into<String>) -> Self {
        SortKey { key: key.into(), dir: Dir::Desc }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Asc,
    Desc,
}

/// A group of rows sharing one property value — a board column, a gallery
/// section. `key`/`label` are opaque to the renderer.
#[derive(Clone, Debug)]
pub struct Group {
    pub key: PropertyValue,
    pub label: String,
    pub rows: Vec<Object>,
}

/// The result of a query: a paginated flat page, optional groups (all rows,
/// ungated by the page window), and the total match count before pagination.
#[derive(Clone, Debug)]
pub struct QueryResult {
    pub rows: Vec<Object>,
    pub groups: Option<Vec<Group>>,
    pub total: usize,
}

/// Run a query over a slice of objects. Pure and filesystem-free: the one stable
/// core every storage backend shares.
pub fn run(query: &Query, objects: &[Object]) -> QueryResult {
    let mut matched: Vec<&Object> =
        objects.iter().filter(|o| matches(&query.filter, o)).collect();

    sort_rows(&mut matched, &query.sort);
    let total = matched.len();

    let groups = query.group_by.as_deref().map(|key| group_rows(&matched, key));

    let start = query.offset.min(matched.len());
    let end = match query.limit {
        Some(limit) => start.saturating_add(limit).min(matched.len()),
        None => matched.len(),
    };
    let rows = matched[start..end].iter().map(|o| (*o).clone()).collect();

    QueryResult { rows, groups, total }
}

fn matches(filter: &Filter, obj: &Object) -> bool {
    filter.all.iter().all(|p| eval(p, obj))
}

fn eval(pred: &Predicate, obj: &Object) -> bool {
    match pred {
        Predicate::Kind(kinds) => kinds.contains(&obj.kind),
        Predicate::Prop { key, op, value } => eval_prop(&obj.get(key), *op, value),
        Predicate::TagsAll(tags) => tags.iter().all(|t| obj.tags.contains(t)),
        Predicate::TagsAny(tags) => tags.iter().any(|t| obj.tags.contains(t)),
        Predicate::DateRange { key, from, to } => in_date_range(&obj.get(key), from, to),
        Predicate::Text(needle) => {
            obj.searchable_text().to_lowercase().contains(&needle.to_lowercase())
        }
        Predicate::Not(inner) => !eval(inner, obj),
        Predicate::Any(preds) => preds.iter().any(|p| eval(p, obj)),
    }
}

fn eval_prop(actual: &PropertyValue, op: Op, expected: &PropertyValue) -> bool {
    match op {
        Op::Exists => !actual.is_null(),
        Op::Eq => actual == expected,
        Op::Ne => actual != expected,
        Op::Lt => actual < expected,
        Op::Lte => actual <= expected,
        Op::Gt => actual > expected,
        Op::Gte => actual >= expected,
    }
}

fn in_date_range(actual: &PropertyValue, from: &Option<Date>, to: &Option<Date>) -> bool {
    let d = match actual {
        PropertyValue::Date(d) => *d,
        PropertyValue::DateTime(dt) => dt.date(),
        _ => return false,
    };
    if let Some(f) = from {
        if d < *f {
            return false;
        }
    }
    if let Some(t) = to {
        if d > *t {
            return false;
        }
    }
    true
}

fn sort_rows(rows: &mut [&Object], keys: &[SortKey]) {
    if keys.is_empty() {
        return;
    }
    rows.sort_by(|a, b| {
        for k in keys {
            let ord = a.get(&k.key).cmp(&b.get(&k.key));
            let ord = match k.dir {
                Dir::Asc => ord,
                Dir::Desc => ord.reverse(),
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn group_rows(rows: &[&Object], key: &str) -> Vec<Group> {
    // IndexMap preserves first-seen order, so columns appear in a stable order.
    let mut buckets: IndexMap<PropertyValue, Vec<Object>> = IndexMap::new();
    for o in rows {
        buckets.entry(o.get(key)).or_default().push((*o).clone());
    }
    buckets
        .into_iter()
        .map(|(k, rows)| Group { label: k.display(), key: k, rows })
        .collect()
}
