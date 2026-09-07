//! Pure query engine: filter, sort, and *generic* group-by. Depends only on
//! `fm-model`. It has no dependency on `rusqlite`, `std::fs`, or any path type —
//! seam 1 is a compile-time guarantee, not a convention.
//!
//! The load-bearing trick: **full-text search is just another predicate**
//! ([`Predicate::Text`]). Here it is a substring scan; `FileStore` implements the
//! same contract with SQLite FTS5. Both satisfy one interface, so search is not
//! special and storage stays swappable.

use fm_model::{Id, Kind, Object, PropertyValue};
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
    let mut matched: Vec<&Object> = objects.iter().filter(|o| matches(&query.filter, o)).collect();

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

fn group_rows(rows: &[&Object], key: &str) -> Vec<Group> {
    // IndexMap preserves first-seen order, so columns appear in a stable order.
    let mut buckets: IndexMap<PropertyValue, Vec<Object>> = IndexMap::new();
    for o in rows {
        buckets.entry(o.get(key)).or_default().push((*o).clone());
    }
    buckets.into_iter().map(|(k, rows)| Group { label: k.display(), key: k, rows }).collect()
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
