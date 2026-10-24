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

/// Why a proposal was **rejected**, in the reviewer's own words: `declined_why: <one line>`.
///
/// Reject writes no commit of its own — `main` is untouched by design — and the rejected text never
/// reaches `main` at all, so this note is the **only** place a rejection's reason can survive. It
/// rides on the proposal note that [`DECLINED`] already rewrites, so it costs no new file.
///
/// A `Text` value is safe here for the same reason [`DECLINED`]'s bare bool is: proposal notes are
/// excluded from **every** board ([`proposals_base`] / [`notes_base`]), so no board drop can write a
/// column name into this key. Do not copy the shape onto a board-visible note class.
///
/// Descriptive only, and never load-bearing: it is hand-editable frontmatter, so it is a record of
/// what someone said, never evidence that anyone approved anything.
pub const DECLINED_WHY: &str = "declined_why";

/// **When a proposal was accepted**: `accepted: <RFC3339, UTC>` (2026-10-24, *a proposal is open,
/// merged, declined or closed*). Stamped by `accept_proposal`, or at start from the accept commit's
/// own time for one accepted before this existed (`commands::settle_proposals`). Like [`DECLINED`], it
/// lives on the immortal proposal note, so every device and every collaborator reads the same state.
pub const ACCEPTED: &str = "accepted";

/// **When a proposal was rejected**: `declined_at: <RFC3339, UTC>`, beside [`DECLINED`] (2026-10-24).
/// Absent on one rejected before it existed.
pub const DECLINED_AT: &str = "declined_at";

/// **A proposal that ended with no record of how**: `closed: true` — its branch gone here and on
/// `origin`, no accept commit, never declined (2026-10-24). Stamped only by `settle_proposals`.
pub const CLOSED: &str = "closed";

/// How a proposal stands, from its note alone: `declined`, `merged`, `closed` or `open`.
pub fn proposal_state(o: &fm_model::Object) -> &'static str {
    let set = |k: &str| match o.get(k) {
        fm_model::PropertyValue::Bool(b) => b,
        fm_model::PropertyValue::Text(s) => !s.trim().is_empty() && s != "false",
        fm_model::PropertyValue::Null => false,
        _ => true,
    };
    if set(DECLINED) {
        "declined"
    } else if set(ACCEPTED) {
        "merged"
    } else if set(CLOSED) {
        "closed"
    } else {
        "open"
    }
}

/// **When a proposal was first actually shown to a person**: `shown: <RFC3339>`.
///
/// The cheapest high-value discriminator in the whole record, and the only handle on the bias that
/// otherwise makes silence unreadable. Without it, "the reviewer left this alone" and "nobody ever
/// looked at it" are the same absence — and they mean opposite things about the model's output.
/// Zed's edit-prediction telemetry carries the same flag for the same reason; Continue.dev, lacking
/// it, has to guess with a ten-second timer and calls the result a rejection.
///
/// A **timestamp rather than a bool**, because it costs the same and answers more: the gap between
/// this and the decision is how long someone actually spent on it.
///
/// Stamped **server-side**, once, and never overwritten — a second viewing is not a second first
/// viewing, and a browser's clock is not evidence. Like [`DECLINED`], it lives on the immortal
/// proposal note: it is a fact that arrives *after* the proposal's commit is written, and a commit
/// message is immutable.
pub const SHOWN: &str = "shown";

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

/// The property that marks a **logged entry** — a meal, a set, a weighing — and says which kind.
///
/// `decisions.md#data`, 2026-10-11: an entry is a note with `log: <kind>`, a day in `start`, and
/// whole numbers whose unit is in the key. There are thousands of them a year, so they are a hidden
/// note-class like messages and proposals. `apply_property` refuses the key, so nothing but the
/// command that makes an entry can set it.
pub const LOG: &str = "log";

/// Is this object a logged entry? The entry twin of [`is_message`], for the surfaces a filter
/// cannot reach.
pub fn is_log(o: &Object) -> bool {
    matches!(o.get(LOG), fm_model::PropertyValue::Text(s)
        if fm_model::parse_log_kind(&s).is_some())
}

/// The base filter for every "things you plan" view: notes, and not messages.
///
/// **The only place `Predicate::Kind(vec![Kind::Note])` is written**, enforced by a grep in
/// `ci/checks.sh` — so a future command cannot quietly reintroduce the un-excluded form, and a
/// `.view` cannot widen it (a `.view`'s conjuncts are `Vec::extend`ed onto this base, so it can
/// only ever narrow).
///
/// Four exclusions, one reason: none of an asset, a message, a proposal or a logged entry is
/// something you *plan*.
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
        .and(Predicate::Not(Box::new(Predicate::NoteRef { key: THREAD_OF.into(), id: None })))
        .and(Predicate::Not(Box::new(Predicate::BranchRef { key: PROPOSES.into() })))
        // **Logged entries** — five to ten a day. Left in, a year of meals is three thousand
        // cards in the board's `(none)` column.
        .and(not_logged())
}

