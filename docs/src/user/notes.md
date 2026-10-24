# Creating & organizing notes

## Making something new

- **New note** — creates a blank note and opens it straight in the editor, so you
  can write a body and set properties.
- **New board** — creates a whiteboard: a note whose body is a freeform canvas.
  It is a note in every other way, so a dated board shows up in the Agenda and a
  statused one groups on the Board.

There is no "type" to pick. Everything is a **note**, and you tell them apart with
**tags** — the only surviving distinction is `asset`, which the app sets itself when
you ingest a file. With more than one vault, the ＋ menu ends with a
**Create in** list — pick the notebook, then pick what to make.

## Templates

If you keep writing the same kind of note — a meeting, a task, a weekly review — make one note
shaped the way you want and **tag it `template`** (in the editor's Tags field). It then shows up
in the **＋ "make something new" menu** as **New from "…"**, once per template.

A note made from a template arrives with the template's **structure**:

- its body (headings, checklists);
- its tags, except `template` itself;
- its status, and any properties you gave it.

It does **not** get the template's title, its dates, or anything that says which note the
template was. So a template tagged `meeting` makes notes tagged `meeting`, and a template with a
status makes notes that land in that column of the board.

A template is nothing special: **just a note with the `template` tag**. Untag it and it stops
being one. It is still an ordinary note you can search for and edit. What it is *not* is one of
the things it makes: a template with a status is not a card on the board or the agenda, and a
template for a diet program does not set your targets.

**Add starter templates**, in the same ＋ menu, writes three ordinary notes into your vault — Task,
Meeting, Recipe — which you can change or delete like any other. Each has a heading and its
sections to fill in, a tag, and the property a view reads. None holds a target or a suggestion.
The item disappears once you have all three.

## Editing a note

Click any card to open it. It shows the rendered read view, under a header that is just the
note's title and a **＋**. **To edit, click or tap anywhere in the note's text**: the editor opens
with the cursor where you clicked. A tick box, a table cell, a link, a chip to another note or a
player does what it does instead of opening the editor, and selecting text to copy it is not a
click. From the keyboard, press **Enter** with the note in focus.

**There is no Done button.** To finish, **tap the title**, press **Escape** or **Ctrl+S** — or simply
leave the note: switch to another window, close it, or press Back on a phone. Your typing is autosaved
throughout, so none of these can lose work.

Everything else a note offers is under the same **＋**: **Add media** (a photo, a video, a recording or a
file — from reading too, in which case it opens the editor and adds at the end), **Full screen** for a
whiteboard, **Copy to…** another vault, **Delete** and **Close**. At the bottom of that window you can
see which vault the note is in and who last edited it.

The editor also gives you a **properties form**:

| Field   | What it sets              | Format                                   |
|---------|---------------------------|------------------------------------------|
| Start   | when it begins                  | a date picker, with an optional time |
| Status  | any label                 | free text (autocompletes known values)   |
| Due     | a deadline                | a date picker (`YYYY-MM-DD`)             |
| Hard    | a hard deadline           | a checkbox                               |
| Title   | an optional title         | free text                                |
| Tags    | tags                      | comma-separated (a tag may contain spaces) |

**Status has a shortcut.** The status chip on every board and timeline card
rotates through the statuses your vault already uses, then through "no status",
one **tap** at a time — the fast way to nudge a card along. To jump straight to a
distant status (or clear it) without stepping through the rotation, **long-press
the chip** (or right-click on a laptop) for a **picker** of all your statuses. Use
either to set a status you already have without opening the editor; use the form's
Status field to invent a new one (it joins the rotation as soon as a note carries it).

Every change writes back to the note's YAML frontmatter immediately. The body is
plain Markdown, and **the editor draws it finished while you write**: a heading looks like a
heading, bold is bold, a link shows its words, a list has its dots and a task its tick box.
**The line you are writing on shows its Markdown marks**, so what you type is always in view;
move to another line and it is drawn finished. **Pictures and formulas are drawn too**, and so are
recordings, videos and embedded notes: on the line you are writing on they show as their Markdown,
and once you move off it they are drawn. Diagrams, tables and a formula written over several lines
still show as their Markdown while you edit, and are drawn when you leave the editor.

**Formulas** are LaTeX between dollars: `$x^2$`, or `$$x^2$$`. Either can sit in a sentence and is
drawn in the line; a `$$…$$` with a line to itself is drawn larger, centred, on its own. `\*` in a
formula is a star, as it is in the rest of the note. For an arrow write `\to` (`->` is a minus and a
greater-than).

**To put a screenshot in a note, paste it.** A screenshot tool that copies to the clipboard (the
Snipping Tool, PrtScn, GNOME's or macOS's screenshot with copy) leaves the picture there; in the
note press **Ctrl+V**, or use Paste from the right-click menu. The picture is added exactly as a
dropped file is, named for when you pasted it, and drawn as soon as you move to the next line.
Dragging a file onto the note does the same.
Nothing is ever rewritten behind your back — the file is exactly the Markdown you wrote,
byte for byte.

**The usual keys format text** (⌘ instead of Ctrl on a Mac): **Ctrl+B** bold, **Ctrl+I** italic,
**Ctrl+E** code and **Ctrl+Shift+H** highlight. They do what the buttons under the note do: on
selected words they add the mark, or take it away if it is there; with nothing selected they put
the marks in with a word to type over. Bold and then italic gives both. **Ctrl+K** is not a link
here: it opens Settings, as it does everywhere in the app; the 🔗 button makes a link.
All four can be changed in **Settings → Keyboard**.

**In the editor a link works.** One press on a finished link opens it (another note opens in its
place, and Back returns; in the *Tiled* layout it opens beside this one); a double press puts the cursor in it to change it. A tick box is pressed in place.

**To link words you have already written**, select them and paste the address: they become the
link's text.

## Undo, and getting back a deleted note

The **↶** button (beside Settings, or in the bottom bar on a phone) opens a small menu:

- **Undo** names the last thing you did, for example *deleted "Lab meeting"*,
  *changed the due date of "Lab meeting"*, *moved a card to "doing"*, *edits to "Lab meeting"* or
  *created a note*. Choose it to undo that. On a computer, **Ctrl+Z** does the same and
  **Ctrl+Shift+Z** redoes. Inside a text field or a whiteboard, those keys undo your typing or
  drawing instead, as usual.
- **While you are writing in a note**, Undo takes back your typing first, a little at a time
  (*typing in "Lab meeting"*), then goes on to the things you did before. On a phone, this is how
  you undo typing.
- **Redo** puts back what you just undid.
- **Recently deleted…** lists the notes deleted in the last 30 days, on this device or another one
  that shares the notebook, even after formicaria was closed. **Bring back** restores a note
  exactly as it was.

Undo never overwrites a change made after it. If the date you want to undo was changed again since,
on this device or another, formicaria says so and leaves it as it is.

## Going back

The **Back** button undoes your last step, one at a time. On a phone that is the Back key or
gesture; on a computer it is the browser's Back button.

- **A menu or panel** that is open closes first — a note's ＋ window, the `/` menu while writing,
  Settings, Help.
- **A level inside something** goes back up one level: a Settings category to the list of them (on
  a phone), a recipe in the food sheet to the list it was chosen from, a diet or an exercise to the
  list, a theme being edited (the change is cancelled). A panel opened from another — New vault from
  Settings, the manual from Help — goes back to that panel.
- **While you edit a note**, Back stops editing, then takes you back.
- **A note you closed** stays closed: Back goes to what was before it, not into the note again.
- **A day picked from Trends** is a step: Back returns to Trends. Moving between a group's own tabs
  is not.
- **In the manual**, following its links does not add steps: Back or Done closes it.

From the first screen, Back closes the app, as usual on Android.

## Reminders

A note with a date can remind you. Open **Settings** and turn on **Remind me of events and
deadlines on this device**; the first time, your phone or browser asks whether formicaria may
show notifications. It is off until you turn it on, and it is a choice for each device.

- A note that has a **start** or a **due** date, and is not finished, is reminded of at its time.
  A date with no time is reminded of at nine in the morning of that day.
- To be told earlier, give the note the property `remind` with a number of minutes: `remind: 30`
  tells you half an hour before. `remind: off` leaves that note out.
- Finishing a note, or changing its date, changes its reminder: nothing else has to be done.

**On an Android phone** a reminder arrives with the app closed, and after the phone has been
restarted. Android may hold it back a few minutes while the phone is asleep. Reminders are set for
the next fourteen days, fifty at most, each time the app is opened or something changes — so open
the app now and then. A date that changed on another device is right once this one has got the
change.

**On a computer**, while formicaria is open in your browser, a reminder shows as a line at the
top of the app, and as a notification from the browser if you allowed it. When you close
formicaria and a reminder is still waiting, it **stays running quietly** and tells you through
your system's own notifications; once the last one has been said, it stops by itself.

- The assistant does not stay: its model is stopped when you close the app and starts again
  when you open it.
- To stop formicaria staying, turn reminders off in Settings.
- After the computer has been restarted, open formicaria once to be reminded again: it does not
  start by itself.
- A reminder that came due while the computer was asleep is said when it wakes, if that is
  within twelve hours.
- On Linux this uses the `notify-send` program, which most desktops have.

**On an iPhone or iPad** a reminder shows as a line in the app while it is open; nothing arrives
when it is closed.

This is new, and has not yet been tried on a phone, on Windows or on a Mac.

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

**You don't have to type the syntax.** The format options are **Bold · Italic · Highlight · Code ·
Colour · Link**, and a **¶ block menu** for **headings, bullet and numbered lists, and quote**.
- **With a mouse and keyboard** they are one line under the note, always there. They work where
  the cursor is: with nothing selected, a mark is opened and closed around the cursor and you
  type inside it; a block style applies to the line you are on.
- **On a phone** select some text and the same options appear below it, where the phone's own
  Cut/Copy menu leaves room.

Inline options wrap the selection in the plain-Markdown (or `[…]{.token}`) form; the block menu
re-formats the whole line or lines. Press an option again over the same text to remove it.

**On a phone or tablet, a row of six buttons sits just above the keyboard** while you write, for
the things you reach for mid-sentence:

| Button | What it does |
|---|---|
| **Dictate** (the microphone) | listens; press **Stop & write** and what you said is written where the cursor is — see [Dictating](#dictating) |
| **Record audio** | starts recording; the clip lands in the note where the cursor is |
| **Add a photo or file** | take a photo, record a video, or pick one from the library or your files |
| **Checklist item** | turns the line you are on into a tick-box line; again to turn it back |
| **Bullet** | turns the line you are on into a bullet; again to turn it back |
| **Text style** | headings, a numbered list, a quote or a callout for the line you are on |

The keyboard stays up when you press one, and the cursor stays at the end of the line so you can
keep typing. On a phone or tablet this row is where media is added: the **＋** there has no **Add
media**, since the row does it in one tap. The line styles are also on the selection toolbar. With a
mouse and keyboard there is no row: the marks are quick to type and there is no on-screen keyboard
to put it above.

### Dictating

Say it instead of typing it. On a phone, **Dictate** — the microphone above the keyboard — listens
until you press **Stop & write**, then writes what you said into the note where the cursor is, as
ordinary text you can change. It understands any language and writes it down in that language; up
to five minutes at a time.

On a computer, record it instead: the note's **＋ → Add media → Record audio**, then write
`@Myrme /transcribe` in its discussion. The recording stays in the note, and the words come back as
a proposal you accept or change.

It uses the speech-to-text of the [study assistant](./assistant.md#the-two-extras), on this device —
nothing you say leaves it — so *Audio transcription* has to be on; until it is, Dictate says so.
On a computer it takes a few seconds per minute spoken; on a phone longer. **What was said is never
lost**: if it cannot be written down, the recording is kept in the note instead, where
`@Myrme /transcribe` can read it later. For more exact words, turn on *Better listening*.

On top of standard Markdown there are three light additions, each chosen so the raw text stays
readable in **any** Markdown editor (nothing here is app-only HTML):

- **Highlight** — `==important==` renders as a highlight. Elsewhere it reads as `==important==`.
- **Callouts** — a quote whose first line is `[!type]` becomes a coloured callout
  (GitHub/Obsidian style). Make a quote from the format options, then type `[!note]` after its
  `>`; pressing a callout's label in the reading view changes its kind. The types are `note`, `tip`, `info`, `warning`, `danger`, `quote`:

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

**Editing tables.** In the read view, **tap a table cell** to edit it in place — a little box opens
over the cell, prefilled with its text; Enter or tap-away saves, Escape cancels. It rewrites only
that one cell (a typed `|` is escaped so the table can't break), and only works on plain
pipe-bordered tables — an unusual cell is simply left to the full editor. It's far kinder than
lining up pipes by hand, especially on a phone.

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
`https://openstreetmap.org/…` links work too and open in a browser. Put both in a note and a saved
spot is one tap from directions. An in-app
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

**Click a chip** and that note opens; **Back** returns to the one you came from. In the
*Tiled* layout it opens in a pane to the right instead, with the note you came from still on
screen, so a chain you follow stays visible; closing a pane there also closes everything to
its right (you reached those *through* it).

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

**Every property is editable in the app.** Open a note's **＋ options** window and pick
**Details**: below the fields formicaria knows about (status, dates, title, tags) you get a row for
every other property the note carries — `project`, `author`, whatever you have added — and an
**Add** row for a new one. Leaving a value empty removes that property.

A few properties are shown but not editable, greyed out: those are ones formicaria sets itself to
keep track of where a note belongs, and typing into them would strand the note.

A value keeps its kind. Typing `2017` stores a number, so a board grouped by it sorts sensibly;
typing `007123` stores exactly that text, because a code with leading zeros is not the number
7123. You get whatever you would have got by typing it into the file yourself.

## Papers

If you read research papers, formicaria will keep them as ordinary notes.

Press **＋ → New paper** and paste whatever you have: a citation copied from a publisher's page (a
BibTeX entry), a DOI, an arXiv link, or just the title. Formicaria fills in what it can — title,
authors, year, journal — and makes a note tagged `paper` that you can then annotate, schedule, put
on a board and search like anything else.

**Nothing is looked up online**, ever. Everything comes from what you pasted, or from the paper
itself: when you attach a PDF, formicaria reads the first page for a DOI or arXiv number and
records it without asking.

To cite one, open its **＋ options** and pick **Copy as BibTeX**.

To collect them, make a view: write `<vault>/views/Papers.view` with `view: board` and a
`- tag: paper` filter. [Views](./views.md#writing-a-view) has the format; there is no screen for
authoring one yet.

*(Reading and highlighting a PDF inside formicaria is not built yet — for now a PDF opens in your
browser's own viewer, and a link can point at a page of it.)*
