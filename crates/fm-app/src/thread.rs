//! Discussion as notes, and proposals as notes — the vocabulary, and the one definition of the
//! hidden note-classes ("is this a message", "is this a proposal").
//!
//! A message is a `Kind::Note` carrying [`THREAD_OF`]: the discussion it belongs to. A proposal
//! is a `Kind::Note` carrying [`PROPOSES`]: a git branch it proposes, with its own discussion
//! hanging off it. No new kind, no new directory, no new store (`fm-model/src/lib.rs`: *resist
//! adding kinds — add properties instead*; `MASTERPLAN.md:110` names note-per-message as the
//! **safe** side of the granularity fork it most fears).
//!
//! **This module exists so the exclusion has exactly one definition.** `decisions.md` records
//! why, from the assets decision that came before it: *"Filtering in each renderer was rejected
//! — it must be repeated per view and silently forgotten by the next one."* That prediction
//! then came true for this very feature: the founding ruling enumerated three surfaces
//! (`board`/`agenda`/`recent`), and by the time it was built there were five — `.view` presets
//! and `activity` had shipped in between, and `activity` was duly forgotten.

use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate};

/// A *proposal* note's durable frontmatter: `proposes: branch:<name>` — the git branch this
/// note proposes merging, with the discussion hanging off it via [`THREAD_OF`]. A proposal is a
/// second hidden note-class: like a message, it is a real `Kind::Note` you do not *plan*, so the
/// planning views exclude it (see [`notes_base`]) and the Collaboration surface lists it.
///
/// The value is a [`fm_model::branch_ref`], not a `note:` reference — a branch is not a note.
/// It is a **tolerant, historical pointer**: the branch may be merged, renamed or deleted while
/// the note lives forever, so a reader treats a branch that no longer resolves as a *warning,
/// never an error*, and never derives live proposal state from the note (that comes from git).
pub const PROPOSES: &str = "proposes";

/// The note a proposal targets: `targets: note:<ULID>` on the proposal note. It ties a PR back to the
/// note whose discussion originated it, so a second `/research` or `/propose` on that note **refines
/// the same proposal** (one living PR per note) instead of piling up a new one — and so the note's
/// own discussion view can surface its current proposal. Descriptive only; the branch is the truth.
pub const TARGETS: &str = "targets";

/// A proposal that was **rejected**: `declined: true`. Reject deletes the branch (the change is
/// dropped, `main` untouched) but KEEPS the proposal note — a proposal is immortal, so a declined PR
/// stays as a record, exactly as a merged one does. It only disambiguates *declined* from *merged*
/// (both have a gone branch, which git alone cannot tell apart); the live open/closed truth still comes
/// from git (`branch_open`), never from this marker.
///
/// **A bare `true` bool is deliberate here** — unlike the note-visible `discussion:`/`thread_of:` keys
/// above, which must be parse-guarded *references* because a `board` group-by could write a column name
/// into them and hide a note. Proposal notes are excluded from **every** board ([`proposals_base`] /
/// [`notes_base`]), so no board-drag can reach this key; a bool is the honest type, and it round-trips a
/// hand-edited `declined: true` where a bool-shaped *string* would not. Do NOT copy this bool shape onto
/// a board-visible note class.
pub const DECLINED: &str = "declined";

/// The discussion a message belongs to: `thread_of: note:<ULID>`, always pointing at the
/// **root note**, never at another message.
///
/// The name is not `thread`. `board` groups by *any* property and writes the column name back
/// on drop, so an unnamespaced English word is a key a user can land on by accident — and a
/// property whose presence hides a note is then one drag away from silently erasing it from
/// every view. `thread_of` reads as a relation rather than a bucket, and the value must parse
/// as a reference before anything is hidden (see [`notes_base`]).
pub const THREAD_OF: &str = "thread_of";

/// The message (or the root) this one answers: `reply_to: note:<ULID>`.
///
/// Always set, including on the first reply, so depth derivation has no special case.
/// Unvalidated by construction — it is frontmatter a human can hand-edit — so every reader
/// must survive a dangling or circular value rather than trusting it.
pub const REPLY_TO: &str = "reply_to";

/// Is this object a discussion message?
///
/// Exists **separately from [`notes_base`]** because `activity` has nothing to hang a
/// `Predicate` on: it is a `git log` read-model (`decisions.md`, *"collaboration is git,
/// exposed"*), so it walks touches and resolves them through `Store::get`. A filter cannot
/// reach it, and that is exactly why the first attempt at this feature let messages flood the
/// recent-edits feed. One definition, two shapes of consumer.
pub fn is_message(o: &Object) -> bool {
    matches!(o.get(THREAD_OF), fm_model::PropertyValue::Text(s)
        if fm_model::parse_note_ref(&s).is_some())
}

