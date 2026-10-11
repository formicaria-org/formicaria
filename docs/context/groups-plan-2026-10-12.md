# Four groups over notes — the plan (2026-10-12)

> **Partly withdrawn the day it was adopted.** After using it on his phone the owner took out the
> group's own bar, *Everything*, *Weight* and every graph laid over a view: see the last four
> 2026-10-12 entries of `decisions.md#ui`, which win over this file. What stands: the frame, the
> four groups, templates, the side panel and the tabs, editable recipes.
>
> **Adopted by the owner on 2026-10-12** (*"as recommended, start with phase 0"*); the ruling is
> `decisions.md`, 2026-10-12, *four groups over notes*. Where this disagrees with the code, the
> code wins. It was made in three passes — a draft, a critic and an advocate reading the code
> independently, and a third pass that settled their disagreements; none of the three built or
> ran anything.

## The frame

- **Note**: one Markdown file with a body, tags and properties. Nothing else is stored.
- **Template**: a note tagged `template`. "New from template" copies its body, its tags minus
  `template`, and its custom properties plus `status`, in one `create_notes` write.
- **Never copied**: `id`, `created`, `updated`, `schema`, `title`, `start`, `due`, `log`,
  `thread_of`, `reply_to`, `proposes`, `group`, any `conflict-*` key.
- **Templates are inert**: never a program in force, a recipe, or a card in Tasks.
- **View**: a filter over notes drawn by one of the six existing renderers.
- **Group**: a name over some views, one add action, and the templates whose `group` names it.
- **Tags classify, properties drive**: only the Library reads tags.

## The four groups

| Group | Shows | Views (renderer) | Add | Templates |
|---|---|---|---|---|
| Library | every planning note; logged entries in an opt-in view | Notes (timeline, with a tag chip row), Search, Everything (timeline) | new note / from template | meeting, plain |
| Tasks | notes with a status or a date | Board, Agenda (with its week grid) | task template | task |
| Food | `log: food` entries, programs, recipes | Day, Trends, Weight (totals) | the food sheet, with editable recipes | recipe, diet program |
| Fitness | `log: workout` entries, programs | Session, Volume (totals) | the workout sheet | training program, session plan |

## Switching

- **Desktop**: four entries at the top of the side panel, then the group's views; the tab strip
  moves from `TrackerPane` into `Pane.svelte`.
- **Phone**: the bar is always a group's. Left, a control that lists the four groups, then search,
  windows, vault, backup, undo and settings. Up to four views around the raised add control.
- **Back**: closes an open sheet first. Within a group a step replaces the history entry;
  changing group pushes one. Library is home.

## Phases — each ships alone, gated by `pixi run ci` and the check named

| # | What | Gate |
|---|---|---|
| 0 | The ruling; the bars, the group list and one better screen per group, on this device only (`fm-spaces`) | the owner accepts it on the phone |
| 1 | Templates: copy rule, inertness, starter templates | a `program: food` template is never in force; a task template is not on the board |
| 2 | Groups and switching for everyone; desktop panel and tabs | Back order; the agenda keeps *Show as* |
| 3 | Library: tag chips for real, *Everything* | `tests/perf.rs`: *Everything* over 10,000 entries inside the board's per-object budget |
| 4 | Recipes: visible recipe note, ingredients added, removed, changed | old recipes still listed; no logged total changes |
| 5 | Visual pass, phone then desktop | a real phone build before any tag |

## Phone quality — from `ui/src/lib/charts`, no dependency

A 14-day bar strip above the agenda (tap picks the day); a ring on Tasks for the share in the last
status column, taken from the `statuses` order; meter rows for tag counts; notes per day in the
Library; changes timed through `motion.ts`; at most four tabs.

## What the two reviews disagreed on, and how it was settled

| Question | Settled for | On what |
|---|---|---|
| Recipe ingredients as a list property in one note | the critic: no | no write path (`fm-core/src/edit.rs`, `fm-app/src/log.rs` `as_written`, `NotePanel.svelte` joins a list into text); a custom key merges as one value (`merge.rs`); stored totals contradict *totals are stored nowhere* |
| The recipe's own note visible | the advocate: yes | it needs no `log`; `source: recipe/<word>` still resolves |
| A tags view | the advocate: none needed | rows already carry tags; `group_by: tags` buckets by the whole list |
| Starter templates embedded in the binary | the critic: no, real notes | "a template is a tagged note" |
| Tasks and Library as `.view` files | neither | the agenda would lose *Show as* (`switchesHead` keys on the pane's kind); a search view takes no typed query |

## Not verified when adopted

- How a recipe made from a template, outside the sheet, gets the word joining it to its ingredients.
- Whether the timeline is bounded at 10,000 rows.
- Whether `get` returns a note's properties to the UI, which the template copy rule depends on.
- No timing of any of it.
