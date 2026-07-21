# 2026-07-21 — Declutter the note header

The note pane header crammed ~10 controls into one row. Refined by a **4-agent review** (my three
principles + a GUI-best-practices agent that surveyed Obsidian, Notion, Bear, Craft, Apple Notes,
Logseq, Linear, Todoist + Material 3 / Apple HIG).

## Two corrections the review made to the literal ask

The owner proposed "keep only vault + users, fold the rest (incl. Delete) into a single `＋`". The
review corrected two points, both evidenced:
- **`⋯`, not `＋`.** `＋` already means *add media* in this header and *new note/discussion* in the
  create menu — a collision. `⋯` is the universal "more actions" trigger everywhere.
- **Keep Edit/Details and StatusChip visible.** Edit is the primary action (a board's canvas eats
  double-click, so the button is the only way in there) and status-rotate is "the most frequent
  edit a note gets" (the code says so). Material 3: keep 1–3 primaries visible — here
  StatusChip + Edit + `⋯`.

## What shipped

- Only **Delete + Copy to…** moved into a new `⋯` overflow in `NotePanel.svelte`, reusing the
  existing `.capture-menu` dropdown **verbatim** — a sibling `<ul>` + a `menuOpen` state mirroring
  `captureOpen`. No new component, no new CSS system. Delete is last + red (reuses `.danger`) and
  keeps its confirm strip (formicaria delete is irreversible — no Trash).
- `canCopy` derived once (`otherVaults.length && !isDiscussion`) so the menu item's guard and the
  copy popover's guard can't disagree — and a discussion (whose body is a thread) shows no Copy-to.
- The transients (delete-confirm, copy popover, copy-undo toast) were **untouched** — already
  decoupled sibling blocks; menu items just set their flags and close the menu.
- **a11y note:** the menu is a plain button group (no `role=menu`/`menuitem`) — matching
  `capture-menu`, and the more honest choice without roving arrow-key focus (a `role=menu` the
  keyboard can't drive is worse). Trigger carries `aria-haspopup`/`aria-expanded`/`aria-label`;
  Escape closes.
- **Durable rule written into `docs/src/dev/design-system.md`:** *header shows identity (vault, who
  edited) + in-place content primaries (status, Edit); `⋯` overflow holds lifecycle/destructive
  (Copy, Delete), destructive last + red; trigger is `⋯` never `＋`; a new kind-specific action
  declares its kinds — never a per-kind list.* An operational test so the overflow can't re-clutter.
- Updated the 4 `App.features.test.ts` copy/delete tests to open the `⋯` first.

`pixi run ci` green (Rust + 243 UI tests + checks + docs). **Uncommitted**, with the session's other
collaboration work.

## Deferred (no one-way door)

Per-kind action array (inline `<li>`s are the floor for two actions; invert to *actions declare
kinds* when a kind-specific one lands — Propose-a-change, board image→blob, pins). No extracted
`NoteActions.svelte`. No bottom-anchored sheet on phones yet (documented as the follow-up if the
overflow ever grows beyond rare actions — it lives top-anchored *because* its contents are rare).