/// "…and not a logged entry." Written once, because `search` needs it without the rest of
/// [`notes_base`]: it must still find messages and proposals, but three hundred notes titled
/// "Coffee" would bury everything it is for.
pub fn not_logged() -> Predicate {
    Predicate::Not(Box::new(Predicate::LogKind { key: LOG.into() }))
}

/// The tag that marks a note as a template — a starting point to make new notes from.
/// The domain every assistant signs its writing with (`fm_agent_run::Agent::author`), pinned by a
/// test on each side since the two crates share no dependency.
pub const AGENT_DOMAIN: &str = "@fm-agents.local";

/// **The name a writer is shown by** (2026-10-18). Identity stays the email (`decisions.md#ui`,
/// 2026-07-31, *a contributor is an email*); only the label changes. The assistant is **Myrme**: its
/// writing from before the name — signed `<model>@fm-agents.local`, and named after the model — reads
/// as Myrme, and `Myrme (laptop)` keeps the device the person gave it. History is not rewritten.
pub fn shown_author(name: String, email: &str) -> String {
    if email.ends_with(AGENT_DOMAIN) && !name.starts_with(ASSISTANT) {
        ASSISTANT.to_string()
    } else {
        name
    }
}

/// The assistant's one name, as `fm_agent::convo::MYRME` spells it (pinned by a test there).
pub const ASSISTANT: &str = "Myrme";

pub const TEMPLATE_TAG: &str = "template";

/// Whether a note is a template. **A template is inert** (`decisions.md#ui`, 2026-10-12): it
/// carries the structure the notes made from it will have — a `status`, a `program`, a `recipe` —
/// so that it never acts as one of them is said here, once, and asked by everything that reads
/// such a key.
pub fn is_template(o: &fm_model::Object) -> bool {
    o.tags.iter().any(|t| t == TEMPLATE_TAG)
}

/// What the board and the agenda draw: the notes you plan with, and not the templates you make
/// them from. A template with a `status` would otherwise sit among the tasks for ever.
pub fn tasks_base() -> Filter {
    notes_base().and(Predicate::Not(Box::new(Predicate::TagsAll(vec![TEMPLATE_TAG.into()]))))
}

/// One log's entries **and** the notes holding its targets, in one filter — so a day's totals and
/// the targets they are measured against cost one pass over the vault, not two. The caller tells
/// the two apart (`is_log`); a program is an ordinary note carrying `program: <kind>`.
pub fn log_with_programs_base(kind: &str, program_key: &str) -> Filter {
    let is = |key: &str| Predicate::Prop {
        key: key.into(),
        op: fm_query::Op::Eq,
        value: fm_model::PropertyValue::Text(kind.to_string()),
    };
    Filter::new()
        .and(Predicate::Kind(vec![Kind::Note]))
        .and(Predicate::Any(vec![is(LOG), is(program_key)]))
}

/// One log's entries **and** the notes that carry `key` — a recipe's own note, which is an ordinary
/// visible note and names its ingredients by a word under that key (`decisions.md#ui`,
/// 2026-10-12). One pass, as for programs; the caller tells the two apart.
pub fn log_with_keyed_base(kind: &str, key: &str) -> Filter {
    Filter::new().and(Predicate::Kind(vec![Kind::Note])).and(Predicate::Any(vec![
        Predicate::Prop {
            key: LOG.into(),
            op: fm_query::Op::Eq,
            value: fm_model::PropertyValue::Text(kind.to_string()),
        },
        Predicate::Prop {
            key: key.into(),
            op: fm_query::Op::Exists,
            value: fm_model::PropertyValue::Null,
        },
    ]))
}

/// The base filter for one log: the entries of that kind, and nothing else. The complement of the
/// entry exclusion in [`notes_base`], here for the same two reasons as [`proposals_base`].
pub fn log_base(kind: &str) -> Filter {
    Filter::new().and(Predicate::Kind(vec![Kind::Note])).and(Predicate::Prop {
        key: LOG.into(),
        op: fm_query::Op::Eq,
        value: fm_model::PropertyValue::Text(kind.to_string()),
    })
}

/// What a plan's progress is measured from, in one pass: the entries of some logs **and** the
/// notes carrying any of some keys — a program note, a profile (`plan.rs`;
/// `decisions.md#data`, 2026-10-12). The caller tells them apart.
pub fn plan_base(logs: &[&str], keys: &[&str]) -> Filter {
    let mut any: Vec<Predicate> = logs
        .iter()
        .map(|kind| Predicate::Prop {
            key: LOG.into(),
            op: fm_query::Op::Eq,
            value: fm_model::PropertyValue::Text((*kind).to_string()),
        })
        .collect();
    any.extend(keys.iter().map(|key| Predicate::Prop {
        key: (*key).into(),
        op: fm_query::Op::Exists,
        value: fm_model::PropertyValue::Null,
    }));
    Filter::new().and(Predicate::Kind(vec![Kind::Note])).and(Predicate::Any(any))
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
