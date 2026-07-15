# Frontmatter schema

Each note is Markdown with a YAML frontmatter header. Well-known fields are
below; **any additional key you add is preserved** and immediately usable for
grouping (e.g. `project: alpha`).

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
| *(custom)*| any                 | preserved verbatim; groupable with no code change.  |

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
- `assets`/`code` are read-only from the UI (set by ingest / tooling).

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
