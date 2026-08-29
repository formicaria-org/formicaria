# Frontmatter schema

Each note is Markdown with a YAML frontmatter header. Well-known fields are
below; **any additional key you add is preserved** and immediately usable for
grouping (e.g. `project: alpha`) — with the reserved exceptions noted in the
table (`thread_of` / `reply_to`, and `proposes` / `base`), which mark a note as a
[discussion message](../user/notes.md#discussion) or a
[proposal](../user/collaboration.md#proposals) and are managed for you rather than
set by hand.

| Field     | Type                | Meaning                                             |
|-----------|---------------------|-----------------------------------------------------|
| `schema`  | int                 | schema version (currently `1`).                     |
| `id`      | ULID                | the note's identity (also the filename).            |
| `type`    | enum                | `note` \| `asset`. Not user-settable — differentiate notes with **tags**. |
| `title`   | string (optional)   | display title; for assets, the original filename.   |
| `status`  | string (optional)   | any workflow label (e.g. `todo`, `doing`, `done`).  |
| `start`   | stamp (optional)    | when the work/meeting begins; the left end of the calendar bar. |
| `due`     | stamp (optional)    | the deadline/end; drives the agenda and urgency.    |
| `hard`    | bool                | a hard deadline (marked ◆).                          |
| `tags`    | list of strings     | tags.                                               |
| `assets`  | list of strings     | blob references (`sha256:<hex>`) this note points at.|
| `code`    | list of strings     | code references (repo + commit).                    |
| `created` | datetime (RFC 3339) | set once, on creation.                              |
| `updated` | datetime (RFC 3339) | bumped on every edit.                               |
| `thread_of`| `note:<ULID>`      | **reserved.** Marks this note as a message in another note's [discussion](../user/notes.md#discussion). A note carrying it is hidden from Board/Agenda/Timeline and is not offered as a link target — so it is **not** groupable like a custom key. Set by *Reply*, not by hand or `set_property`. |
| `reply_to` | `note:<ULID>`      | **reserved.** The message (or note) this one answers, for indentation only. Same handling as `thread_of`. |
| `proposes` | `branch:<name>`    | **reserved.** Marks this note as a [proposal](../user/collaboration.md#proposals) of a git branch. Like a message, a note carrying it is hidden from Board/Agenda/Timeline and gathered by the Collaboration view instead. See *Proposal semantics* below. |
| `base`    | commit SHA          | **reserved.** The commit a proposal's branch is measured against (an immutable SHA, not a branch name). Used by the diff, not for grouping. |
| *(custom)*| any                 | preserved verbatim; groupable with no code change.  |

## Proposal semantics

A [proposal](../user/collaboration.md#proposals) is an ordinary note that names a git branch it
proposes merging. Four rules keep it honest, because a note lives in git history **forever** while
a branch does not:

- **`proposes` is a tolerant, historical pointer — not live state.** A branch may be merged,
  renamed, or deleted while the note remains. A proposal whose branch no longer resolves is shown
  with a *warning*, never treated as an error or hidden. Whether a proposal is open, merged, or
  abandoned is read **from git**, never from the note.
- **`status` on a proposal means author *readiness* only** — `draft` (don't review yet) or `ready`
  (please review). It is the same `status` field, reused. It never carries lifecycle
  (`merged`/`closed`/`rejected`): that is derived from git, because a stored lifecycle would be a
  lie the moment the branch moved.
- **`base` is an immutable commit SHA**, so the diff still resolves years later. A branch name
  would dangle.
- **Who proposed it is git's answer, not a field.** There is no `author`/`agent`/`proposed_by`
  key — a proposal by an AI agent differs from a human's only in the git commit identity, which
  the "edited by" label already shows. A proposal can never be *accepted* (merged) by an agent;
  that is a human action.

## Stamps: `start` and `due`

A **stamp** is a calendar day with an *optional* wall-clock time:

| Written as         | Means                                              |
|--------------------|----------------------------------------------------|
| `2026-07-20`       | all day — a deadline with no particular hour.      |
| `2026-07-20T14:30` | 14:30 on that day — a meeting, a talk, a call.     |

A stamp is **naive**: it carries no UTC offset, because 14:30 means 14:30 where
you are. (`created`/`updated` are different — they are *instants*, so they keep a
full RFC 3339 offset.)

Both spellings are accepted when you hand-edit a note; fm normalizes them to the
two forms above on its next write:

- `2026-07-20 14:30` (a space instead of `T`)
- `2026-07-20T14:30:00` (seconds — truncated; stamps are minute-granular)

Give a note a `start` *and* a `due` and it draws as a bar across those days in
the calendar. When both ends carry a time, the bar is labelled with its window
(`14:30–15:00`) — that is what a meeting looks like.

Notes:

- `id`, `created`, `updated`, and `schema` are **not editable** through
  `set_property`.
- Urgency (overdue / soon / week / later) is **derived** from `due` against the
  local date — there is no stored priority field. It is **day-granular**: a time
  changes how a note is *displayed*, never which urgency band it lands in.
- The body is literal Markdown; it round-trips byte-for-byte through edits.
- `assets`/`code` are lists of references. Ingest sets `assets`; both are editable from the note's
  details panel like any other field, comma-separated.

Example:

```markdown
---
schema: 1
id: 01J8Z9X0K2A3B4C5D6E7F8G9H0
type: note
status: doing
start: 2026-07-20T14:30
due: 2026-07-20T15:00
hard: true
tags:
  - meeting
  - meta-rl
assets:
  - sha256:39908450001673e971ff2e6ecd0e43b0ae534d74dfd91759cff9a4bdb06a5f2b
created: 2026-07-15T10:00:00Z
updated: 2026-07-15T12:30:00Z
project: alpha
---

Meeting notes go here.
```
