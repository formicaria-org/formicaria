# Design system

The UI has no CSS framework and no runtime style dependency — just CSS custom
properties in `ui/src/app.css`, organized in three layers so that a theme is one
file and renderers never need editing to be re-skinned.

## The three layers

1. **Scales (theme-agnostic)** — the raw vocabulary:
   - spacing: `--space-1 … --space-8` (a 4px base)
   - type: `--text-xs … --text-2xl` with paired `--lh-*` line-heights, and
     `--measure` (the reading column width)
   - `--radius-{sm,md,lg,pill}`, `--shadow-{sm,md,lg}`
   - motion: `--ease`, `--dur-fast`, `--dur-med`
   - layout: `--rail-w`, `--header-h`

2. **Semantic tokens (per theme)** — meaning, not raw values. Defined for dark
   (default) and overridden under `:root[data-theme="light"]`:
   - surfaces: `--bg`, `--surface`, `--surface-elevated`, `--surface-hover`
   - lines: `--border`, `--border-strong`
   - text: `--text`, `--text-muted`, `--text-subtle`
   - accent: `--accent`, `--accent-hover`, `--accent-contrast`, `--accent-subtle`
   - state: `--danger-*`, `--ok-*`; value/urgency tints (`--tint-*`, `--u-*`)

3. **Legacy aliases** — the token names renderers already use
   (`--card-bg`, `--muted`, `--column-border`, `--tag-bg`, …) alias the semantic
   tokens. This is why adding the light theme required **zero** renderer edits.

## Rules of thumb

- **Use tokens, not literals.** Reach for `--space-3`, `--text-sm`,
  `--radius-md` rather than hand-picked px/rem, so rhythm stays consistent.
- **Tints are value-keyed.** Status/urgency colors are applied via
  `[data-value="…"]` / `[data-urgency="…"]` selectors in `app.css`, never in a
  renderer (see [How to add a feature](./adding-features.md)).
- **Focus is global.** One `:focus-visible` rule styles every interactive
  element; don't remove outlines locally.
- **Respect reduced motion.** Transitions collapse under
  `prefers-reduced-motion` — keep animations compositor-only (`transform` /
  `opacity`).
- **A header shows identity + primaries; everything else goes behind `⋯`.** For a
  note/board/discussion pane header the operational test is: **visible** = identity
  (which vault, who edited) plus what *mutates the note's primary content in place*
  (the status chip, Edit/Details); the **`⋯` "more actions" overflow** = lifecycle /
  cross-cutting / destructive actions (Copy to…, Delete), with the destructive one
  **last and red**. The trigger is `⋯`, **never `＋`** — `＋` means *add / create*
  (it is already "add media" and "new note"). When a new action is kind-specific,
  let the *action* declare which kinds it applies to; never keep a per-kind list of
  actions (that is a menu fork). This keeps the overflow from re-accreting into a
  junk drawer.

## Adding a theme

Copy the `:root[data-theme="light"]` block, rename the theme, and change only the
semantic token values. Wire it into the theme toggle in `App.svelte` (which sets
`document.documentElement.dataset.theme` and persists to `localStorage`).
