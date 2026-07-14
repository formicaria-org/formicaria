//! Markdown file <-> Object: YAML frontmatter + body.
//!
//! The frontmatter is handled as a generic YAML *mapping*, not a fixed struct,
//! so that **custom** properties a user hand-adds (in Vim, or via `fm set`)
//! survive a read/edit/write cycle. Dropping unknown keys would be silent data
//! loss in a files-as-truth tool. Well-known keys are lifted into typed `Object`
//! fields; every other key lands in `Object::extra` and is written out again.
//!
//! Keys are emitted in a fixed order — well-known first (in a stable order), then
//! custom keys sorted (`extra` is a `BTreeMap`) — so `to_file` is a fixed point
//! on its own output. That idempotence is the byte-round-trip invariant: a file
//! fm wrote, parsed and re-serialized, is byte-identical.
//!
//! Dates carry as strings at the YAML layer to sidestep `time`'s serde wiring;
//! typed conversion happens at this boundary.

use fm_model::{Id, Kind, Object, PropertyValue};
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeMap;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime};

// ISO date (`YYYY-MM-DD`) for the `due` property. Inlined at each use site to
// avoid naming time's format-item type (which churns across versions).
macro_rules! date_fmt {
    () => {
        format_description!("[year]-[month]-[day]")
    };
}

#[derive(Debug)]
pub enum ParseError {
    NoFrontmatter,
    Yaml(String),
    Field(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::NoFrontmatter => write!(f, "missing `---` frontmatter fence"),
            ParseError::Yaml(e) => write!(f, "yaml: {e}"),
            ParseError::Field(e) => write!(f, "field: {e}"),
        }
    }
}

/// Serialize an object to its on-disk Markdown form: `---\n<yaml>---\n<body>`.
pub fn to_file(obj: &Object) -> Result<String, ParseError> {
    let mut map = Mapping::new();
    // Well-known keys in a fixed order; optional/empty ones are omitted.
    map.insert("schema".into(), Value::from(u64::from(fm_model::schema::SCHEMA_VERSION)));
    map.insert("id".into(), Value::from(obj.id.to_string()));
    map.insert("type".into(), Value::from(obj.kind.as_str()));
    if let Some(t) = &obj.title {
        map.insert("title".into(), Value::from(t.clone()));
    }
    if let Some(s) = &obj.status {
        map.insert("status".into(), Value::from(s.clone()));
    }
    if let Some(d) = obj.due {
        let s = d.format(&date_fmt!()).map_err(|e| ParseError::Field(e.to_string()))?;
        map.insert("due".into(), Value::from(s));
    }
    if obj.hard {
        map.insert("hard".into(), Value::from(true));
    }
    map.insert("created".into(), Value::from(fmt_dt(obj.created)?));
    map.insert("updated".into(), Value::from(fmt_dt(obj.updated)?));
    if !obj.tags.is_empty() {
        map.insert("tags".into(), str_seq(&obj.tags));
    }
    if !obj.assets.is_empty() {
        map.insert("assets".into(), str_seq(&obj.assets));
    }
    if !obj.code.is_empty() {
        map.insert("code".into(), str_seq(&obj.code));
    }
    // Custom properties, in BTreeMap (sorted) order, after the well-known keys.
    for (k, v) in &obj.extra {
        map.insert(Value::from(k.clone()), prop_to_yaml(v));
    }

    let yaml = serde_yaml_ng::to_string(&Value::Mapping(map))
        .map_err(|e| ParseError::Yaml(e.to_string()))?;
    Ok(format!("---\n{yaml}---\n{}", obj.body))
}

