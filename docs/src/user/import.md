# Bringing notes in from another app

If you already keep notes in **Logseq** or **Obsidian**, formicaria can read that folder and turn
it into notes of its own.

> **Your original notes are never touched.** The folder you point at is only ever *read* — nothing
> in it is changed, moved or deleted. What you get is a copy, in formicaria's own format. The two
> do not stay in step afterwards: this is a move, not a sync.

## Doing it

1. Open **Settings** and choose **New vault** (or **New vault…** in the Back up panel).
2. Pick **Import from another app**.
3. Type the folder your graph or vault lives in — for example `~/Documents/my-logseq-graph`.
4. formicaria looks at it and tells you what it found: how many pages, how many daily notes, how
   many attachments. If the folder isn't a Logseq graph or an Obsidian vault, it says so and the
   button stays off.
5. Choose where the notes go — **a new vault** (the default) or one you already have.
6. Press **Import notes**.

Afterwards you get a summary: how many notes arrived, how many links now work, and — just as
importantly — anything that didn't come across cleanly.

**A new vault is the easy thing to undo.** If you don't like the result, forget the vault and
nothing else in formicaria changes. Importing into an existing vault is recorded as a single entry
in that vault's history, so it can be undone in one step too.

## What comes across

| In Logseq / Obsidian | In formicaria |
|---|---|
| A page | A note — one page, one file |
| `[[Another Page]]` | A real link you can click through |
| `#tag`, `#[[two words]]` | A tag on the note, so you can filter and group by it |
| An image or PDF | Stored with your other attachments, and searchable if it has text |
| A daily note / journal | A note dated the day it was written, tagged `journal` |
| `TODO` / `DONE` items | Checkboxes you can tick |
| Page properties (`key:: value`, or frontmatter) | Note properties you can sort and group by |
| A Logseq block reference | The words that block actually said |

Your Logseq outline stays an outline — the bullets and their nesting come across as they are.

## What doesn't, and why

- **Links to pages that were never written.** In Logseq a page exists the moment you link to it,
  so a graph usually has a lot of these. They stay as plain text, and the summary lists them. If
  you'd rather have a note for each one, tick *"Make a note for pages that are only linked to"* —
  but be aware every one of them will then show up in your board, calendar and timeline.
- **Individual bullets are not separately addressable.** In formicaria a note is the smallest
  thing that has an identity — that's a deliberate design decision, and it's what keeps your notes
  readable as plain files. So a `TODO` bullet becomes a checkbox in the note rather than an item
  in your calendar. To schedule something, make it its own note.
- **Whiteboards, Canvas files and plugin data.** These are specific to the app that made them and
  have nowhere to go here. The preview counts them so you know what's staying behind.
- **Org-mode graphs.** Markdown only.

## Doing it again later

Import the same folder again and formicaria adds only what it hasn't seen before. Notes it already
brought across are **left exactly as they are** — including any edits you've made to them here. It
will never overwrite your work with an older version from the other app.

## A note on size

A few thousand notes is fine. A very large graph — tens of thousands of pages — will make lists
slower to draw, and the app will be busy while it works. The preview tells you the size before you
commit to it.

Importing reads a folder on the computer running formicaria, so it isn't available from a phone or
from a paired tablet.
