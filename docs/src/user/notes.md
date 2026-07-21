# Creating & organizing notes

## Making something new

- **New note** — creates a blank note and opens it straight in the editor, so you
  can write a body and set properties.
- **New board** — creates a whiteboard: a note whose body is a freeform canvas.
  It is a note in every other way, so a dated board shows up in the Agenda and a
  statused one groups on the Board.

There is no "type" to pick. Everything is a **note**, and you tell them apart with
**tags** — the only surviving distinction is `asset`, which the app sets itself when
you ingest a file. With more than one vault configured, an **in** selector beside
these buttons chooses which vault the new file lands in.

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
| Start   | when it begins                  | a date picker, with an optional time |
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

## Discussion

Every note has a **Discussion** panel below its read view — collapsed, showing a
count, opened with a click. It is where the conversation *about* a note lives:
questions, second opinions, "why did we drop the second arm?" — the reasoning
that used to end up in a chat app where it is unfindable in a year.

**Reply** posts a message (Ctrl/Cmd+Enter sends it). Reply to the note, or to a
particular message — either way it joins the same discussion, indented under what
it answers. Each message carries who wrote it and, when the note is shared, a
badge for the vault (audience) it belongs to.

The design is the same one the rest of the app uses: **a message is just a note.**

- One message is **one Markdown file**, ULID-named, living in the **same vault as
  the note it is about** — so the conversation shares exactly the audience the
  note does, and travels with it to collaborators through git like everything else.
  Two people replying at once are writing different files, so there is nothing to
  conflict.
- Because messages are notes, they are **fully searchable** — a decision recorded
  in a reply is as findable in five years as one in a note body.
- But they are **kept out of the planning views**: a reply never appears on the
  Board, the Agenda or the Timeline, and is never offered as a `/` link target, so
  a busy thread cannot bury your real notes. (This is why `thread_of` / `reply_to`
  are managed for you and cannot be set by hand — see the
  [frontmatter reference](../reference/frontmatter.md).)
- Deleting a note **does not delete its discussion**: the reasoning about a
  decision outlives the note that prompted it.

### Standalone discussions

A discussion does not have to hang off one note. **New discussion** (in the `+`
menu) starts a discussion that **stands on its own** — a place to talk about a
topic, or about several notes and whiteboards at once. Give it a title, then talk;
as the conversation goes, link whatever notes or whiteboards are relevant **inside
your messages** (a `note:` link renders as a chip), rather than fixing the subject
up front.

All your standalone discussions are listed in the **Discussions** view, most
recently active first, showing each one's title, its vault, and who has posted —
so you can see at a glance where the conversation is happening. Open one and it
*is* the conversation (there is no document body to edit; the messages are the
content).

Under the hood a discussion is still just a note — one that is the root of its own
thread — so it is searchable, travels through git, and stays out of the Board and
Timeline like any other message. There is no new "type"; the same reply mechanism
carries it.

## Organizing

There are no folders to manage. You organize by **properties** and see the
results through views: a status board, an agenda of due dates, tag-based boards,
custom properties like `project`. Add any property you like — a new key in a
note's frontmatter flows straight through to grouping with no configuration.
