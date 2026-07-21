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

## Templates

To reuse a layout — a meeting scaffold, a paper-reading checklist, a daily log — make
a note with that body and **tag it `template`** (in the editor's Tags field). It then
shows up in the **＋ "make something new" menu** as **New from "…"**, once per template
— and in the Ctrl+K palette too, where you can filter to one by name. Picking it opens a
fresh note pre-filled with the template's body, ready to edit.

A template is nothing special: **just a note with the `template` tag**. Untag it and it
stops being one; there is no separate "template type" to manage, and the template itself
stays a normal, searchable note. "New from" copies only the **body** (the scaffold you
actually reuse) — the new note is its own untitled note, not another template, so the
`template` tag is left behind rather than cloned.

## Editing a note

Click any card to open it. It shows the rendered read view. To edit it, either
open the header's **＋ options** window and pick **Edit**, or — quicker —
**double-click anywhere in the note**; both land in the same place. Double-clicking
a reference chip, a link, or embedded media does what that element does instead of
opening the editor.

**To finish, click the header** — the title and the note's identity line turn into
a "done" target while you edit (an accent underline and a pointer cursor mark it),
so putting the pen down is a tap where your eye already is, not a hunt for a button.
**Ctrl+S** and **Escape** do the same. Your typing is autosaved either way, so none
of these can lose work.

The editor also gives you a **properties form**:

| Field   | What it sets              | Format                                   |
|---------|---------------------------|------------------------------------------|
| Start   | when it begins                  | a date picker, with an optional time |
| Status  | any label                 | free text (autocompletes known values)   |
| Due     | a deadline                | a date picker (`YYYY-MM-DD`)             |
| Hard    | a hard deadline           | a checkbox                               |
| Title   | an optional title         | free text                                |
| Tags    | tags                      | comma- or space-separated                |

**Status has a shortcut.** The status chip on every board and timeline card
rotates through the statuses your vault already uses, then through "no status",
one **tap** at a time — the fast way to nudge a card along. To jump straight to a
distant status (or clear it) without stepping through the rotation, **long-press
the chip** (or right-click on a laptop) for a **picker** of all your statuses. Use
either to set a status you already have without opening the editor; use the form's
Status field to invent a new one (it joins the rotation as soon as a note carries it).

Every change writes back to the note's YAML frontmatter immediately. The body is
plain Markdown, edited as literal text (KaTeX math and Mermaid diagrams render in
the read view). Nothing is ever rewritten behind your back — the file round-trips
byte-for-byte.

## Ticking things off

A task list — `- [ ] milk` — renders with a real checkbox in the read view, and you can **tap it
to toggle** without opening the editor. A tap flips just that one `[ ]`↔`[x]` in the file (the same
one byte a text editor would change — nothing else moves), strikes the line through when done, and
saves. It's the quickest way to work a checklist on a phone. Checkboxes shown inside an **embedded**
note stay read-only — you tick those off by opening that note itself.

## Formatting text

A note's body is Markdown, so **bold**, *italic*, lists, tables, headings, `code`, and
fenced ```mermaid diagrams / `$math$` all render in the read view. For a **mind-map or tree**,
a Mermaid flowchart (`graph TD`) draws the hierarchy — no plugin and no extra download.
On top of standard Markdown there are three light additions, each chosen so the raw text stays
readable in **any** Markdown editor (nothing here is app-only HTML):

- **Highlight** — `==important==` renders as a highlight. Elsewhere it reads as `==important==`.
- **Callouts** — a blockquote whose first line is `[!type]` becomes a coloured callout
  (GitHub/Obsidian style). The types are `note`, `tip`, `info`, `warning`, `danger`, `quote`:

  ```markdown
  > [!warning] Heads up
  > this stands out.
  ```

  An unrecognised type is just an ordinary blockquote. In the read view each callout shows its
  kind as a small badge — **tap the badge to change the type** from a picker, which rewrites just
  that `[!type]` word in the file (a badge inside an embedded note is a plain label, not a picker).
- **Coloured text** — `[some words]{.token}` colours a phrase. The token is a **meaning**, not a
  colour: `accent`, `info`, `ok`, `warn`, `muted` — so the note stays readable when you switch
  light/dark or change theme. An unknown token is left as the literal text.

Colours come from the theme, never a raw colour or hex code — a deliberate limit so a note written
today still looks right under a theme designed years from now, and so a note body can never carry
arbitrary HTML/styles (which would be both unportable and a security hole in a shared vault).

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

**Places & maps.** A `geo:` link opens a place in your device's own map app —
`[tap to open](geo:1.2807,103.8720)` (latitude, longitude). formicaria loads no
map itself (nothing phones home — the app stays true to that); the link just hands
the coordinates to Google Maps / OSMand / Apple Maps. Ordinary
`https://openstreetmap.org/…` links work too and open in a browser. The **Place**
template pairs both, so a saved spot is one tap from directions. An in-app
interactive map is deliberately *not* provided: live tiles would mean the app
reaching out to a server on every note you open, which is exactly what formicaria
refuses to do.

**Embed a whole note** straight from the picker. Two ways in, so it works on a phone
as well as a keyboard:

- Type **`//`** (two slashes) instead of `/` — the menu opens in **embed mode** (an
  accent frame), and a plain tap/Enter inserts an embed. No modifier key needed.
- Or in the ordinary `/` menu, press **Shift+Enter** (or Shift-click) to embed the
  highlighted result instead of linking it.

Under the hood an embed is just the reference written as an image — `![](note:<id>)`
— so you can also type it by hand, or turn an existing link into an embed by adding a
`!` in front.
Instead of a chip, the target note's content is rendered **inline**, in a card, so
a hub note can pull several others together on one screen. It is always the whole
note (never a fragment — the file is the unit), a missing target degrades to a
placeholder, and a note that embeds itself (or a loop) stops with a small marker
rather than recursing forever.

**Click a chip** and that note opens in a pane to the right, with the note you
came from still on screen — follow a chain and the whole trail stays visible, so
you can see the path you took. Closing a pane also closes everything to its
right (you reached those *through* it); closing the first closes the trail.

Open a note and, below its content, a **Linked from** panel lists every note whose
body references *this* one (a `note:` link or an embed) — the backlinks, so you can
walk the connection in either direction. It appears only when something links here,
and is derived by scanning (no index to keep in sync).

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
