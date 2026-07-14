//! Pure data model for formicarium. No filesystem, no database — this crate is
//! part of the compile-time guarantee that the query engine never touches I/O.

use std::collections::BTreeMap;
use time::{Date, OffsetDateTime};
use ulid::Ulid;

pub mod schema;

/// Stable object identity. A ULID: time-sortable and rename-proof. Links point
/// at ids, never filenames — this is what kills the rename problem outright.
pub type Id = Ulid;

/// The deliberately-small set of object types. A `Task` is a `Note` with a
/// `status` and optionally a `due`; resist adding types — add properties instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Kind {
    Note,
    Task,
    Meeting,
    Asset,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Note => "note",
            Kind::Task => "task",
            Kind::Meeting => "meeting",
            Kind::Asset => "asset",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "note" => Ok(Kind::Note),
            "task" => Ok(Kind::Task),
            "meeting" => Ok(Kind::Meeting),
            "asset" => Ok(Kind::Asset),
            other => Err(format!("unknown kind: {other}")),
        }
    }
}

/// A typed property value — what the query engine filters, sorts and groups on.
///
/// Ordering and hashing are *derived* (variant order, then inner value), so a
/// value can be a group-by map key and a sort key with no bespoke logic. There
/// is deliberately no float variant, which keeps `Ord`/`Eq`/`Hash` total.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
    Date(Date),
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
            PropertyValue::Date(d) => d.to_string(),
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
    pub due: Option<Date>,
    pub hard: bool,
    pub created: OffsetDateTime,
    pub updated: OffsetDateTime,
    pub tags: Vec<String>,
    pub assets: Vec<String>,
    pub code: Vec<String>,
    pub body: String,
    pub extra: BTreeMap<String, PropertyValue>,
}

impl Object {
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
            "due" => self.due.map(PropertyValue::Date).unwrap_or(PropertyValue::Null),
            "hard" => PropertyValue::Bool(self.hard),
            "created" => PropertyValue::DateTime(self.created),
            "updated" => PropertyValue::DateTime(self.updated),
            "tags" => list_text(&self.tags),
            "assets" => list_text(&self.assets),
            "code" => list_text(&self.code),
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
