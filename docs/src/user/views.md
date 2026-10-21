# Views

Every view is the same data seen a different way — "a query plus a renderer".
Open a group from the sidebar — down the left of a wide window, along the bottom of a narrow one.
There is always **one view**, the place you are: choosing another view changes it, it does not open
a window beside it. **Only notes open as windows**, over the view: each has a **✕** at the top right
of its header that closes it and takes you back to the view (Back does the same). If you would
rather see a note beside the view, choose **Tiled** under *Layout* in Settings.

**Search is not a view.** Type in the search field and the notes that match drop down from it, over
whatever you are looking at; pick one to open it. ✕, Escape, Back or a tap outside puts the list
away.

## The groups

The views are gathered into five groups, and they are all ways of looking at the same notes:

| Group | What it is for | Its views |
|---|---|---|
| **Library** | everything you have written: on shelves, or browsed by tag | Shelves, Timeline |
| **Tasks** | notes with a status or a date | Board, Agenda |
| **Food** | meals, recipes, targets, a menu for the days ahead, a diet to follow | Day, Menu, Trends, Diet |
| **Fitness** | sessions, a week's timetable, how each exercise is done, a regime to follow | Week, Exercises, Regime |
| **Health** | a diary of what you track, looking back with a page for a doctor, cards to follow ([Health](./health.md)) | Diary, Over time, Guidance |

- **A group's views are tabs** across the top of its window: in Tasks, *Board* and *Agenda*; in
  Food, *Day*, *Menu*, *Trends* and *Diet*.
- **On a wide window** the groups head the sidebar.
- **On a phone** the bar along the bottom is the same on every screen: the **＋**, search, the
  menu of views, your open notes (a numbered button, there while a note is open), backing up,
  settings. The menu of views lists the same
  places, by the same names, as the sidebar of a wide window.
