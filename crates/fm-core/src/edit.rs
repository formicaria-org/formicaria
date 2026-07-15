//! Write-side property mutation: parse a string value into the correct typed
//! field of an `Object`. This is shared by two callers on purpose — `fm set`
//! (CLI) and the app's `set_property` command (the board's drag write-back) —
//! so a dragged card and a hand-typed `fm set` change the file *identically*.
//! The drag is not a special path; it is a `set_property`.
//!
//! It is the write-side mirror of [`fm_model::Object::get`]: well-known keys
//! become typed fields; anything else is a custom `extra` property. An empty
//! value clears an optional or custom property (dragging a card to the "(none)"
//! column clears the grouped property).

use crate::StoreError;
use fm_model::{Kind, Object, PropertyValue};
use std::str::FromStr;
use time::macros::format_description;
use time::Date;

/// Apply `raw` to property `key` on `obj`. Callers stamp `updated` and `put`.
pub fn apply_property(obj: &mut Object, key: &str, raw: &str) -> Result<(), StoreError> {
    let raw = raw.trim();
    let some = |s: &str| (!s.is_empty()).then(|| s.to_string());
    match key {
        "status" => obj.status = some(raw),
        "title" => obj.title = some(raw),
        "type" | "kind" => obj.kind = Kind::from_str(raw).map_err(StoreError::Parse)?,
        "hard" => obj.hard = matches!(raw, "true" | "yes" | "1"),
        "due" => {
            obj.due = match some(raw) {
                Some(s) => Some(
                    Date::parse(&s, &format_description!("[year]-[month]-[day]"))
                        .map_err(|e| StoreError::Parse(format!("due must be YYYY-MM-DD: {e}")))?,
                ),
                None => None,
            }
        }
        "start" => {
            obj.start = match some(raw) {
                Some(s) => Some(
                    Date::parse(&s, &format_description!("[year]-[month]-[day]"))
                        .map_err(|e| StoreError::Parse(format!("start must be YYYY-MM-DD: {e}")))?,
                ),
                None => None,
            }
        }
        "tags" => {
            obj.tags = raw
                .split([',', ' '])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(String::from)
                .collect()
        }
        "id" | "created" | "updated" | "schema" => {
            return Err(StoreError::Parse(format!("`{key}` is not editable")))
        }
        other => {
            if raw.is_empty() {
                obj.extra.remove(other);
            } else {
                obj.extra.insert(other.to_string(), PropertyValue::Text(raw.to_string()));
            }
        }
    }
    Ok(())
}
