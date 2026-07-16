# 2026-07-16 — Note references (`note:` links) + the sliding-pane trail

**Outcome:** notes can reference notes. `/` in the editor now lists notes as well
as assets and inserts `[Title](note:<ulid>)`; the read view resolves that to a
chip showing the target's live title + status; clicking it opens the target as a
**pane to the right**, keeping the note you came from on screen. Verified as far
as headless allows: `pixi run ci` exit 0, `svelte-check` 0 errors, 107 UI tests
(19 new), including a real App-driven flow that clicks a chip and asserts two
panes. The **visual layout of the trail** (horizontal scroll, snap, pane widths)
is unverified — it needs a real browser; the owner confirms visually.

## Why this shape

`roadmap.md:116` had asked for `[[wikilinks]]`. We **did not build wikilinks** —
we built the same capability on plain-Markdown syntax instead. The reasoning:

- **Zero new parser.** `asset:` already works by letting `marked` parse an
  ordinary link and inspecting the URL scheme *post-hoc* (`render.ts`). A
  `note:` link rides that exact path. `[[…]]` would have needed a pre-Markdown
  extract step (like `extractMath`) **plus** a title→id resolver.
- **Files-as-truth.** IDs are ULIDs. `[[01KXGCF248…]]` tells a human reading the
  `.md` nothing; `[Q3 planning](note:01KXGCF248…)` keeps the title in the file
  and still renders as a link in any other Markdown viewer.
- **Rename-proof.** The ULID is the target, so retitling the other note never
  breaks the link — the failure mode title-based wikilinks have.
- `![[board-id]]` (roadmap.md:71, board transclusion) stays free.

**Navigation:** the owner chose sliding panes over a back/forward stack, real
Tauri windows, or tabs — in a PKM the path between notes *is* the knowledge, so
the trail stays visible rather than being a history you can't see.

## What we implemented

- **`render.ts`** — `ResolvedNote`/`NoteResolver` + `resolveNotes()`, a
  structural twin of `resolveAssets()`. `renderInto` takes `resolveNote` as an
  **optional 4th param** (without it a `note:` link stays a plain link), which
  kept all 22 existing call sites untouched. Chips are built with
  `createElement` + `textContent`, never `innerHTML` — `render.ts` assigns
  marked's output to `innerHTML` and there is no DOMPurify in the project, so
  this deliberately adds **no new HTML sink** (a test pins it).
- **The chip is a `<button>`, not an `<a>`.** There is nothing for a `note:`
  href to navigate to, and a real href would send the webview to a dead scheme
  if the handler ever missed; a button is focusable and Enter/Space work natively.
  `data-type` is a CSS hook, so no type→icon map lives in JS (same trick as
  `Card.svelte:38`). Styled in the global `app.css` beside the asset elements —
  scoped component CSS cannot reach render.ts-built nodes.
- **`NotePanel.svelte`** — `noteRef()` beside `assetRef()` (shared
  `escapeLabel`); `refFor()` picks syntax by type. The `/` menu **dropped its
  `type === 'asset'` filter** and now excludes only the current note (a
  self-reference is never what it's for); each row shows its type. One
  **delegated** click listener on `.read` handles every chip (they're created
  outside Svelte, so per-chip binding would re-bind on every render).
- **`App.svelte`** — `openId: string` → **`openIds: string[]`**, the trail.
  `openNote` starts a fresh trail; `pushNote` appends, or **truncates** if the
  target is already open (two panes over one file = two editors over it);
  `closeFrom(i)` drops pane `i` and everything downstream (it was reached
  *through* `i`).

## The structural change worth knowing

`NotePanel` used to own its own `.overlay` + `.backdrop` — a self-contained
modal. That **cannot repeat per pane**, so the overlay, backdrop, and the new
`.trail` row moved up to `App.svelte`; a pane is now just `<article class="panel">`.
The `wide` full-screen preference moved with them (it's the trail's property, not
a pane's) and is passed back down as a prop, with the toggle button still in the
pane header where it visually belongs.

`.panel.wide` became **`.panel.wide.solo`**: a lone pane still fills the viewport
(today's reading default, preserved exactly), but once a second pane joins, panes
fall back to their column width — otherwise "full screen" would hide the trail
that following a link exists to show.

## Two things this session surfaced

- **`svelte-check` is not in the CI gate.** `pixi run ci` =
  `test, test-ui, deny, checks, docs` — no `pnpm -C ui check`, so nothing in CI
  typechecks `.svelte` files. Run it by hand after component work
  (`pixi run pnpm -C ui check`). Worth adding to `[tasks.ci]`.
- **`App.flow.test.ts` leaks mock state across tests.** `mock.ts` keeps
  `bodyOverrides` in module scope, so the edit-walk test rewrites the GAE note's
  body for every later test in the file. The new tests open the *Muesli* note to
  dodge it. A `resetMock()` export would be the real fix.

Driving the flow also caught a genuine bug the unit tests could not: `pushNote`
called `trailEl.scrollTo`, which **jsdom does not implement** — an unhandled
rejection. Now guarded (`?.scrollTo?.()`) like the `localStorage` access beside it.

## Not built (deliberately)

- **Backlinks** — "what links *here*". Needs a link index (scan bodies on
  reindex, or a `links` table in `index.sqlite`). The other half of
  `roadmap.md:116` and the real PKM payoff; deserves its own change.
- Hover popover previews — the owner picked always-visible chips instead.
- `resolveNote` calls `get`, which over-fetches the body a chip ignores. Cheap
  enough at one call per reference; revisit if a link-dense note measures slow.
