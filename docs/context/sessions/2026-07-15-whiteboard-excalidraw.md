# 2026-07-15 — Whiteboard view (embedded Excalidraw, lazy-loaded)

**Outcome:** board notes — a freeform Excalidraw canvas as a note. Verified as far
as headless allows: `pixi run ci` exit 0 (61 UI tests + Rust suites), the UI
**builds and code-splits** Excalidraw into its own chunk, and a board note
round-trips through the real server (`view: board` + scene JSON on disk, chunk
served 200). The **canvas rendering itself is unverified** — Excalidraw needs a
real browser; the owner confirms visually.

## Context / pivot

Brainstormed two features (split-view + a whiteboard). Owner first picked a
minimal hand-rolled SVG canvas, then **changed to Excalidraw** mid-build (the
started `scene.ts`/`scene.test.ts` were removed). Split-view is still queued
after this.

## What we implemented

- **A board is a note**, marked `view: board`, whose **body is the Excalidraw
  scene JSON**. No new `Kind` (the model resists that), no backend change — the
  custom property + JSON body ride the existing `set_property`/`update_body`/
  frontmatter-`extra` paths. Files-as-truth: `.excalidraw` JSON is plain text on
  disk.
- **`ui/src/lib/Whiteboard.svelte`** — lazily imports `react`, `react-dom/client`,
  `@excalidraw/excalidraw` + its CSS in `onMount`, mounts a React root into a
  Svelte host via `createElement` (no JSX → no React build plugin). `onChange`
  serializes with `serializeAsJSON` and debounces a `update_body` write (600 ms,
  skips no-ops); `onDestroy` flushes + unmounts. Same lazy discipline as
  KaTeX/Mermaid, so the ~744 KB-gz editor is a separate chunk loaded only when a
  board opens (base `index.js` stays ~34 KB gz).
- **`NotePanel`** — `isBoard = note.props.view === 'board'` renders `Whiteboard`
  instead of the textarea/read-view; the canvas is always live. Delete/full-screen/
  close stay. Theme passed through.
- **A board is treated exactly like a note** (owner ask): the header Edit button
  becomes **"Details"** for a board and opens the *same* props editor a note gets
  — Status / Start / Due / Hard / Title / Tags — rendered **above** the live
  canvas. So a board is agenda-trackable (set a Due → it shows in Agenda/Calendar),
  kanban-groupable (Status), and timeline-listed (creation day), just like any
  note. The toggle is guarded — leaving "Details" on a board does **not**
  flush the textarea `draft` over the canvas-owned body (`if (editing && !isBoard)
  await save()`); the canvas autosaves itself via `saveBoard`/`onSave`.
- **"New board"** — a command-palette entry: `capture` an empty scene →
  `set_property view=board` + a title → open. `mock.ts` returns a blank scene for
  board notes and seeds one ("Architecture sketch") for `pnpm dev`.
- **Deps:** `@excalidraw/excalidraw` 0.18.1, `react`/`react-dom` 19.2.7,
  `@types/react`(-dom) dev. Excalidraw + React 19 built with Vite with **zero**
  config changes (no `process.env` shim needed).

## Decisions

- See `decisions.md` — **Excalidraw, lazy-loaded** (a deliberate reversal of "no
  new UI runtime dep", chosen by the owner; mitigated by code-splitting +
  files-as-truth). Minimal-SVG-canvas was the road not taken.

## Notes / gotchas

- **Offline fonts deferred** — Excalidraw pulls hand-drawn fonts from a CDN unless
  `EXCALIDRAW_ASSET_PATH` is set + the ~14 MB font dir is served; offline it uses
  fallback fonts. A follow-up if full-offline fidelity matters.
- **Headless can't verify the canvas** — same class of gap as the earlier "GAE
  math" report. Build/code-split/round-trip are the verifiable parts.
- React types (`@types/react`, `@types/react-dom`) were needed for `svelte-check`
  to resolve the dynamic `react-dom/client` import.
