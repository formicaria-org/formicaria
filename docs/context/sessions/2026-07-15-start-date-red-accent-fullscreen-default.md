# 2026-07-15 — Optional start date, red brand accent, full-screen-by-default

**Outcome:** three owner-requested refinements, verified (`pixi run ci` exit 0 —
29 Rust suites + 61 UI tests; live server smoke of the `start` round-trip). Built
on the launcher-UX work.

## What we implemented

- **Red brand accent, matching the app icon (`#dc2626`).** Retinted the accent
  layer in `ui/src/app.css` for both themes (accent/hover/contrast/subtle/link/
  focus-ring) from the old amber to the icon's red — dark uses a brighter red-500
  for interactive text/rings, light a darker red-700 for legibility, buttons the
  exact icon red. Added the ant as the browser **favicon** (`ui/public/favicon.svg`,
  a copy of `packaging/formicarium.svg`) + `<link rel=icon>` and `<meta
  name=theme-color content=#dc2626>` in `ui/index.html`. The urgency palette
  (overdue red, soon amber, week blue, later green) is unchanged — a distinct role
  (small dots/bar edges) from the brand accent (buttons/links/focus).
- **Optional, settable `start` date; no default start or due.** Notes already had
  no default due; the calendar had been *assuming* creation day as the bar's
  start. Added a real optional `start: Option<Date>` field (distinct from the
  `created` timestamp), unset by default and user-settable, threaded through all
  four layers: `fm-model` (`Object.start` + `get("start")`), `fm-core`
  (`frontmatter` write/parse `start:`, `apply_property` "start" arm mirroring
  "due"), `fm-app` (`ObjectMeta.start`), and the UI (`types.ts`, `mock.ts`, a
  **Start** date input in `NotePanel` next to Due). `Calendar.svelte` now spans
  `start`→`due`; with no start it's a **single-day marker on `due`** (creation is
  never assumed). The agenda stays due-driven, so a note appears once it has a due.
- **Note panel full-screen by default.** `NotePanel`'s `wide` now defaults to true
  and the choice is remembered in `localStorage['fm-note-wide']` (like the theme);
  the toggle shrinks to the docked side-sheet.

## Decisions

- **`start` is a first-class date field, not `created`.** `created` is an
  immutable creation timestamp (not editable); a settable "when work begins" is a
  separate optional field. Calendar geometry keys off `start`/`due`, never
  `created` — so "no default start" is literally true.
- **Single-day-on-due when start is unset**, rather than a creation→due bar. Most
  notes are just a deadline; the multi-day bar is opt-in by setting a start.
- **Brand red ≠ urgency red.** Kept them separate; they occupy different UI roles.

## Notes / gotchas

- **Guard every `localStorage` access.** The vitest/jsdom env has no
  `localStorage`; an unguarded `localStorage.getItem` at component init (the
  full-screen default) crashed NotePanel on mount and failed 3 tests. Wrapped in
  try/catch (the default — full screen — applies when storage is unavailable),
  matching `App.svelte`'s existing pattern.
- `isoDate` in `calendar.ts` is now unused by the app (kept as a tested utility);
  the Calendar import dropped it.
