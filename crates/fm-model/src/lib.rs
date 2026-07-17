//! Pure data model for formicaria. No filesystem, no database — this crate is
//! part of the compile-time guarantee that the query engine never touches I/O.

use std::collections::BTreeMap;
use time::OffsetDateTime;
use ulid::Ulid;

pub mod schema;
pub mod stamp;

pub use stamp::Stamp;

/// Stable object identity. A ULID: time-sortable and rename-proof. Links point
/// at ids, never filenames — this is what kills the rename problem outright.
pub type Id = Ulid;

/// The two object kinds. Everything the user writes is a `Note`; an `Asset` is a
/// note that catalogs an ingested file (its blob + extracted text). There is no
/// task/meeting distinction — differentiate notes with **tags**, not a type.
/// Resist adding kinds — add properties instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Kind {
    Note,
    Asset,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Note => "note",
            Kind::Asset => "asset",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = String;
    /// Lenient by design: only `asset` is a distinct kind; every other value
    /// (including legacy `task`/`meeting` frontmatter) collapses to `Note`, so
    /// old vault notes migrate silently on load.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s == "asset" { Kind::Asset } else { Kind::Note })
    }
}

/// A typed property value — what the query engine filters, sorts and groups on.
///
/// Ordering and hashing are *derived* (variant order, then inner value), so a
/// value can be a group-by map key and a sort key with no bespoke logic. There
/// is deliberately no float variant, which keeps `Ord`/`Eq`/`Hash` total.
///
/// Because `Ord` compares the *variant* before the inner value, two variants
/// that both mean "a moment" would sort as two disjoint blocks — which is why
/// scheduling points are a single `Stamp` (day + optional time) rather than a
/// `Date`/`DateTime` pair. `DateTime` here is only ever an instant we stamped
/// ourselves (`created`/`updated`), never a user-set deadline.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
    Stamp(Stamp),
    DateTime(OffsetDateTime),
    List(Vec<PropertyValue>),
}

impl PropertyValue {
    /// Human-facing label — used for group headers, chips and pills.
    pub fn display(&self) -> String {
        match self {
            PropertyValue::Null => "(none)".to_string(),
            PropertyValue::Bool(b) => b.to_string(),
            PropertyValue::Int(i) => i.to_string(),
            PropertyValue::Text(s) => s.clone(),
            // Lossless on purpose — `display()` is what the board's drag
            // write-back feeds back into `apply_property`, so dropping the time
            // here would erase a meeting's time on every drag.
            PropertyValue::Stamp(s) => s.to_string(),
            PropertyValue::DateTime(dt) => dt.date().to_string(),
            PropertyValue::List(v) => {
                v.iter().map(PropertyValue::display).collect::<Vec<_>>().join(", ")
            }
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PropertyValue::Null)
    }
}

/// One object = one note = one Markdown file. Structure lives in these fields;
/// the body is free text. `extra` carries any *custom* frontmatter property, so
/// grouping/filtering on a user-invented property needs no code change.
#[derive(Clone, PartialEq, Debug)]
pub struct Object {
    pub id: Id,
    pub kind: Kind,
    pub title: Option<String>,
    pub status: Option<String>,
    /// Optional deadline (end of the calendar bar). No default — a fresh note has
    /// none until the user sets one. Carries an optional time (see [`Stamp`]), so
    /// a 15:00 meeting end and an all-day deadline are the same field.
    pub due: Option<Stamp>,
    /// Optional start of the work (left end of the calendar bar). Distinct from
    /// `created` (the creation timestamp) and, like `due`, unset by default.
    pub start: Option<Stamp>,
    pub hard: bool,
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
    pub tags: Vec<String>,
    pub assets: Vec<String>,
    pub code: Vec<String>,
    pub body: String,
    pub extra: BTreeMap<String, PropertyValue>,
    /// Which vault this note lives in — i.e. **who can see it**.
    ///
    /// Derived from location by the store that loaded it, and **never serialized**:
    /// `to_file` has no arm for it and `from_file` strips it, so it cannot be typed
    /// into a file. That is the whole point. A vault is one repo, one remote, one
    /// collaborator list, so *location is the permission*; a frontmatter `access:`
    /// label would have zero enforcement power, and since git history is forever, one
    /// typo would be permanent disclosure to everyone who ever cloned. Making this a
    /// content field would make the permission forgeable — by a typo, or by an agent.
    ///
    /// Empty for an object that came from nowhere in particular (`Object::new`, a
    /// `MemoryStore`): "no audience stated", never "all audiences".
    pub vault: String,
}

impl Object {
    /// A fresh object: a new ULID and `created == updated == now`. Capture writes
    /// a new small file per entry, so file mtime behaves like entry mtime.
    pub fn new(kind: Kind, body: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Object {
            id: Ulid::new(),
            kind,
            title: None,
            status: None,
            due: None,
            start: None,
            hard: false,
            created: now,
            updated: now,
            tags: Vec::new(),
            assets: Vec::new(),
            code: Vec::new(),
            body: body.into(),
            extra: BTreeMap::new(),
            // Not yet anywhere. The store that writes it is what gives it an audience.
            vault: String::new(),
        }
    }

    /// Uniform accessor over well-known fields AND the `extra` map. The query
    /// engine reads *every* property through this and never names a hardcoded
    /// field, which is exactly what makes group-by generic: point a board at
    /// `type` instead of `status` and it just works.
    pub fn get(&self, key: &str) -> PropertyValue {
        match key {
            "id" => PropertyValue::Text(self.id.to_string()),
            "type" | "kind" => PropertyValue::Text(self.kind.as_str().to_string()),
            "title" => opt_text(&self.title),
            "status" => opt_text(&self.status),
            "due" => self.due.map(PropertyValue::Stamp).unwrap_or(PropertyValue::Null),
            "start" => self.start.map(PropertyValue::Stamp).unwrap_or(PropertyValue::Null),
            "hard" => PropertyValue::Bool(self.hard),
            "created" => PropertyValue::DateTime(self.created),
            "updated" => PropertyValue::DateTime(self.updated),
            "tags" => list_text(&self.tags),
            "assets" => list_text(&self.assets),
            "code" => list_text(&self.code),
            // Explicit, and above the `extra` fallback on purpose: without an arm here a
            // hand-typed `vault:` in someone's frontmatter would answer this query, and
            // the permission would be whatever the file claimed. `Predicate::Prop`
            // reaches this, which is what makes "filter/group by vault" free.
            "vault" => PropertyValue::Text(self.vault.clone()),
            other => self.extra.get(other).cloned().unwrap_or(PropertyValue::Null),
        }
    }

    /// Text visible to full-text search: title + body. Extracted asset text
    /// (pdftotext output) is appended here by the indexer in a later slice.
    pub fn searchable_text(&self) -> String {
        match &self.title {
            Some(t) => format!("{t}\n{}", self.body),
            None => self.body.clone(),
        }
    }
}

fn opt_text(o: &Option<String>) -> PropertyValue {
    o.clone().map(PropertyValue::Text).unwrap_or(PropertyValue::Null)
}

fn list_text(v: &[String]) -> PropertyValue {
    PropertyValue::List(v.iter().cloned().map(PropertyValue::Text).collect())
}
