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
use fm_model::{Kind, Object, Stamp};
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
        // **Comma-separated, so a tag may contain a space.** This split on `[',', ' ']` until
        // 2026-08-29, which made a multi-word tag unrepresentable through the only write path the
        // app has: `Machine Learning` silently became two unrelated tags. That blocks every
        // mapping of an external name onto a tag — a Zotero collection called `To Read`, a folder,
        // an imported keyword — and it blocks the board's own drag write-back, which sends a
        // column *name*.
        //
        // A "comma when present, else whitespace" heuristic was tried first and rejected: it left
        // a *single* multi-word tag needing a trailing comma, which is a rule nobody would guess.
        // One separator, the same one `assets`/`code` use, is the rule that needs no explaining.
        // The cost is that `todo urgent` is now one tag rather than two — visible immediately as a
        // single chip, and the field's own placeholder says "comma separated".
        "tags" => obj.tags = split_list(raw),
        // **`assets` and `code` are typed `Vec<String>` fields, and until 2026-08-29 neither had
        // an arm here.** Both fell through to the `extra` catch-all below, which writes a
        // `PropertyValue::Text`; `to_file` then serialised a *scalar* (`assets: sha256:…`) while
        // `from_file` reads them with `as_string_seq`, which answers `Vec::new()` for anything
        // that is not a sequence. So the value was gone on the next load, silently, at every
        // layer — on the one field that ties a note to its blob.
        //
        // **Comma only, deliberately not `tags`' `split([',', ' '])`.** A blob reference or a path
        // may not contain a space, so splitting on one buys nothing here and costs the ability to
        // ever express a value that has one. (That split is also why a multi-word tag is
        // unrepresentable today; the bug should not spread to a second key by imitation.)
        "assets" => obj.assets = split_list(raw),
        "code" => obj.code = split_list(raw),
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
                // **Typed the way the file would have typed it.** Storing every hand-entered value
                // as `Text` meant `year: 2017` was `Text` when the app wrote it and `Int` when a
                // text editor did — and `PropertyValue`'s derived `Ord` compares the variant first,
                // so one vault sorted into two disjoint blocks by nothing but provenance. The
                // inference is lossless-only: see `frontmatter::scalar_property`.
                obj.extra.insert(other.to_string(), crate::frontmatter::scalar_property(raw));
            }
        }
    }
    Ok(())
}

/// A comma-separated list into its parts, empties dropped — so an empty value clears the list,
/// the same way an empty value clears every other optional property.
fn split_list(raw: &str) -> Vec<String> {
    raw.split(',').map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect()
}

fn parse_stamp(key: &str, raw: Option<String>) -> Result<Option<Stamp>, StoreError> {
    match raw {
        Some(s) => Stamp::from_str(&s).map(Some).map_err(|e| {
            StoreError::Parse(format!("{key} must be YYYY-MM-DD or YYYY-MM-DDTHH:MM: {e}"))
        }),
        None => Ok(None),
    }
}
