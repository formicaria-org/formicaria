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
- **Collaboration** — the open [proposals](./collaboration.md#proposals) across
  your vaults: notes that propose a change to a git branch, newest first. Open one
  to read it and its discussion. (Conflicts awaiting resolution are surfaced by the
  backup panel, not here.)
- **Search** — full-text search across every note (and the extracted text of
  ingested PDFs). Type to filter; click a result to open it.

The Board, Agenda and Timeline show **notes only** — which excludes three kinds of
note that are not things you plan. An [asset](./assets.md) is a file a note refers
to, so it never gets a card of its own — open one from the note that references it.
A **discussion message** (a reply, see [Notes](./notes.md#discussion)) belongs to
the note it is about, not to your board, so it stays in that note's discussion
panel. A **proposal** (see [Collaboration](./collaboration.md#proposals)) is a note
that proposes a change to a git branch; it lives in the **Collaboration** view, not
on your board. All three still turn up in **Search** — a message or a proposal you
cannot find in five years would defeat the point of keeping it in the vault at all.

Urgency and grouping are **derived**, never stored — there is no priority field;
nudging a note's `due` date is the whole reprioritization gesture.

## Saved views

A **view** is an arrangement you keep — *"my board grouped by status"*, *"this week's lab
agenda"* — that appears alongside Board / Agenda / Timeline in the top bar and in each pane's
view picker.

**To make one:** arrange a board, agenda or timeline the way you want it, then press **Save
view** in the pane's own header and give it a name. That is the whole thing.

Because a view lives in your vault as a small file, it is **git-tracked and travels to
collaborators** — a shared view is shared exactly like a note.

### Filtering a view

A view can also *filter* what it shows. The **Save view** box asks for one optional thing
besides the name — *only notes tagged …* — which is enough for the common case: a `Papers` view
that shows the notes tagged `paper`, a `Reading` view for `reading`.

Anything more than a single tag is still written into the view's file by hand, in the format
below — the full grammar is nine kinds of condition, and a screen for it would be a query builder.
A view you filtered by hand **keeps its filter**: saving over it from the app is refused rather
than quietly dropping it, and re-saving a tag-filtered view without touching the tag box leaves
its tag where it is.
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

> **Use `board`, `agenda` or `timeline`.** Those are the three that draw differently. A view
> asking for `search` or `gallery` is accepted but renders as a timeline. Full-text search is in
> the top bar, and images are reached from the notes that use them.

```yaml
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

### A view says what it leaves out

A view draws through the same renderer as the built-in it shadows: a `view: board` and the
Board are the same screen. So a filter that removes a whole column removes it *silently* —
and a missing column reads as missing notes.

The pane header therefore shows what the view narrows to, in the words of the file itself —
**filtered: status is not done** — and clicking it opens the plain, unfiltered Board (or
Agenda, or Timeline) with the same grouping, so the rest of your notes are one click away.
The view picker takes you back.

## Theme

Toggle light / dark with the sun/moon button. Your choice is remembered; the
first launch follows your OS preference.
