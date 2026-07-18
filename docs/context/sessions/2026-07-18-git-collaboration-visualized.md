# 2026-07-18 — Surface git as the collaboration layer (authorship, activity, contributors, awareness)

The collaboration *machinery* already shipped (per-vault git, the `.md` merge driver, push/pull,
a per-vault signed identity). What was missing was the **read** side: git knows who changed what
and when, but nothing showed it. This session exposes it — **don't build git features, expose
them** — with one read-only command behind everything.

## The whole read-model is one `git log`
`crates/fm-core/src/git.rs::activity(vault, since)` runs
`git log --no-merges --since=… --pretty=format:\x01%an\x1f%ae\x1f%aI --name-only` and, walking
newest-first, records the **first** time each `notes/<ULID>.md` path appears = its **last edit**.
Returns `Vec<Touch { id, author, email, time }>`. A note file's stem *is* its id
(`FileStore::path_for`), so no mapping is invented; `--no-merges` because a merge commit's author
is the merger, not the writer; `\x1f`/`\x01` separators survive names with spaces. Read-only,
nothing stored — git stays the source of truth. `commands::activity` resolves each touch to its
current `title`/`type`/`vault` via the store (dropping notes gone from the index); the
`"activity"` server arm aggregates across vaults, newest-first. Tests in
`crates/fm-core/tests/activity.rs` (two identities, last-editor-wins, empty without a repo).

## Four UI surfaces, all reusing existing patterns
- **`EditedBy.svelte`** — "● name · 5m ago" on every card (Card/Timeline/Agenda/Search) and the
  open note. Person-coloured by the shared `hashHue` (generalised from `vaultColor.vaultHue`), so
  a person's colour matches their filter chip and activity rows. Reaches renderers without
  prop-drilling via a **runes-in-module store** `ui/src/lib/activity.svelte.ts` (`setActivity` /
  `lastEditFor` / `activityEvents` / `contributors`); App fills it in `refresh()`, in parallel and
  non-blocking. Placeholder committer (`formicaria@localhost`) reads as "you".
- **Activity pane** (`PaneKind: 'activity'`, `renderers/Activity.svelte`) — git's log as a
  first-class view in the flexible workspace: recent edits grouped by day (reusing `dayHeading`),
  each row EditedBy + title + VaultBadge, click opens. `feedKey('activity')` is null — it reads
  the module, no separate fetch. A `+ Activity` top-bar chip and a palette command open it.
- **Contributor filter** — chips beside the vault chips (`hiddenAuthors`, persisted like
  `hiddenVaults`), each carrying the person's colour dot. `App.shown` (already threaded to every
  pane/renderer) gained an author check, so one click hides a person's notes **everywhere**,
  including their activity rows. `shown`'s param is now structural `{id, vault}` so both
  `ObjectMeta` and `EditEvent` satisfy it.
- **Automatic awareness** — a top-bar "⟳ {vault}: get changes" chip. A **network** poll
  (`backup_status`/`remote_moved`, ~45 s, visibility-gated — *not* the 3 s heartbeat) fills
  `movedVaults`; the chip runs the existing `pull` then refreshes. Wires the previously-deferred
  automatic "someone pushed" nudge using only existing commands. Production-only (the mock has no
  remote).

## Reused, not rebuilt
`git(vault)` helper, `remote_moved`/`pull`/`backup_status`, `FileStore::path_for`, the
pane/feed/`shown` system, the vault-filter chip pattern, `VaultBadge`/`hashHue`, `dayHeading`. New
files are small: `git::activity`, `commands::activity`+`EditEvent`, `EditedBy.svelte`,
`activity.svelte.ts`, `Activity.svelte`, plus a `relativeTime` helper in `stamp.ts`.

## Verified
`pixi run ci` green: Rust `activity` tests, `relativeTime` unit test, and UI tests (EditedBy on
cards; the Activity pane lists edits; hiding a contributor drops their rows). 177 UI tests; the
renderer no-status-literal grep still passes. Backend changed → the server must be **restarted**
(not just rebuilt) to pick up `activity`.

## Deferred (unchanged from the plan)
- **Creator attribution** ("created by") and a finer per-commit event log — the last-editor map is
  the MVP.
- **Anchored comments / discussion threads** — needs the deferred backlinks index; the activity
  stream is the git-native stand-in.
- **Live presence** ("editing now") — impossible without the descoped peer; git knows only
  *pushed* state.
