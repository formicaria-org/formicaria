# 2026-08-24 — a column that was there, and a count that was not

**The report.** *"My done column in the board is not showing up, even though I have done notes.
Also, when I backup I still see 1 not in history, which should be solved by back up no?"*

Two sentences, two different failures — and neither of them was in the thing being blamed. The
notes were on disk, the board had the column, backup had recorded the note. Both were **surfaces
that would not say what they knew.**

## The column: the board was right, and two things can hide a column silently

Asking the running server directly settled the data question in one call:

```
POST /api/board {"groupBy":"status"}  →  doing(6)  done(3)  (none)(9)  thinking(3)
```

The column was in the payload. Nothing in the UI drops one either — `boardOrder.arrange` reorders
and appends but never omits, `Pane.svelte` keeps a column whose cards are all vault-filtered away,
and the CI grep keeps status literals out of the renderers entirely. So the column was not missing;
it was **not on screen**, and there are exactly two ways for that to happen:

1. **A saved view filtered it out.** The vault has one `.view` file, `Active`, and its filter is
   `not: {prop: status, eq: done}`. A `view: board` renders through the *same* `Board.svelte` as
   the Board pane — same pixels, one column fewer, and nothing anywhere naming the filter.
2. **It was off the right-hand edge.** `.board` is one horizontally scrolling strip, and under
   `@container pane (max-width: 40rem)` a column takes `min(85vw,22rem)` and the strip snaps — one
   column at a time, with no indication of how many there are.

The lesson is the one this repo keeps re-learning in other forms: *an absence explains nothing by
itself.* The manual already said it for a **broken** `.view` — "a broken view tells you why … so
'no matches' never masquerades as 'your file is wrong'" — and the same sentence, applied to a
**working** view, is the whole fix. So:

- `run_view` now returns `filters`: one plain-English phrase per `filter:` entry
  (`views::describe_pred`, written against the file's own DSL, negation carried *into* the phrase so
  it reads `status is not done` rather than `not (status is done)`). The pane header shows it as
  **`filtered: status is not done`**, and clicking it opens the unfiltered built-in renderer with
  the same grouping.
- The board grew a **rail**: every column by name and count, the one at the edge marked, click to
  jump.

**Where the words live turned out to be the real design decision.** They started on `ViewInfo`
(from `list_views`, fetched once per vault change) — economical, and wrong: that fetch can arrive
late, and on the phone it has failed outright, which would leave a filtered board with no
explanation. That is the reported bug, reintroduced through the fix's own data source. They ride on
`ViewResult` instead, arriving with the very rows they describe, from the same parse.

A note for whoever writes the next renderer test: the rail names every column, so a column label now
appears **twice** in the document. `App.flow.test.ts` broke on `getByText('alpha')` finding two
nodes. The rail entry reads `alpha ·` (separator in its own text node), which keeps an exact-text
query pointed at the column header — a small piece of markup doing a real job.

## The count: backup did record it, and the toolbar never asked again

`backUpNotes` runs `commit → push` over every vault, and since 2026-08-20 a commit records the whole
backlog, not just this process's writes (`decisions.md#data`, pinned by
`fm-app/tests/backup_records_everything.rs`). All of that worked. But `unrecordedList` was loaded
**once per `vaults` change**, and again only after the Unrecorded panel's own Record button — so
after a backup the chip went on displaying the number it had read at startup.

From the only place the owner works, a durability action that appears not to have worked is
indistinguishable from one that did not. `backUpNotes` now re-reads the count in its `finally`, and
so does the debounced auto-commit — the two moments git actually changes. The mock's `commit` arm
had to start answering like the Rust (it cleared nothing, so no test could see through it), which is
the second time in two sessions that **a mock too polite to model failure was the reason a defect
was invisible**.

By the time this was diagnosed the vaults were already clean (`unrecorded` → `[]`), so the "1" on
screen was a label over an empty vault — the fix is that the label cannot outlive the fact again.

## What went in

- `crates/fm-app/src/views.rs` — `describe_pred` + `ViewResult.filters` (3 new tests, 10 in the module).
- `ui/src/lib/Pane.svelte`, `ui/src/lib/panes.ts`, `ui/src/App.svelte`, `ui/src/lib/types.ts`,
  `ui/src/lib/mock.ts` — the chip, and the feed that carries its words.
- `ui/src/renderers/Board.svelte` — the column rail (measured overflow; CSS decides visibility).
- `ui/src/App.svelte`, `ui/src/lib/mock.ts` — the unrecorded count re-read after backup and commit.
- New tests: `App.filteredView.test.ts`, `Board.rail.test.ts`, `App.backupClearsUnrecorded.test.ts`
  (the last two verified failing before the fix).
- `docs/src/user/views.md` — the new section, plus a fence repair that had been rendering a whole
  prose paragraph as a code block.

`pixi run ci`: green (376 UI tests, 83 Rust test binaries).
