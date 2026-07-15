# 2026-07-15 — Search-first sidebar (drop quick-capture)

**Outcome:** the sidebar top is now **search-first**. Verified green:
`svelte-check` 0 errors, 61/61 vitest, prod build. UI-only change.

## Why

The owner noticed the "New board" button overflowed the narrow "Capture a
note…" composer box, and questioned what the capture box was even for. It was a
Logseq-style quick-capture (type + Enter → a note whose *body* is that text) —
but with **New note** opening a full editor and a ⌘K palette, it was a redundant
second input (capture in the sidebar, search in the stage), which is what made
the layout feel confusing.

Owner picked **"Search-first, two buttons"** from a previewed set of layouts.

## What changed (all in `ui/src/App.svelte` unless noted)

- **Removed the quick-capture composer** (`.composer` form + `onCapture` +
  App-level `draft` state).
- **Search is now the primary input**: a persistent `.searchfield` at the top of
  the sidebar (magnifier icon + `<input type="search">`), bound to `searchQuery`,
  `oninput` runs the existing debounced search, `onfocus` switches to the search
  view. The separate **"Search" nav item is gone**, and so is the **duplicate
  in-stage search box** (the header no longer renders one for `view === 'search'`).
- **Two stacked full-width create buttons** below the field: **New note**
  (primary/filled) and **New board** (`.secondary`, outlined — two filled red
  buttons stacked was too loud). Fixes the overflow.
- **Keyboard map** updated: `/` focuses the search field (was: just switch view);
  `c` now creates a **New note** (was: focus the capture box); number keys are
  **1–3** (Board/Agenda/Timeline) since Search is no longer a nav item. Palette:
  dropped "Capture a note"; "Search notes" now focuses the field.
- **Mock fidelity** (`ui/src/lib/mock.ts`): `update_body` now derives a plain
  note's card **preview** from the body's first non-empty line (minus a leading
  `#`), mirroring the real backend — a board keeps its title (its body is scene
  JSON, never surfaced). This lets a freshly-written note show its text on the
  board without the old quick-capture path.
- **Tests** retargeted off the removed capture box:
  `App.flow.test.ts` step 2 now does New note → type body → leave edit → close →
  assert it lands on the board; `App.features.test.ts` delete test creates the
  throwaway via New note and asserts the double-confirm via panel open→close.

## Notes

- `capture` (the ipc) is unchanged and still used by New note / New board — only
  the UI quick-capture *box* is gone.
- Collapsed rail / mobile breakpoint hide `.create` (renamed from `.composer`),
  same as before.
