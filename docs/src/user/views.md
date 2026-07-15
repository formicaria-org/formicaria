# Views

Every view is the same data seen a different way — "a query plus a renderer".
Switch views from the top bar.

- **Board** — a Kanban board. Group by any property (status, type, project,
  tags, or a custom key) via the *group by* control. Drag a card to a column to
  set that property; the change writes straight back to the note's frontmatter.
- **Agenda** — dated, unfinished items. Three layouts via the sub-toggle:
  - **Month** — a calendar grid; items land on their due day, tinted by urgency
    (overdue / soon / this week / later), hard deadlines marked ◆.
  - **Week** — a denser single-week grid.
  - **List** — a sorted, soonest-first list.
- **Timeline** — a journal: every note grouped under the day it was created,
  newest first.
- **Gallery** — a grid of your assets, with thumbnails.
- **Search** — full-text search across every note (and the extracted text of
  ingested PDFs). Type to filter; click a result to open it.

Urgency and grouping are **derived**, never stored — there is no priority field;
nudging a note's `due` date is the whole reprioritization gesture.

## Theme

Toggle light / dark with the sun/moon button. Your choice is remembered; the
first launch follows your OS preference.
