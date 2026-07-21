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

/// How an id-valued **property** spells a pointer at another note: `note:<ULID>`.
///
/// Two reasons it is not a bare ULID, both learned the hard way.
///
/// 1. **`refs::strip_cross_vault` keys off this prefix.** It is what stops `copy_note` carrying
///    a pointer into another vault's permanent git history (`decisions.md`, "a copy can never
///    point outside its new vault"). A bare ULID is invisible to it, so a property holding one
///    silently defeats a guarantee the product makes in writing.
/// 2. **It is the same spelling the body already uses**, so the reverse index, when it lands,
///    sees frontmatter pointers through the machinery it already needs for `[..](note:<ulid>)`
///    — one discovery mechanism rather than two.
pub fn note_ref(id: Id) -> String {
    format!("note:{id}")
}

/// The inverse of [`note_ref`], and **the only way to compare one**.
///
/// Never compare these as strings. `Ulid`'s parse is case-insensitive over Crockford base32,
/// so a hand-typed lowercase pointer names the same note as an uppercase one — but
/// `"note:01arz…" != "note:01ARZ…"` as text, which would silently orphan the message from its
/// own thread. Parse both sides, compare `Id`s.
///
/// Returns `None` for anything that is not a well-formed reference, which is what lets a
/// caller distinguish "this property points at a note" from "a user typed a word here".
pub fn parse_note_ref(s: &str) -> Option<Id> {
    s.strip_prefix("note:")?.trim().parse().ok()
}

/// A pointer to a git branch: `branch:<name>`. The durable frontmatter of a *proposal* note
/// (`proposes: branch:<name>`) — a proposal is a branch plus a note that carries the discussion.
///
/// It is deliberately the same `<kind>:<value>` shape as [`note_ref`], for the same reason:
/// a value hides a proposal note from the planning views only when it *parses as one of these*,
/// so a stray word a board drag might write into the key can never erase a note. See
/// [`parse_branch_ref`].
pub fn branch_ref(name: &str) -> String {
    format!("branch:{name}")
}

/// The inverse of [`branch_ref`], and the guard that distinguishes "this note proposes a branch"
/// from "a user typed a word into `proposes`".
///
/// Returns the branch name only for a well-formed `branch:<name>` with a non-empty name and no
/// whitespace/control characters (git branch names have none). **This is a shape test, not a
/// git-validity test** — whether the branch actually exists, or is a legal ref, is the write
/// half's concern; here all that matters is that a bare column-name value never reads as a
/// pointer (the same hazard [`parse_note_ref`] guards, one property over).
pub fn parse_branch_ref(s: &str) -> Option<&str> {
    let name = s.strip_prefix("branch:")?.trim();
    if name.is_empty() || name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod ref_tests {
    use super::*;

    #[test]
    fn a_note_ref_round_trips_and_is_case_insensitive() {
        let id = Ulid::new();
        assert_eq!(parse_note_ref(&note_ref(id)), Some(id));
        // The case a hand-edited file produces: same note, different spelling.
        assert_eq!(parse_note_ref(&note_ref(id).to_lowercase()), Some(id));
    }

    #[test]
    fn anything_that_is_not_a_reference_is_none() {
        // A bare ULID is deliberately NOT a reference — that spelling is invisible to the
        // cross-vault strip, so accepting it here would re-open the leak.
        assert_eq!(parse_note_ref(&Ulid::new().to_string()), None);
        // And the value a board drag would write, which must never read as a pointer.
        for s in ["doing", "", "note:", "note:not-a-ulid", "2026-07-20"] {
            assert_eq!(parse_note_ref(s), None, "{s:?} must not parse as a note reference");
        }
    }

    #[test]
    fn a_branch_ref_round_trips() {
        assert_eq!(parse_branch_ref(&branch_ref("fix-protocol")), Some("fix-protocol"));
        assert_eq!(parse_branch_ref("branch:feature/rework"), Some("feature/rework"));
        // Leading/trailing space around the name is tolerated (a hand-edited file).
        assert_eq!(parse_branch_ref("branch:  trimmed  "), Some("trimmed"));
    }

    #[test]
    fn a_stray_value_never_reads_as_a_branch() {
        // The column names a board drag would write into `proposes` must never hide the note.
        for s in ["doing", "todo", "", "branch:", "branch:   ", "branch:has space", "2026-07-20"] {
            assert_eq!(parse_branch_ref(s), None, "{s:?} must not parse as a branch reference");
        }
    }
}
