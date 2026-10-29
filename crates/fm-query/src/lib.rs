//! Pure query engine: filter, sort, and *generic* group-by. Depends only on
//! `fm-model`. It has no dependency on `rusqlite`, `std::fs`, or any path type —
//! seam 1 is a compile-time guarantee, not a convention.
//!
//! The load-bearing trick: **full-text search is just another predicate**
//! ([`Predicate::Text`]). Here it is a substring scan; `FileStore` implements the
//! same contract with SQLite FTS5. Both satisfy one interface, so search is not
//! special and storage stays swappable.

pub mod days;

use fm_model::{Id, Kind, Object, PropertyValue};
use indexmap::IndexMap;
use std::collections::BTreeMap;
use time::Date;

mod rank;
pub use rank::{is_cjk, relevance};

/// A full query: what to keep, how to order, whether to group, and a page window.
#[derive(Clone, Debug, Default)]
pub struct Query {
    pub filter: Filter,
    pub sort: Vec<SortKey>,
    /// Group rows by *any* property name. The renderer receives opaque groups
    /// and never learns what the property is — that is the whole thesis.
    pub group_by: Option<String>,
    /// Properties to total, by name — over every matched row, and over each group's rows when
    /// grouping. See [`Tally`]. Empty costs nothing.
    pub tally: Vec<String>,
    pub limit: Option<usize>,
    pub offset: usize,
    /// **Order by how well each row answers this search** ([`relevance`]), before `sort`, which then
    /// only breaks ties. `None` keeps the order `sort` gives. Search sets it to what was typed
    /// (2026-10-29): it is computed here, from the rows, so every store and every vault rank alike.
    pub rank_by: Option<String>,
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
    Prop {
        key: String,
        op: Op,
        value: PropertyValue,
    },
    /// `tags ⊇ set` (every listed tag present).
    TagsAll(Vec<String>),
    /// `tags ∩ set ≠ ∅` (any listed tag present).
    TagsAny(Vec<String>),
    /// A date property within an inclusive `[from, to]` window (open-ended if None).
    DateRange {
        key: String,
        from: Option<Date>,
        to: Option<Date>,
    },
    /// Full-text predicate. Substring scan here; FTS5 in `FileStore`.
    Text(String),
    Not(Box<Predicate>),
    /// Disjunction (OR) of predicates.
    Any(Vec<Predicate>),
    /// `key` holds a well-formed `note:<ULID>` reference — to `id` specifically when given,
    /// or to *any* note when `None`.
    ///
    /// **Why this is not expressible with [`Predicate::Prop`], and why it is one variant and
    /// not two.** It does two jobs that are only correct together:
    ///
    /// - `id: None` asks *"is this value shaped like a note reference?"* — a **type test**.
    ///   `Op` has `Eq/Ne/Lt/…/Exists`, none of which can shape-check an unknown value, so an
    ///   `Exists` test would hide a note the moment a user (or a board drag) put any text in
    ///   the property at all.
    /// - `id: Some(_)` asks *"does it point at this note?"* compared as **parsed `Id`s**, never
    ///   as strings, so a hand-typed lowercase pointer still resolves to its thread.
    ///
    /// Deliberately has no `.view` spelling: a `.view` may only narrow the renderer's base
    /// filter, so nothing is lost by keeping this out of the user-facing DSL.
    NoteRef {
        key: String,
        id: Option<Id>,
    },
    /// `key` holds a well-formed `branch:<name>` reference — the durable frontmatter of a
    /// *proposal* note (`proposes: branch:<name>`).
    ///
    /// The sibling of [`Predicate::NoteRef`] one property over, and it exists for the identical
    /// reason: proposals are a hidden note-class (like messages), so the planning views exclude
    /// them — but only when the value *parses as a branch reference*, never on bare presence.
    /// `Prop{Exists}` would hide any note the instant a board drag wrote a column name into
    /// `proposes`. A branch name is not a ULID, so this cannot reuse `NoteRef`.
    BranchRef {
        key: String,
    },
    /// `key` holds a well-formed log kind (`food`, `workout`) — the mark of a *logged entry*.
    ///
    /// The third sibling of [`Predicate::NoteRef`] and [`Predicate::BranchRef`], for the same
    /// reason: entries are a hidden note-class, hidden only when the value passes a shape test
    /// (`fm_model::parse_log_kind`), never on bare presence.
    LogKind {
        key: String,
    },
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
    /// One [`Tally`] per property named in [`Query::tally`], over this group's rows.
    pub tallies: BTreeMap<String, Tally>,
}

