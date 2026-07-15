//! Serializable data-transfer objects — the JSON shape the frontend receives.
//!
//! These are deliberately *meta only*: a card carries id, the well-known
//! properties, a one-line body preview, and — crucially — an **open `props`
//! map** of every custom frontmatter property. That open map is what lets a
//! board group by a user-invented property with no backend change: a new key in
//! a note's YAML flows straight through `Object::extra` into `props`. The full
//! body is never shipped in a list; it is fetched per-note when a card is opened.

use fm_model::{Object, PropertyValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::format_description::well_known::Rfc3339;

/// One card. `#[serde(rename = "type")]` matches the frontmatter key name.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: Option<String>,
    /// First non-empty line of the body, truncated — enough to recognize a card.
    pub preview: String,
    pub status: Option<String>,
    pub due: Option<String>,
    pub start: Option<String>,
    pub hard: bool,
    pub created: String,
    pub updated: String,
    pub tags: Vec<String>,
    /// Content-addressed blob references (`sha256:<hex>`) this object points at.
    /// A gallery tile needs its asset's hash to fetch the thumbnail; carried here
    /// exactly like `tags` so no extra fetch is required to render a preview.
    pub assets: Vec<String>,
    /// Every custom property, keyed by name. Flows through untouched.
    pub props: BTreeMap<String, serde_json::Value>,
}

impl From<&Object> for ObjectMeta {
    fn from(o: &Object) -> Self {
        ObjectMeta {
            id: o.id.to_string(),
            kind: o.kind.as_str().to_string(),
            title: o.title.clone(),
            preview: preview(&o.body),
            status: o.status.clone(),
            due: o.due.map(|d| d.to_string()),
            start: o.start.map(|d| d.to_string()),
            hard: o.hard,
            created: o.created.format(&Rfc3339).unwrap_or_default(),
            updated: o.updated.format(&Rfc3339).unwrap_or_default(),
            tags: o.tags.clone(),
            assets: o.assets.clone(),
            props: o.extra.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect(),
        }
    }
}

/// A single note with its full body — the read view's payload. The meta is
/// flattened, so the frontend receives one flat object (id, type, …, body).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteDetail {
    #[serde(flatten)]
    pub meta: ObjectMeta,
    pub body: String,
}

/// A board column = the distinct value of the grouped property, its display
/// label, and the cards under it. `value` is the string the frontend echoes
/// back to `set_property` on drop, so a drop and `fm set` write the same bytes;
/// the "(none)" column's `value` is empty, which clears the property.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub value: String,
    pub label: String,
    pub cards: Vec<ObjectMeta>,
}

/// A board: the property it groups by (opaque — the renderer never learns the
/// name means "status"), plus the columns.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Board {
    pub group_by: String,
    pub columns: Vec<Column>,
}

/// The settable string for a grouped value. `Null` becomes empty (the clear
/// gesture); every other value uses its human display, which for text/int/bool/
/// date is also exactly what `apply_property` parses back.
pub fn value_string(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Null => String::new(),
        other => other.display(),
    }
}

fn preview(body: &str) -> String {
    let line = body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    let mut s: String = line.chars().take(140).collect();
    if line.chars().count() > 140 {
        s.push('…');
    }
    s
}

fn prop_to_json(p: &PropertyValue) -> serde_json::Value {
    use serde_json::Value;
    match p {
        PropertyValue::Null => Value::Null,
        PropertyValue::Bool(b) => Value::Bool(*b),
        PropertyValue::Int(i) => Value::Number((*i).into()),
        PropertyValue::Text(s) => Value::String(s.clone()),
        PropertyValue::Date(d) => Value::String(d.to_string()),
        PropertyValue::DateTime(dt) => {
            Value::String(dt.format(&Rfc3339).unwrap_or_default())
        }
        PropertyValue::List(v) => Value::Array(v.iter().map(prop_to_json).collect()),
    }
}
