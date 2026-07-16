# Creating & organizing notes

## Capture vs. New

- **Capture** — the top box. Type a line and press Enter to create a note
  instantly (its text becomes the body). Pick a **type** (note / task / meeting)
  from the dropdown to label it as you capture.
- **New** — creates a blank note of the chosen type and opens it straight in the
  editor, so you can write a full body and set properties.

## Editing a note

Click any card to open it. It shows the rendered read view. To edit it, either
click **Edit** in the header or **double-click anywhere in the note** — the
shortcut saves you the trip to the header, and both land in the same place.
**Ctrl+S** saves and drops you back to the read view, as does Escape or the
**Done** button (your typing is autosaved either way). Double-clicking a
reference chip, a link, or embedded media does what that element does instead of
opening the editor.

The editor also gives you a **properties form**:

| Field   | What it sets              | Format                                   |
|---------|---------------------------|------------------------------------------|
| Type    | `note`/`task`/`meeting`/`asset` | choose from the dropdown           |
| Status  | any label                 | free text (autocompletes known values)   |
| Due     | a deadline                | a date picker (`YYYY-MM-DD`)             |
| Hard    | a hard deadline           | a checkbox                               |
| Title   | an optional title         | free text                                |
| Tags    | tags                      | comma- or space-separated                |

**Status has a shortcut.** The chip beside the note's title — and on every board
and timeline card — rotates through the statuses your vault already uses, then
through "no status", one click at a time. Use it to set a status you already have
without opening the editor; use the form's Status field to invent a new one (it
joins the rotation as soon as a note carries it).

Every change writes back to the note's YAML frontmatter immediately. The body is
plain Markdown, edited as literal text (KaTeX math and Mermaid diagrams render in
the read view). Nothing is ever rewritten behind your back — the file round-trips
byte-for-byte.

## Linking notes together

A note can reference another note. While editing, **type `/`** and search — the
menu lists your notes and [assets](./assets.md) together, labelled by type. Pick
a note and it inserts an ordinary Markdown link:

```markdown
Follow-up on [the clipping ablation](note:01KXGCF248QC70Z7NB4E22A9QJ)
```

That is deliberately plain Markdown, so your note stays readable — and still
renders as a link — in any other editor. The link points at the other note's
**id**, not its title, so renaming that note never breaks the reference.

In the read view the reference becomes a **chip** showing the target's current
title and status, so you can see what a note points at without opening it. A
reference whose note no longer exists shows as a dashed placeholder rather than
disappearing.

**Click a chip** and that note opens in a pane to the right, with the note you
came from still on screen — follow a chain and the whole trail stays visible, so
you can see the path you took. Closing a pane also closes everything to its
right (you reached those *through* it); closing the first closes the trail.

> Backlinks — seeing what links *to* a note — are not built yet.

## Organizing

There are no folders to manage. You organize by **properties** and see the
results through views: a status board, an agenda of due dates, tag-based boards,
custom properties like `project`. Add any property you like — a new key in a
note's frontmatter flows straight through to grouping with no configuration.
