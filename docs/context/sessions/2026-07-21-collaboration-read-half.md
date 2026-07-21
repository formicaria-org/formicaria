# 2026-07-21 — Collaboration read-half: proposals as notes

Built the first increment of the collaboration UX the strategic conclusion has always pointed at
(*"files you own + git underneath + a UI a non-git person can use"*). It shipped behind a plan
that was refined by **two rounds of a 3-agent review** (maintainability / longevity /
simplicity-efficiency); the plan file is `~/.claude/plans/staged-drifting-shamir.md`, with a
prior-art brainstorm (git-bug, Radicle COBs, Fossil, Obsidian, Automerge, Pijul) recorded there.

## The model (locked with the owner)

**A proposal = a git branch + an ordinary note.** The note lives on `main`, carries
`proposes: branch:<name>`, and holds its discussion via the existing `thread_of`. It is a
**second hidden note-class** (like a message): a real `Kind::Note` you do not *plan*, so the
planning views exclude it and a new **Collaboration** view gathers it. No new kind — "resist
adding kinds, add properties" (`fm-model/src/lib.rs:20`).

Agent vs human proposal = **provenance only** (git identity); **accept/merge is human-only** and
is the *write half*, deliberately **not** in this increment. This is the safe-containment pattern
for AI contributions the 2026 industry converged on (Dependabot→Claude/Codex draft PRs, hard human
gate) — Ruling 18, exactly.

## What shipped (all read-only; no git writes)

- `fm_model::parse_branch_ref` / `branch_ref` — the `branch:<name>` twin of `parse_note_ref`, a
  *shape test* so a stray value (a board drop) never reads as a pointer.
- `fm_query::Predicate::BranchRef { key }` — the query-engine sibling of `NoteRef`; same reason it
  is not `Prop{Exists}` (that would hide a note the instant any text landed in `proposes`).
- `thread::PROPOSES`, `is_proposal`, `proposals_base()`, and the proposal exclusion added to
  `notes_base()`. `commands::proposals()` returns `Vec<ObjectMeta>` (a store query like `recent`,
  **not** a per-vault git read); dispatch arm `"proposals"`.
- **The bug a filter-shaped fix would miss:** `activity()` is a `git log` read-model with no
  `Filter`, so the exclusion is applied by hand — `is_proposal` added beside `is_message`, or a
  pile of proposals would flood the recent-edits feed. Tested (`tests/proposals.rs`).
- UI: a **built-in-pane registry** (`BUILTIN_PANES` in `panes.ts`) that the ⌘K palette
  (`App.svelte`), the pane view-picker (`Pane.svelte`), and the bottom-bar icons (`ViewBar.svelte`)
  all derive from — killing the three-place silent literal spread (and retiring `activity`'s). The
  `collaboration` pane reuses the **Timeline renderer** (no new `.svelte`), fed by `proposals()`.
  A `merge` icon added to `Icon.svelte`. Mock mirrors the server (`isProposal`, parse-guard).
- Docs: new `user/collaboration.md` (#proposals), `views.md` third hidden class (property-filtered
  *by choice*), `commands.md` (`proposals`), and **`frontmatter.md` "Proposal semantics"** — the
  four durable one-way-door rules locked *now* because git history is forever (Ruling 4): `proposes`
  is a tolerant historical pointer (missing branch = warning, live state from git); `status` =
  author *readiness* only (lifecycle from git); `base` = immutable SHA; provenance = git identity,
  never a frontmatter `author` key.

`pixi run ci` green (Rust + 240 UI tests + checks + docs). **Uncommitted**, along with the earlier
discussion + stale work from this session.

## Deferred (named in the plan, not silently dropped)

Write half: `create-proposal` (branch+note), `accept`→merge (human-only dispatch seam), propose-
modifications, the read-only **diff** (branch vs `base`) + branch/status badges. Two adjacent
slices the owner scoped: an **explicit push-permission message** (classify a protected-branch
rejection *local to `push_squashed`*, offer "propose instead"; generic non-fast-forward → *pull*,
not propose) and **leaning the note/whiteboard action menu** (one `…` overflow reusing
`capture-menu`, not a new component). Ruling 17 (board image→blob) still gates *shared* boards.
