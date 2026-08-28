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

/// How a timestamp crosses the wire: RFC 3339, one spelling, everywhere.
///
/// A single function so that one spelling crosses the wire — a stamp formatted two ways is a
/// stamp that compares unequal to itself.
///
/// **It is no longer what the lost-update guard compares.** That was this function's original
/// reason to exist: `update_body` took the caller's `updated` and matched it against the note's.
/// Since 2026-07-18 the token is [`version_of`], a hash of the body, because a stamp only moves
/// when the *writer* bothers to move it and Vim does not. Nothing about the wire format changed,
/// so this doc is all that was left pointing at the old design — and it pointed three other
/// comments, and two UI call sites, at sending a stamp where a hash belongs.
pub fn stamp(t: time::OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

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
    /// Which vault — i.e. which audience — this note belongs to. Derived from where
    /// the file is, never from what it says, so the UI can badge a card "lab" and be
    /// telling the truth about who can see it. Empty in a single-vault setup, where
    /// there is no boundary to draw.
    pub vault: String,
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
            created: stamp(o.created),
            updated: stamp(o.updated),
            tags: o.tags.clone(),
            assets: o.assets.clone(),
            props: o.extra.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect(),
            vault: o.vault.clone(),
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
    /// What this note's body hashed to when it was read — the token an editor sends back as
    /// `update_body`'s `base`, and the whole lost-update guard.
    ///
    /// **A hash and not the `updated` stamp**, which is what this used to be. A stamp only
    /// catches writers that bump it: the app does, and the `.md` merge driver does, but
    /// hand-editing a note in Vim does not — so a Vim edit was invisible to the check, and
    /// `FileStore::put`'s mtime guard is disarmed by the poll's own reindex a few seconds
    /// later. Content is the only thing that cannot lie about whether the body moved.
    pub version: String,
}

/// The version token for a body — see [`NoteDetail::version`].
///
/// sha256 because it is already a dependency and already how this project identifies bytes;
/// the cost is ~2 ms on the largest thing a body ever is (a whiteboard scene), against a
/// 600 ms save debounce that then writes and fsyncs that same body.
pub fn version_of(body: &str) -> String {
    fm_core::blob::sha256_hex(body.as_bytes())
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

/// A conflicted note **and what kind of conflict it is**.
///
/// The kind is the whole point. The UI used to receive a bare `ObjectMeta` and tell the user, in
/// every case, to *"open each one — both versions are marked in the text"*. That is true for exactly
/// one kind of conflict. For a delete/modify there are no markers and never will be (one side has no
/// file, so there is nothing to interleave and the `.md` driver is not even called), so the advice
/// was impossible to follow and the note was a dead end — while its vault committed nothing at all.
/// Found the hard way: a week frozen, 95 notes unrecorded (2026-07-31).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub note: ObjectMeta,
    /// The vault-relative path git is unmerged on. Empty when this came from the body-marker scan
    /// rather than from git (markers left in the text after the index was settled).
    pub path: String,
    pub vault: String,
    /// Git's two-letter code (`UU`, `DU`, `UD`, …), so a bug report can be precise.
    pub code: String,
    /// One plain sentence: what the two sides actually did.
    pub what: String,
    /// Are there `<<<<<<<` markers in the file to edit? When false, editing is **not** a resolution
    /// and the only answers are keep-theirs or keep-mine.
    pub has_markers: bool,
}

/// Notes that exist on disk but are **not in git history**, per vault.
///
/// `commit_all` stages only the paths the app remembers writing, and that memory is per-process — so
/// every note written before the last restart was silently unstageable, permanently. This is what
/// the app forgot, offered back as something the user can act on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unrecorded {
    pub vault: String,
    pub count: usize,
    /// **Split by kind, because the kinds mean opposite things.** `new` notes exist nowhere else if
    /// this vault has no remote; a pile of `modified` ones means something is rewriting notes it did
    /// not need to; `deleted` means the *deletion* is what git has not recorded. The count alone
    /// could not distinguish these, which is exactly why "146 not in history" on the owner's phone
    /// was a number nobody could act on (2026-07-31).
    pub new: usize,
    pub modified: usize,
    pub deleted: usize,
    /// A bounded sample with enough detail to recognise what happened — see [`UNRECORDED_DETAIL`].
    /// The counts above are the complete picture; this is the evidence.
    pub notes: Vec<UnrecordedNote>,
}

/// How many detail rows `unrecorded` carries. Enough to see a pattern (all created in one minute?
/// all modified with the same size?), few enough that a vault with thousands does not build a
/// payload nobody reads — the counts are what answer "how bad".
pub const UNRECORDED_DETAIL: usize = 50;

/// How many outstanding notes are read to count duplicates. Higher than the detail cap because the
/// *count* must cover everything to mean anything; capped so a pathological vault cannot turn one
/// command into ten thousand file reads.
pub const UNRECORDED_SCAN: usize = 2_000;

/// One note git does not have, in enough detail to recognise it without a shell.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnrecordedNote {
    pub id: String,
    pub path: String,
    /// `new` | `modified` | `deleted` — a string on the wire so the UI can group without importing
    /// a Rust enum's numbering, the same way `conflicts` carries its two-letter code.
    pub kind: String,
    /// The note's title, or its first body line. `None` for a deleted note: there is no file to read,
    /// and inventing a name for it would be worse than admitting that.
    pub title: Option<String>,
    /// Size on disk, and when it last changed — the two facts that make a pattern visible. Both
    /// `None` for a deleted note.
    pub bytes: Option<u64>,
    /// When the *file* was last written. A copy, restore or migration resets this for every file at
    /// once, so it does not mean "when the note was made" — see `created`.
    pub modified: Option<String>,
    /// The note's own `created`, from its frontmatter. Survives copying, so this is the one that says
    /// when the note came into being.
    pub created: Option<String>,
    /// **What kind of note this is**: `note`, `message` (a discussion reply), `proposal`, or
    /// `unreadable`. It names the *code path* that wrote it, which a title cannot — and that is the
    /// difference between "the capture path duplicated something" and "the reply path did".
    pub role: String,
    /// **How many of this vault's unrecorded notes share this exact body**, this one included. `1` is
    /// the normal case. Anything higher is the finding: it turns "147 notes not in history" into "9
    /// distinct notes, one of them written 138 times", which is a diagnosis rather than a count.
    pub copies: usize,
}

/// A filesystem timestamp as RFC-3339, for display beside a note. Best-effort: a clock the OS cannot
/// answer for yields an empty string rather than failing the whole report.
pub fn stamp_of(t: std::time::SystemTime) -> String {
    time::OffsetDateTime::from(t).format(&Rfc3339).unwrap_or_default()
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
/// stamp is also exactly what `apply_property` parses back — `Stamp`'s `Display`
/// keeps its time for exactly this reason (a lossy one would erase the time of
/// any timed note dragged between columns).
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
        PropertyValue::Stamp(s) => Value::String(s.to_string()),
        PropertyValue::DateTime(dt) => {
            Value::String(dt.format(&Rfc3339).unwrap_or_default())
        }
        PropertyValue::List(v) => Value::Array(v.iter().map(prop_to_json).collect()),
    }
}