/// What a property adds up to across some rows — the answer to *"how much, today?"*.
///
/// **Whole numbers only, and it says what it left out.** A tracked value is an `Int` whose unit is
/// in its key (`kcal`, `protein_mg`); `12.5` in frontmatter parses as text and is not a number
/// here. A row that carries such a value is **counted in `odd`, never skipped silently** — a total
/// that quietly dropped an entry would be a confident wrong answer, the same failure the
/// `PropertyValue` ordering trap is kept out of `.view` files for.
///
/// `n` is how many rows carried a whole number at all, which is what lets a caller say *"from 9 of
/// 10 entries"* instead of presenting a partial total as a whole one.
///
/// A key beginning `conflict-` is never tallied: that is where a merge keeps the value that lost
/// (`fm-core`'s `merge_objects`), and adding it in would count one entry twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// The sum of the whole numbers. Saturates rather than wrapping.
    pub sum: i64,
    /// The largest of them, or `None` when no row carried one.
    pub max: Option<i64>,
    /// How many rows carried a whole number.
    pub n: usize,
    /// How many rows carried something else under this key.
    pub odd: usize,
}

/// Where a merge keeps a value that lost. Mirrors `fm_core::merge::DEMOTED_PREFIX`; this crate
/// cannot depend on that one, and a test beside that constant holds the two together.
pub const DEMOTED_PREFIX: &str = "conflict-";

/// The result of a query: a paginated flat page, optional groups (all rows,
/// ungated by the page window), and the total match count before pagination.
#[derive(Clone, Debug)]
pub struct QueryResult {
    pub rows: Vec<Object>,
    pub groups: Option<Vec<Group>>,
    pub total: usize,
    /// One [`Tally`] per property named in [`Query::tally`], over **every** matched row — like
    /// `groups` and `total`, not gated by the page window.
    pub tallies: BTreeMap<String, Tally>,
}

