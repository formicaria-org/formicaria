//! Markdown file <-> Object: YAML frontmatter + body.
//!
//! Dates are carried as strings in the serde layer to sidestep `time`'s serde
//! format wiring; conversion to typed `Date`/`OffsetDateTime` happens at the
//! boundary. (Custom/`extra` frontmatter properties round-trip in a later slice;
//! S0 only writes the well-known fields.)

use fm_model::{Id, Kind, Object};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
struct Frontmatter {
    schema: u32,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    due: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    hard: bool,
    created: String,
    updated: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    assets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    code: Vec<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
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
    let fm = Frontmatter {
        schema: fm_model::schema::SCHEMA_VERSION,
        id: obj.id.to_string(),
        kind: obj.kind.as_str().to_string(),
        title: obj.title.clone(),
        status: obj.status.clone(),
        due: match obj.due {
            Some(d) => {
                Some(d.format(&date_fmt!()).map_err(|e| ParseError::Field(e.to_string()))?)
            }
            None => None,
        },
        hard: obj.hard,
        created: obj.created.format(&Rfc3339).map_err(|e| ParseError::Field(e.to_string()))?,
        updated: obj.updated.format(&Rfc3339).map_err(|e| ParseError::Field(e.to_string()))?,
        tags: obj.tags.clone(),
        assets: obj.assets.clone(),
        code: obj.code.clone(),
    };
    let yaml = serde_yaml_ng::to_string(&fm).map_err(|e| ParseError::Yaml(e.to_string()))?;
    Ok(format!("---\n{yaml}---\n{}", obj.body))
}

/// Parse an on-disk Markdown file back into an object.
pub fn from_file(text: &str) -> Result<Object, ParseError> {
    let rest = text.strip_prefix("---\n").ok_or(ParseError::NoFrontmatter)?;
    let end = rest.find("\n---\n").ok_or(ParseError::NoFrontmatter)?;
    let yaml = &rest[..end];
    let body = &rest[end + "\n---\n".len()..];

    let fm: Frontmatter =
        serde_yaml_ng::from_str(yaml).map_err(|e| ParseError::Yaml(e.to_string()))?;

    Ok(Object {
        id: Id::from_str(&fm.id).map_err(|e| ParseError::Field(format!("id: {e}")))?,
        kind: Kind::from_str(&fm.kind).map_err(ParseError::Field)?,
        title: fm.title,
        status: fm.status,
        due: match fm.due {
            Some(s) => Some(
                Date::parse(&s, &date_fmt!()).map_err(|e| ParseError::Field(format!("due: {e}")))?,
            ),
            None => None,
        },
        hard: fm.hard,
        created: OffsetDateTime::parse(&fm.created, &Rfc3339)
            .map_err(|e| ParseError::Field(format!("created: {e}")))?,
        updated: OffsetDateTime::parse(&fm.updated, &Rfc3339)
            .map_err(|e| ParseError::Field(format!("updated: {e}")))?,
        tags: fm.tags,
        assets: fm.assets,
        code: fm.code,
        body: body.to_string(),
        extra: BTreeMap::new(),
    })
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
}
