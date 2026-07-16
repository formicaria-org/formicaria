# Views

Every view is the same data seen a different way — "a query plus a renderer".
Switch views from the top bar.

- **Board** — a Kanban board. Group by any property (status, project, tags, or a
  custom key) via the *group by* control. Drag a card to a column to set that
  property; the change writes straight back to the note's frontmatter. Drop it
  **where you want it** in the column — cards stay in the order you arrange them,
  which is remembered in this browser rather than written to your notes. Drag a
  column header to reorder the columns the same way.
- **Agenda** — dated, unfinished items. Three layouts via the sub-toggle:
  - **Month** — a calendar grid; items land on their due day, tinted by urgency
    (overdue / soon / this week / later), hard deadlines marked ◆.
  - **Week** — a denser single-week grid.
  - **List** — a sorted, soonest-first list.
- **Timeline** — a journal: every note grouped under the day it was created,
  newest first.
- **Search** — full-text search across every note (and the extracted text of
  ingested PDFs). Type to filter; click a result to open it.

The Board, Agenda and Timeline show **notes only**. An [asset](./assets.md) is a
file a note refers to, not something you plan, so it never gets a card of its
own — open one from the note that references it, or find it in Search, which does
cover assets (including the text extracted from a PDF).

Urgency and grouping are **derived**, never stored — there is no priority field;
nudging a note's `due` date is the whole reprioritization gesture.

## Theme

Toggle light / dark with the sun/moon button. Your choice is remembered; the
first launch follows your OS preference.