/// Parse an on-disk Markdown file back into an object. Unknown frontmatter keys
/// are preserved in `Object::extra`; only the first `\n---\n` after the opening
/// fence closes the frontmatter, so a `---` line inside the body is safe.
pub fn from_file(text: &str) -> Result<Object, ParseError> {
    let rest = text.strip_prefix("---\n").ok_or(ParseError::NoFrontmatter)?;
    let end = rest.find("\n---\n").ok_or(ParseError::NoFrontmatter)?;
    let yaml = &rest[..end];
    let body = &rest[end + "\n---\n".len()..];

    let value: Value =
        serde_yaml_ng::from_str(yaml).map_err(|e| ParseError::Yaml(e.to_string()))?;
    let map = match value {
        Value::Mapping(m) => m,
        Value::Null => Mapping::new(),
        _ => return Err(ParseError::Field("frontmatter is not a mapping".into())),
    };

    let mut id = None;
    let mut kind = None;
    let mut title = None;
    let mut status = None;
    let mut due = None;
    let mut hard = false;
    let mut created = None;
    let mut updated = None;
    let mut tags = Vec::new();
    let mut assets = Vec::new();
    let mut code = Vec::new();
    let mut extra = BTreeMap::new();

    for (k, v) in map {
        let key = match k {
            Value::String(s) => s,
            _ => continue, // frontmatter keys are strings; ignore anything else
        };
        match key.as_str() {
            "schema" => {} // presence validated implicitly; Object carries no schema field
            "id" => id = as_string(v),
            "type" => kind = as_string(v),
            "title" => title = as_string(v),
            "status" => status = as_string(v),
            "due" => due = as_string(v),
            "hard" => hard = matches!(v, Value::Bool(true)),
            "created" => created = as_string(v),
            "updated" => updated = as_string(v),
            "tags" => tags = as_string_seq(v),
            "assets" => assets = as_string_seq(v),
            "code" => code = as_string_seq(v),
            _ => {
                extra.insert(key, yaml_to_prop(&v));
            }
        }
    }

    let id = id.ok_or_else(|| ParseError::Field("missing id".into()))?;
    let kind = kind.ok_or_else(|| ParseError::Field("missing type".into()))?;
    let created = created.ok_or_else(|| ParseError::Field("missing created".into()))?;
    let updated = updated.ok_or_else(|| ParseError::Field("missing updated".into()))?;

    Ok(Object {
        id: Id::from_str(&id).map_err(|e| ParseError::Field(format!("id: {e}")))?,
        kind: Kind::from_str(&kind).map_err(ParseError::Field)?,
        title,
        status,
        due: match due {
            Some(s) => Some(
                Date::parse(&s, &date_fmt!()).map_err(|e| ParseError::Field(format!("due: {e}")))?,
            ),
            None => None,
        },
        hard,
        created: OffsetDateTime::parse(&created, &Rfc3339)
            .map_err(|e| ParseError::Field(format!("created: {e}")))?,
        updated: OffsetDateTime::parse(&updated, &Rfc3339)
            .map_err(|e| ParseError::Field(format!("updated: {e}")))?,
        tags,
        assets,
        code,
        body: body.to_string(),
        extra,
    })
}

fn fmt_dt(dt: OffsetDateTime) -> Result<String, ParseError> {
    dt.format(&Rfc3339).map_err(|e| ParseError::Field(e.to_string()))
}

fn str_seq(items: &[String]) -> Value {
    Value::Sequence(items.iter().map(|s| Value::from(s.clone())).collect())
}

fn as_string(v: Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s),
        _ => None,
    }
}

fn as_string_seq(v: Value) -> Vec<String> {
    match v {
        Value::Sequence(items) => items.into_iter().filter_map(as_string).collect(),
        _ => Vec::new(),
    }
}

/// PropertyValue -> YAML for a custom key. No float variant exists in the model,
/// so dates/datetimes serialize as their ISO strings (they come back as `Text`).
fn prop_to_yaml(p: &PropertyValue) -> Value {
    match p {
        PropertyValue::Null => Value::Null,
        PropertyValue::Bool(b) => Value::from(*b),
        PropertyValue::Int(i) => Value::from(*i),
        PropertyValue::Text(s) => Value::from(s.clone()),
        PropertyValue::Date(d) => Value::from(d.to_string()),
        PropertyValue::DateTime(dt) => Value::from(dt.format(&Rfc3339).unwrap_or_default()),
        PropertyValue::List(v) => Value::Sequence(v.iter().map(prop_to_yaml).collect()),
    }
}

