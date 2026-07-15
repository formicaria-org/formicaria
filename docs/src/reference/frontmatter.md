# Frontmatter schema

Each note is Markdown with a YAML frontmatter header. Well-known fields are
below; **any additional key you add is preserved** and immediately usable for
grouping (e.g. `project: alpha`).

| Field     | Type                | Meaning                                             |
|-----------|---------------------|-----------------------------------------------------|
| `schema`  | int                 | schema version (currently `1`).                     |
| `id`      | ULID                | the note's identity (also the filename).            |
| `type`    | enum                | `note` \| `task` \| `meeting` \| `asset`.           |
| `title`   | string (optional)   | display title; for assets, the original filename.   |
| `status`  | string (optional)   | any workflow label (e.g. `todo`, `doing`, `done`).  |
| `due`     | date (optional)     | `YYYY-MM-DD`; drives the agenda and urgency.         |
| `hard`    | bool                | a hard deadline (marked ◆).                          |
| `tags`    | list of strings     | tags.                                               |
| `assets`  | list of strings     | blob references (`sha256:<hex>`) this note points at.|
| `code`    | list of strings     | code references (repo + commit).                    |
| `created` | datetime (RFC 3339) | set once, on creation.                              |
| `updated` | datetime (RFC 3339) | bumped on every edit.                               |
| *(custom)*| any                 | preserved verbatim; groupable with no code change.  |

Notes:

- `id`, `created`, `updated`, and `schema` are **not editable** through
  `set_property`.
- Urgency (overdue / soon / week / later) is **derived** from `due` against the
  local date — there is no stored priority field.
- The body is literal Markdown; it round-trips byte-for-byte through edits.
- `assets`/`code` are read-only from the UI (set by ingest / tooling).

Example:

```markdown
---
schema: 1
id: 01J8Z9X0K2A3B4C5D6E7F8G9H0
type: meeting
status: doing
due: 2026-07-20
hard: true
tags:
  - meta-rl
assets:
  - sha256:39908450001673e971ff2e6ecd0e43b0ae534d74dfd91759cff9a4bdb06a5f2b
created: 2026-07-15T10:00:00Z
updated: 2026-07-15T12:30:00Z
project: alpha
---

Meeting notes go here.
```