- **The ＋ is one control, and what it makes follows what you are looking at.** In Food it logs a
  food; in Fitness, an exercise or an activity; in Health, something new to track; anywhere else it opens the list of things to make — a note, a
  board, a discussion, a paper, and one entry for each of your
  [templates](./notes.md#templates).
- **Back** returns to where you came from. Stepping between a group's own views does not add a
  step to walk back through; a day picked from Trends does (see
  [Going back](./notes.md#going-back)).

**The Library opens on its shelves** — the place to come back to. Each note is a book, lying on its
side so its title reads across; books stand on the shelf their **`shelf`** property names, and the
ones with no shelf are on **Other** (its newest thirty; every note is in the Timeline). To put a book
on a shelf, give it a `shelf` in the note's details (a new name is a new shelf); with a mouse you can
also drag it onto another shelf, which sets the same property. A new notebook starts with a few short guides on
**Tutorials**.

In the Library, the row of tags above the timeline shows every tag on your notes and how many
carry it; tap one to see only those notes, and again to see them all. Otherwise the Library is
the timeline, and the board and the agenda in Tasks are the ones described below.

![The Board: three columns — todo, doing, done — of note cards, each showing its tags, its due
date and who last touched it](../images/board.png)

- **Board** — a Kanban board, grouped by `status`. It can group by any property
  (project, tags, or a custom key) — but that is chosen in a saved view's file,
  `group_by:`, not from a control on screen; see [Saved views](#saved-views).
  Drag a card to a column to set that
  property; the change writes straight back to the note's frontmatter. Drop it
  **where you want it** in the column — cards stay in the order you arrange them,
  which is remembered in this browser rather than written to your notes. Drag a
  column header to reorder the columns the same way.
- **Agenda** — dated, unfinished items. Three layouts via the sub-toggle:
  - **Month** — a calendar grid; items land on their due day, tinted by urgency
    (overdue / soon / this week / later), hard deadlines marked ◆.
  - **Week** — the seven days as columns and the hours down the side. An item with a time sits
    in its slot, as tall as it is long; one with only a date, or one that runs across days, is a
    bar above the hours. A line marks the time now. On a narrow screen the days scroll sideways
    and open on today.
  - **List** — a sorted, soonest-first list.

  ![The Agenda in Month layout: a calendar grid for September with notes on their due
  days](../images/agenda.png)
- **Settings** (the gear) is a short list of categories: *Look*, *Keyboard*, *Vaults & backup*,
  *Food & reminders*, *Assistant*, *Connections*, *About & updates*. Each says what it is set to beside its name; pick
  one to see only its options, or type in **Search settings** to find an option wherever it is
  (it understands "update", "backup", "shortcut" and the like). A longer explanation is under
  **More**. On a phone the list is the first screen and a category the second.
- **Timeline** — a journal: every note grouped under the day it was created,
  newest first.
- **Collaboration** — the open [proposals](./collaboration.md#proposals) across
  your vaults: notes that propose a change to a git branch, newest first. Open one
  to read it and its discussion. Notes still in conflict after a merge are listed at
  its top, under *Needs resolution*.
- **Discussions** — every discussion across your vaults in one place, so a thread
  you replied to last week is not lost inside the note it hangs off.

Two more are real views but are **not** in the sidebar, and that is deliberate:

- **Activity** — who changed what, and when, read straight out of each vault's git
  log. Nothing is stored for it: git already knows, so this is a view of history
  rather than a record of its own. It is genuinely useful and genuinely rare, so it
  lives in **Settings** rather than costing a permanent row of attention.
  It is *not* the same as the Timeline: the Timeline is your notes by the day you
  wrote them; Activity is every edit, including other people's.

  ![Settings open over the Board, listing Open Board, Open Agenda, Open Timeline,
  Open Activity, Open Collaboration and Open Discussions under a filter box](../images/settings.png)
- **Search** — full-text search across every note — its title, its text, its tags and the
  values of its properties (an author, a venue) — and the extracted text of ingested PDFs. Reached by typing into the **search box** at the top of the
  sidebar, never from the list: clicking an empty search view produces a blank pane,
  so there is nothing to click.

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
agenda"*. It is a small file in your vault, at `<vault>/views/<name>.view`, and every one of them
appears alongside Board / Agenda / Timeline in the sidebar.
Because it lives in the vault it is **git-tracked and travels to collaborators** — a shared view
is shared exactly like a note.

> **Views are written by hand at the moment, not from a screen.** There were **Save view**,
> **Rename** and **Delete view** buttons; they were withdrawn on 2026-08-31, because views are
> settled for now and authoring them deserves its own design rather than a box bolted to a pane
> header. Nothing about *reading* a view changed: a `.view` file you write yourself, or one a
> collaborator pushes to you, still lists and still opens, on the desktop and on the phone. To
> remove one, delete its file and let git carry the deletion; to rename one, rename the file.

### Writing a view

The simplest view is two lines:

```yaml
name: All my notes
view: timeline
```

A filtered, grouped board:

```yaml
name: Lab board
view: board          # board | agenda | timeline | shelf  (see the note below)
group_by: status     # board and shelf (a shelf groups by `shelf` unless told otherwise)
filter:              # every entry is ANDed onto the renderer's own filter
  - tag: lab
```

### The filter

A view can also *filter* what it shows — a `Papers` view of the notes tagged `paper`, a `Reading`
view for `reading`. `filter` is a list; a note must satisfy **every** entry. Each entry is one of:

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
> asking for `gallery` is listed as broken and says to change it to timeline or board. Full-text search is the
> box at the top of the sidebar, and images are reached from the notes that use them.

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

The view's menu therefore shows what the view narrows to, in the words of the file itself —
**filtered: status is not done** — and clicking it opens the plain, unfiltered Board (or
Agenda, or Timeline) with the same grouping, so the rest of your notes are one click away.
The sidebar takes you back.

## Renaming and deleting a view

Both are file operations, because there is no screen for either (see the note under
[Saved views](#saved-views)). Rename `<vault>/views/<name>.view`, or delete it. The notes a view
was showing are untouched either way — a view is a way of looking, and removing one removes only
the looking.

Each device has its own clone of the vault, so a view you delete on the laptop is still on the
phone until the change has been pushed from one and pulled by the other.

## Two ways to read the timeline

The timeline opens as a **feed**: each note as a post, with its picture, its first line, the
notebook it belongs to and who last touched it. In a shared vault that is one stream of everything
happening — your notes and your collaborators', newest first.

Switch to **List** in the view's menu for the original one line per note. That is the faster
view when you are hunting for a note you half-remember rather than catching up. The choice is per
window and is remembered, so you can keep one of each open.

![The Timeline as a feed: each note a post under its day, with its first lines, its tags and its
author](../images/timeline.png)

And the same notes as a list — one line each, for hunting rather than catching up:

![The Timeline in List layout: one dense row per note](../images/timeline-list.png)

The feed loads thirty posts at a time; **Show more** at the bottom loads the next thirty and says
how many are left.
