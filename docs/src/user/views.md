# Views

Every view is the same data seen a different way — "a query plus a renderer".
Open a view into a **pane** from the top bar; each pane has its own picker, so several
views can be on screen at once, side by side.

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
- **Activity** — who changed what, and when, read straight out of each vault's git
  log. Nothing is stored for it: git already knows, so this is a view of history
  rather than a record of its own. It is also where the contributor filter comes
  from — hiding a person there hides their notes everywhere.
- **Search** — full-text search across every note (and the extracted text of
  ingested PDFs). Type to filter; click a result to open it.

The Board, Agenda and Timeline show **notes only**. An [asset](./assets.md) is a
file a note refers to, not something you plan, so it never gets a card of its
own — open one from the note that references it, or find it in Search, which does
cover assets (including the text extracted from a PDF).

Urgency and grouping are **derived**, never stored — there is no priority field;
nudging a note's `due` date is the whole reprioritization gesture.

## Saved views (`.view` files)

A **view** is a saved query drawn through one of the built-in renderers — *"a filtered
board"*, *"this week's lab agenda"*, *"everything tagged `reading` that isn't done"*. You
write one as a small YAML file in your vault under `views/`, ending in `.view`; it then
appears alongside Board / Agenda / Timeline in the top bar and in each pane's view picker.
Because it lives in the vault, it is
**git-tracked and travels to collaborators** — a shared view is shared exactly like a note.

The simplest view is two lines:

```yaml
name: All my notes
view: timeline
```

A filtered, grouped board:

```yaml
name: Lab board
view: board          # board | agenda | timeline  (see the note below)
group_by: status     # board only
filter:              # every entry is ANDed onto the renderer's own filter
  - tag: lab
```

### The filter

`filter` is a list; a note must satisfy **every** entry. Each entry is one of:

| Entry | Matches |
|---|---|
| `prop: <key>` + `eq: <value>` | the property equals the value (`status`, `vault`, or any frontmatter key) |
| `prop: <key>` + `ne: <value>` | …does not equal it |
| `prop: <key>` + `exists: true` | the property is set (use `exists: false` for unset) |
| `tag: <name>` | the note carries that tag |
| `tags_any: [a, b]` / `tags_all: [a, b]` | any / all of the tags |
| `text: <words>` | full-text match (the same engine as Search) |
| `date: <key>` + `from:` / `to:` | a date property within an inclusive window |
| `not:` + one entry | the negation of it |
| `any:` + a list of entries | at least one of them (an OR) |

```

> **Only three renderers actually draw differently.** `search` and `gallery` are still
> *accepted* in a `.view` file, but there is no Search or Gallery pane renderer any more —
> both fall through to the **timeline**, so a view asking for them renders as a journal.
> Use `board`, `agenda` or `timeline` and say what you mean. (Full-text search lives in the
> top bar; assets are reached from the notes that reference them, which is why the standalone
> gallery went away.)yaml
name: Lab, due this fortnight, still open
view: agenda
filter:
  - prop: vault
    eq: lab
  - date: due
    from: 2026-07-14
    to: 2026-07-27
  - not:
      prop: status
      eq: done
```

Two things are deliberate. **Dates only compare through `date:`** — there is no `prop: due,
gt: …`, because a text/date mix-up there would return a confident wrong answer; a date window
parses real dates and cannot. And **a board/agenda/timeline view always shows notes only** —
your filter narrows *within* that, it cannot widen it to include assets.

If a `.view` file has a mistake it is still **listed, with its parse error** rather than
quietly dropped — a broken view tells you why. Running it surfaces the same error instead of
returning an empty result, so "no matches" never masquerades as "your file is wrong".

## Theme

Toggle light / dark with the sun/moon button. Your choice is remembered; the
first launch follows your OS preference.
