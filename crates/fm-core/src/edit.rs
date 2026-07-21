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
use fm_model::{Kind, Object, PropertyValue, Stamp};
use std::str::FromStr;

/// Apply `raw` to property `key` on `obj`. Callers stamp `updated` and `put`.
pub fn apply_property(obj: &mut Object, key: &str, raw: &str) -> Result<(), StoreError> {
    let raw = raw.trim();
    let some = |s: &str| (!s.is_empty()).then(|| s.to_string());
    match key {
        "status" => obj.status = some(raw),
        "title" => obj.title = some(raw),
        "type" | "kind" => obj.kind = Kind::from_str(raw).map_err(StoreError::Parse)?,
        "hard" => obj.hard = matches!(raw, "true" | "yes" | "1"),
        // `Stamp` owns the format on both sides now, so the date literal is no
        // longer duplicated across crates — and an optional time comes for free.
        "due" => obj.due = parse_stamp("due", some(raw))?,
        "start" => obj.start = parse_stamp("start", some(raw))?,
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
        // Discussion structure is written by `reply`, never by hand — and this refusal is a
        // safety guard, not tidiness. `board` groups by **any** property and writes the column
        // name back on drop, so without this a board grouped by `thread_of` plus one drag would
        // stamp `thread_of: doing` onto a note. The value would not parse as a note reference,
        // so the views would not hide it — but the note would now claim to be part of a
        // discussion, and a later fix that trusted the key would lose it. Refuse loudly at the
        // one gesture that can reach it.
        "thread_of" | "reply_to" => {
            return Err(StoreError::Parse(format!(
                "`{key}` is discussion structure — reply to a note instead of setting it"
            )))
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

fn parse_stamp(key: &str, raw: Option<String>) -> Result<Option<Stamp>, StoreError> {
    match raw {
        Some(s) => Stamp::from_str(&s).map(Some).map_err(|e| {
            StoreError::Parse(format!("{key} must be YYYY-MM-DD or YYYY-MM-DDTHH:MM: {e}"))
        }),
        None => Ok(None),
    }
}