/// YAML -> PropertyValue for a custom key. Integers become `Int`; anything the
/// model can't type precisely (floats, nested maps) is kept as `Text` so the
/// value round-trips rather than being lost.
fn yaml_to_prop(v: &Value) -> PropertyValue {
    match v {
        Value::Null => PropertyValue::Null,
        Value::Bool(b) => PropertyValue::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => PropertyValue::Int(i),
            None => PropertyValue::Text(n.to_string()),
        },
        Value::String(s) => PropertyValue::Text(s.clone()),
        Value::Sequence(seq) => PropertyValue::List(seq.iter().map(yaml_to_prop).collect()),
        _ => PropertyValue::Text(
            serde_yaml_ng::to_string(v).unwrap_or_default().trim_end().to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_is_lossless_for_known_fields() {
        let mut o = Object::new(Kind::Task, "trust region clipping\n\nmore body");
        o.status = Some("doing".into());
        o.due = Some(time::macros::date!(2026 - 07 - 20));
        o.hard = true;
        o.tags = vec!["meta-rl".into()];

        let text = to_file(&o).unwrap();
        assert!(text.starts_with("---\n"));
        let back = from_file(&text).unwrap();

        assert_eq!(back.id, o.id);
        assert_eq!(back.kind, o.kind);
        assert_eq!(back.status, o.status);
        assert_eq!(back.due, o.due);
        assert_eq!(back.hard, o.hard);
        assert_eq!(back.tags, o.tags);
        assert_eq!(back.body, o.body);
    }

    #[test]
    fn custom_properties_round_trip_and_are_queryable() {
        let mut o = Object::new(Kind::Note, "body");
        o.extra.insert("project".into(), PropertyValue::Text("alpha".into()));
        o.extra.insert("count".into(), PropertyValue::Int(3));
        o.extra.insert("reviewed".into(), PropertyValue::Bool(true));
        o.extra.insert(
            "collab".into(),
            PropertyValue::List(vec![
                PropertyValue::Text("a".into()),
                PropertyValue::Text("b".into()),
            ]),
        );

        let back = from_file(&to_file(&o).unwrap()).unwrap();
        assert_eq!(back.extra.get("project"), Some(&PropertyValue::Text("alpha".into())));
        assert_eq!(back.extra.get("count"), Some(&PropertyValue::Int(3)));
        assert_eq!(back.extra.get("reviewed"), Some(&PropertyValue::Bool(true)));
        // Reachable through the same generic accessor the query engine uses.
        assert_eq!(back.get("project"), PropertyValue::Text("alpha".into()));
        assert_eq!(back.get("collab").display(), "a, b");
    }

    #[test]
    fn serialization_is_idempotent_byte_for_byte() {
        let mut o = Object::new(Kind::Task, "line one\n---\nafter a fence line\ncafé ☕\n");
        o.title = Some("Idempotence".into());
        o.status = Some("doing".into());
        o.due = Some(time::macros::date!(2026 - 07 - 20));
        o.hard = true;
        o.tags = vec!["a".into(), "b".into()];
        o.extra.insert("project".into(), PropertyValue::Text("alpha".into()));
        o.extra.insert("count".into(), PropertyValue::Int(3));

        let bytes1 = to_file(&o).unwrap();
        let parsed = from_file(&bytes1).unwrap();
        let bytes2 = to_file(&parsed).unwrap();
        assert_eq!(bytes1, bytes2, "to_file must be a fixed point on its own output");
        assert_eq!(parsed.body, o.body, "body preserved, incl. the --- line and unicode");
    }
}