/// Is this note a **discussion root** — a first-class discussion, created from `+ New discussion`?
///
/// A discussion is a note whose [`THREAD_OF`] points at **itself** (`thread_of: note:<own-id>`).
/// That self-anchor is the whole trick:
/// - it is a *reference* (parse-guarded), so unlike a `discussion: true` bool it cannot be forged
///   or erased by a board drag writing a column name into the key;
/// - it makes the discussion an [`is_message`] note automatically, so it is excluded from every
///   planning view (`notes_base`), from `activity`, and from the mock's `isNote` **for free** —
///   a discussion is simply the root of its own thread, needing no new hidden note-class;
/// - `thread_of` therefore already encodes the anchor: a standalone discussion points at itself, a
///   comment-on-a-note points at that note — one mechanism, not two.
pub fn is_discussion_root(o: &Object) -> bool {
    matches!(o.get(THREAD_OF), fm_model::PropertyValue::Text(s)
        if fm_model::parse_note_ref(&s) == Some(o.id))
}

/// Is this object a proposal note? The proposal twin of [`is_message`], and it exists for the
/// same reason: `activity` is a `git log` read-model with no `Predicate` to hang the exclusion
/// on, so a proposal commit would otherwise flood the recent-edits feed, the `EditedBy` labels
/// and the contributor filter — exactly the surface a filter-shaped fix silently misses.
///
/// True only when `proposes` *parses* as `branch:<name>`, so a stray property value never marks
/// an ordinary note as a proposal.
pub fn is_proposal(o: &Object) -> bool {
    matches!(o.get(PROPOSES), fm_model::PropertyValue::Text(s)
        if fm_model::parse_branch_ref(&s).is_some())
}

/// The base filter for every "things you plan" view: notes, and not messages.
///
/// **The only place `Predicate::Kind(vec![Kind::Note])` is written**, enforced by a grep in
/// `ci/checks.sh` — so a future command cannot quietly reintroduce the un-excluded form, and a
/// `.view` cannot widen it (a `.view`'s conjuncts are `Vec::extend`ed onto this base, so it can
/// only ever narrow).
///
/// Three exclusions, one reason: none of an asset, a message, or a proposal is something you
/// *plan*.
/// - **Assets** — a blob a note references. Findable via `search` or via its note.
/// - **Messages** — a 40-message thread is 40 notes. Left in, they fill the board's `(none)`
///   column, bury real notes in the timeline, and flood `recent()` — which is what the editor's
///   `/` note-picker lists, so linking one note to another would stop working.
/// - **Proposals** — a note that proposes a branch (`proposes: branch:<name>`). It belongs to
///   the Collaboration surface, not the board; left in, it is the same `(none)`-column pollution.
///
/// Each exclusion is `Not(<ref>)`, not `Prop{Exists}`: the value must *parse as a reference*
/// before a note is hidden, or a board drop writing a column name into `thread_of`/`proposes`
/// would silently erase the note from every view.
///
/// `search` and `gallery` deliberately do **not** use this: discussion (or a proposal) that is
/// not findable in five years defeats the reason for keeping it in the vault at all.
pub fn notes_base() -> Filter {
    Filter::new()
        .and(Predicate::Kind(vec![Kind::Note]))
        .and(Predicate::Not(Box::new(Predicate::NoteRef {
            key: THREAD_OF.into(),
            id: None,
        })))
        .and(Predicate::Not(Box::new(Predicate::BranchRef {
            key: PROPOSES.into(),
        })))
}

/// The base filter for the Collaboration surface: notes that **are** proposals — the exact
/// complement of the proposal exclusion in [`notes_base`].
///
/// It lives here, beside `notes_base`, for two reasons: the `Predicate::Kind(vec![Kind::Note])`
/// literal is CI-grepped to this file only (so it cannot be hand-rolled in a command), and "what
/// counts as a proposal" then has a single definition the exclusion and the surface both derive
/// from — they can never drift into disagreement about which notes are proposals.
pub fn proposals_base() -> Filter {
    Filter::new()
        .and(Predicate::Kind(vec![Kind::Note]))
        .and(Predicate::BranchRef { key: PROPOSES.into() })
}