/// Run a query over a slice of objects. Pure and filesystem-free: the one stable
/// core every storage backend shares.
pub fn run(query: &Query, objects: &[Object]) -> QueryResult {
    let mut matched: Vec<&Object> = objects.iter().filter(|o| matches(&query.filter, o)).collect();

    sort_rows(&mut matched, &query.sort);
    if let Some(text) = query.rank_by.as_deref().filter(|t| !t.trim().is_empty()) {
        // Stable: rows the search scores alike keep the order `sort` gave them.
        let mut scored: Vec<(u32, &Object)> =
            matched.into_iter().map(|o| (relevance(text, o), o)).collect();
        scored.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
        matched = scored.into_iter().map(|(_, o)| o).collect();
    }
    let total = matched.len();

    let groups = query.group_by.as_deref().map(|key| group_rows(&matched, key, &query.tally));
    let tallies = tally_rows(matched.iter().copied(), &query.tally);

    let start = query.offset.min(matched.len());
    let end = match query.limit {
        Some(limit) => start.saturating_add(limit).min(matched.len()),
        None => matched.len(),
    };
    let rows = matched[start..end].iter().map(|o| (*o).clone()).collect();

    QueryResult { rows, groups, total, tallies }
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
        Predicate::NoteRef { key, id } => match obj.get(key) {
            // Parse, never string-compare: `note:01arz…` and `note:01ARZ…` name the same note,
            // and a value that is not a reference at all (a word a board drag wrote) is not a
            // pointer no matter what it says.
            PropertyValue::Text(s) => match fm_model::parse_note_ref(&s) {
                Some(found) => id.is_none_or(|want| found == want),
                None => false,
            },
            _ => false,
        },
        Predicate::BranchRef { key } => match obj.get(key) {
            // Shape test only: a value counts when it parses as `branch:<name>`, so a stray
            // word (a board drop, a typo) never reads as a proposal pointer.
            PropertyValue::Text(s) => fm_model::parse_branch_ref(&s).is_some(),
            _ => false,
        },
        Predicate::LogKind { key } => match obj.get(key) {
            PropertyValue::Text(s) => fm_model::parse_log_kind(&s).is_some(),
            _ => false,
        },
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

/// Day-granular: a `DateRange` asks "which calendar days", so a stamp's optional
/// time and a timestamp's clock are both narrowed to their day before comparing.
fn in_date_range(actual: &PropertyValue, from: &Option<Date>, to: &Option<Date>) -> bool {
    let d = match actual {
        PropertyValue::Stamp(s) => s.date,
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

fn group_rows(rows: &[&Object], key: &str, tally: &[String]) -> Vec<Group> {
    // IndexMap preserves first-seen order, so columns appear in a stable order.
    let mut buckets: IndexMap<PropertyValue, Vec<Object>> = IndexMap::new();
    for o in rows {
        buckets.entry(o.get(key)).or_default().push((*o).clone());
    }
    buckets
        .into_iter()
        .map(|(k, rows)| Group {
            label: k.display(),
            key: k,
            tallies: tally_rows(rows.iter(), tally),
            rows,
        })
        .collect()
}

/// Total each named property over `rows`. See [`Tally`] for what counts.
fn tally_rows<'a>(
    rows: impl Iterator<Item = &'a Object> + Clone,
    keys: &[String],
) -> BTreeMap<String, Tally> {
    let mut out = BTreeMap::new();
    for key in keys {
        if key.starts_with(DEMOTED_PREFIX) {
            continue;
        }
        let mut t = Tally::default();
        for o in rows.clone() {
            match o.get(key) {
                PropertyValue::Null => {}
                PropertyValue::Int(v) => {
                    t.sum = t.sum.saturating_add(v);
                    t.max = Some(t.max.map_or(v, |m| m.max(v)));
                    t.n += 1;
                }
                _ => t.odd += 1,
            }
        }
        out.insert(key.clone(), t);
    }
    out
}

#[cfg(test)]
mod tally_tests {
    use super::*;
    use fm_model::{Kind, Object};

    fn entry(day: &str, props: &[(&str, PropertyValue)]) -> Object {
        let mut o = Object::new(Kind::Note, "");
        o.extra.insert("day".into(), PropertyValue::Text(day.into()));
        for (k, v) in props {
            o.extra.insert((*k).into(), v.clone());
        }
        o
    }
    fn q(group: bool, keys: &[&str]) -> Query {
        Query {
            group_by: group.then(|| "day".to_string()),
            tally: keys.iter().map(|k| (*k).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_property_is_summed_over_every_row_and_over_each_group() {
        let rows = [
            entry("mon", &[("kcal", PropertyValue::Int(200))]),
            entry("mon", &[("kcal", PropertyValue::Int(350))]),
            entry("tue", &[("kcal", PropertyValue::Int(90))]),
        ];
        let r = run(&q(true, &["kcal"]), &rows);
        assert_eq!(r.tallies["kcal"], Tally { sum: 640, max: Some(350), n: 3, odd: 0 });
        let groups = r.groups.unwrap();
        assert_eq!(groups[0].tallies["kcal"].sum, 550);
        assert_eq!(groups[1].tallies["kcal"], Tally { sum: 90, max: Some(90), n: 1, odd: 0 });
    }

    /// The count is what lets a screen say "from 2 of 3 entries" rather than pass a partial
    /// total off as a whole one.
    #[test]
    fn a_row_without_the_property_is_not_counted_and_not_an_error() {
        let rows = [
            entry(
                "mon",
                &[("kcal", PropertyValue::Int(200)), ("iron_ug", PropertyValue::Int(900))],
            ),
            entry("mon", &[("kcal", PropertyValue::Int(700))]),
        ];
        let r = run(&q(false, &["kcal", "iron_ug"]), &rows);
        assert_eq!(r.tallies["kcal"].n, 2);
        assert_eq!(r.tallies["iron_ug"], Tally { sum: 900, max: Some(900), n: 1, odd: 0 });
    }

    /// `12.5` in frontmatter is text. It must not be added, and it must not vanish.
    #[test]
    fn a_value_that_is_not_a_whole_number_is_reported_never_dropped_silently() {
        let rows = [
            entry("mon", &[("kcal", PropertyValue::Int(200))]),
            entry("mon", &[("kcal", PropertyValue::Text("12.5".into()))]),
            entry("mon", &[("kcal", PropertyValue::Bool(true))]),
        ];
        let r = run(&q(false, &["kcal"]), &rows);
        assert_eq!(r.tallies["kcal"], Tally { sum: 200, max: Some(200), n: 1, odd: 2 });
    }

    /// A merge keeps the losing value under `conflict-<key>`. Adding it in would count one
    /// entry twice.
    #[test]
    fn a_demoted_value_is_never_tallied() {
        let rows = [entry(
            "mon",
            &[("kcal", PropertyValue::Int(200)), ("conflict-kcal", PropertyValue::Int(180))],
        )];
        let r = run(&q(false, &["kcal", "conflict-kcal"]), &rows);
        assert_eq!(r.tallies["kcal"].sum, 200);
        assert!(!r.tallies.contains_key("conflict-kcal"));
    }

    #[test]
    fn nothing_asked_for_is_nothing_computed_and_nothing_matched_is_zero() {
        let rows = [entry("mon", &[("kcal", PropertyValue::Int(200))])];
        assert!(run(&q(true, &[]), &rows).tallies.is_empty());
        let none: [Object; 0] = [];
        let r = run(&q(false, &["kcal"]), &none);
        assert_eq!(r.tallies["kcal"], Tally::default());
    }

    #[test]
    fn a_sum_saturates_instead_of_wrapping() {
        let rows = [
            entry("mon", &[("x", PropertyValue::Int(i64::MAX))]),
            entry("mon", &[("x", PropertyValue::Int(5))]),
        ];
        assert_eq!(run(&q(false, &["x"]), &rows).tallies["x"].sum, i64::MAX);
    }
}

#[cfg(test)]
mod noteref_tests {
    use super::*;
    // `Id` is `fm_model`'s alias for a ULID. Used rather than depending on `ulid` here: the
    // whole point of this crate is that its dependency list stays minimal and storage-free.
    use fm_model::{Id, Kind, Object};

    fn note_with(key: &str, value: &str) -> Object {
        let mut o = Object::new(Kind::Note, "body");
        o.extra.insert(key.into(), PropertyValue::Text(value.into()));
        o
    }

    /// The guard that stops a board drag erasing a note. `board` groups by *any* property and
    /// writes the column name back on drop, so an `Exists` test would hide a note the moment
    /// any text landed in the key. Only a well-formed reference counts.
    #[test]
    fn only_a_well_formed_reference_reads_as_one() {
        let any = |key: &str| Predicate::NoteRef { key: key.into(), id: None };
        let root = Id::new();

        assert!(eval(&any("thread_of"), &note_with("thread_of", &fm_model::note_ref(root))));
        // What a board drop, a typo, or a bare id would put there — none is a pointer.
        for bad in ["doing", "", "note:", "note:nope", &root.to_string()] {
            assert!(
                !eval(&any("thread_of"), &note_with("thread_of", bad)),
                "{bad:?} must not read as a note reference"
            );
        }
        // A non-text value cannot be a pointer either.
        let mut n = Object::new(Kind::Note, "b");
        n.extra.insert("thread_of".into(), PropertyValue::Int(3));
        assert!(!eval(&any("thread_of"), &n));
        // And a note that simply has no such property is not a message.
        assert!(!eval(&any("thread_of"), &Object::new(Kind::Note, "ordinary")));
    }

    /// Targets are compared as parsed ULIDs. A hand-edited lowercase pointer names the same
    /// note; comparing the strings would silently orphan the message from its own thread.
    #[test]
    fn a_target_is_matched_by_parsed_id_not_by_spelling() {
        let root = Id::new();
        let other = Id::new();
        let to_root = Predicate::NoteRef { key: "thread_of".into(), id: Some(root) };

        assert!(eval(&to_root, &note_with("thread_of", &fm_model::note_ref(root))));
        assert!(eval(&to_root, &note_with("thread_of", &fm_model::note_ref(root).to_lowercase())));
        assert!(!eval(&to_root, &note_with("thread_of", &fm_model::note_ref(other))));
    }

    /// The exclusion shape the views use: notes that are NOT messages.
    #[test]
    fn negating_it_gives_the_view_exclusion() {
        let not_a_message =
            Predicate::Not(Box::new(Predicate::NoteRef { key: "thread_of".into(), id: None }));

        assert!(eval(&not_a_message, &Object::new(Kind::Note, "a real note")));
        assert!(!eval(&not_a_message, &note_with("thread_of", &fm_model::note_ref(Id::new()))));
    }

    /// `BranchRef` reads only a well-formed `branch:<name>`, so a proposal is hidden from the
    /// planning views only when it genuinely names a branch — never on a stray property value.
    #[test]
    fn only_a_well_formed_branch_reference_reads_as_one() {
        let proposes = Predicate::BranchRef { key: "proposes".into() };

        assert!(eval(&proposes, &note_with("proposes", &fm_model::branch_ref("fix-protocol"))));
        assert!(eval(&proposes, &note_with("proposes", "branch:feature/x")));
        // The column names a board drop would write, and other non-references.
        for bad in ["doing", "todo", "", "branch:", "branch:has space"] {
            assert!(
                !eval(&proposes, &note_with("proposes", bad)),
                "{bad:?} must not read as a branch reference"
            );
        }
        // A note with no such property is not a proposal.
        assert!(!eval(&proposes, &Object::new(Kind::Note, "ordinary")));
    }
}
