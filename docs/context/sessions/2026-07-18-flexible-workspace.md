# 2026-07-18 — A flexible pane workspace, and the nav moved off the rail

## What changed

The single-view app became a **flexible grid of panes**, and the tall left sidebar
became a **horizontal top bar**. This is the headline UI ask, and it was asked for
twice — the first delivery proposed hardcoded preset layouts (1 / 2 / 3 / 2×2) and
was rejected: *"do not hard code a view … a flexible interface for the user to open
views and rearrange them where it wants in a flexible grid."* Plus a specific
complaint the top bar answers: the sidebar was tall and wasted vertical space.

### The model — `ui/src/lib/panes.ts` (new, pure + unit-tested)
- `Pane = { id, kind, groupBy, agendaMode, query, noteId, viewName, colSpan, rowSpan }`;
  `kind ∈ board | agenda | timeline | search | note | view`.
- `Workspace = { cols, panes }` — a column count and an **ordered flat list** that
  flows into a `repeat(cols, 1fr)` CSS grid. No split tree: the one structure with
  no natural stopping point is the one we don't build. Guard rails: `MAX_PANES = 8`
  (a soft cap with a message, not a silent truncation) and no pane-inside-a-pane.
- `paneId()` is a monotonic counter — **no `Math.random`/`Date`** (the same
  determinism rule the workflow/runtime holds; keeps `{#each}` keys stable and tests
  hermetic).
- **Feed-key dedup** is the efficiency seam: `feedKey(p)` maps a pane to its data
  source (`board:<groupBy>` / `agenda` / `timeline` / `search:<q>` / `view:<name>`;
  `note` → `null`, NotePanel self-fetches). `distinctFeeds()` is what App actually
  fetches, so **N panes over M feeds cost M requests, not N**, and the 3 s heartbeat
  refreshes the distinct set. Two boards grouped differently are two feeds; two
  boards grouped the same share one.

### The pane — `ui/src/lib/Pane.svelte` (new)
A thin frame over the existing **pure** renderers (Board/Agenda/Calendar/Timeline/
Search), which are already `height:100%` with internal scroll — that purity is the
whole reason the grid is cheap. Header = a drag grip, a kind `<select>` (built-ins +
saved `.view`s), per-kind inline controls (board group-by, agenda M/W/L, search box),
and a close ×. `onmove` now takes the pane's **own** `groupBy` — a drag in one board
writes its own property, never a sibling's (a real bug the moment two boards exist).

### Rearrange + resize (S3)
- **Drag to reorder**: the header grip is a `draggable` carrying `{ paneIndex }`; the
  whole pane is a `dropTargetForElements`. Same `@atlaskit/pragmatic-drag-and-drop`
  the board cards already use. A pane drag carries `paneIndex`; card/column drags
  carry `id`/`columnValue`, so the branches never cross — a card dropped on a pane is
  ignored, a pane dropped on a board column is ignored there.
- **Corner resize**: a bottom-right grip; native pointer events with pointer capture.
  Self-measured — one track = `paneRect.width / colSpan` — so no access to the grid
  container is needed. Clamped to the column count (`clampSpan`) and ≤4 rows.
- **Persistence**: the whole `Workspace` round-trips through
  `localStorage['fm-workspace']` — a **view preference**, the same settled class as
  theme and column order (never in a vault). A resize does **not** refetch (span moves
  no feed); a regroup/search-edit does (the feed key changed).

### Shell — `ui/src/App.svelte`
`.app` grid went from `15rem 1fr` (+ trail col) to `grid-rows: auto 1fr`: a top bar
spanning row 1 over the workspace in row 2. The note trail stays a **peer column**
(row 2, col 2) — reading a note is still not a mode. Removed the entire dead sidebar
CSS (~230 lines); two of those selectors (`.wordmark`, `.searchfield`) were *live
duplicates* colliding with the top-bar rules via the cascade, so the cleanup also
fixed the top-bar search field's styling.

## Verified
- `pixi run ci` green: 164 UI tests (incl. `panes.test.ts` pure helpers), the Rust
  suites, `cargo deny`, the three architectural greps, and the mdBook build. No unused
  CSS warnings remain.
- Release rebuilt (`pixi run build`); the local server on `127.0.0.1:8765` serves the
  new bundle (HTTP 200, `index-EUjN4Lhg.js`).

## NOT verified — needs the owner's eyes
The Claude-in-Chrome extension is **not connected**, so the layout was verified
*structurally*, not *seen*. Whether the top bar wraps gracefully, whether drag/resize
feel right, and whether the grid reads the way the owner pictured are open until they
look. This is the third UI turn on this ask; confirm the grid before S4.

### S4 — note / whiteboard panes (the trail retired)
A note is a **pane** now (`kind: 'note'`), not a separate side-trail. `Pane` renders
`NotePanel` (lazy-imported, so the read pipeline still code-splits) for that kind, its
header shrinking to just the drag grip — NotePanel already carries its own chrome
(title, status, edit, delete, close), so a second header would be redundant. A
board-note renders its Excalidraw canvas exactly as before, so **"whiteboard in a
pane" is free**. NotePanel's old full-screen toggle is repurposed to **maximize** the
pane (span → full width; toggle back to 1×1).

The load-bearing invariant carried over from the trail: **one note, one pane**.
`openNoteInPane(id)` dedups by `noteId` — a card click, a followed `note:` chip, or a
re-follow *focuses* an existing note pane rather than opening a second editor over the
same file (two editors / two Excalidraw roots would race `updateBody` and lose writes).
This is why the existing `follows-a-reference` / `re-following-truncates` tests still
pass unchanged: pane dedup gives the same panel-count outcomes the trail's truncation
did. Retired with the trail: `openIds`, `pushNote`, `closeFrom`, the `fm-note-wide`
preference, and the `--trail`/`--ws` grid columns — `.app` is now a plain
`rows: auto 1fr`. `editingId` (transient, unpersisted) is the one note that opens
straight in the editor (the "New note" flow); a reload reopens note panes in read mode.

## Result: the plan is complete (S1–S4)
All four slices shipped; `pixi run ci` green (164 UI tests, Rust, deny, seam greps,
docs); release rebuilt (`index-BOoSEPYL.js`), served on `127.0.0.1:8765`. Still
**unseen** — the browser extension is not connected, so drag/resize/note-pane *feel*
is the owner's to confirm.

## Still open
- Visual confirmation (owner's eyes) — the one thing CI cannot cover.
- Standing order still in force: **no remote CI** until the owner lifts it (all four
  GitHub workflows are `workflow_dispatch`-only; memory `no-remote-ci-billing`).
