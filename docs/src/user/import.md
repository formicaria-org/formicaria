# Bringing notes in from another app

If you already keep notes in **Logseq** or **Obsidian**, formicaria can read that folder and turn
it into notes of its own.

> **Your original notes are never touched.** The folder you point at is only ever *read* — nothing
> in it is changed, moved or deleted. What you get is a copy, in formicaria's own format. The two
> do not stay in step afterwards: this is a move, not a sync.

## Doing it

1. Open **Back up** and choose **New vault…**.
2. Pick **Import from another app**.
3. Type the folder your graph or vault lives in — for example `~/Documents/my-logseq-graph`.
4. formicaria looks at it and tells you what it found: how many pages, how many daily notes, how
   many attachments. If the folder isn't a Logseq graph or an Obsidian vault, it says so and the
   button stays off. If you have **imported this folder before**, it tells you which vault holds it
   and offers to update that one instead of making a second copy.
5. Choose where the notes go — **a new vault** (the default) or one you already have.
6. Choose which **attachments** come over (see below). All of them, unless you say otherwise.
7. Press **Import notes**.

Afterwards you get a summary: how many notes arrived, how many links now work, and — just as
importantly — anything that didn't come across cleanly.

**A new vault is the easy thing to undo.** If you don't like the result, forget the vault and
nothing else in formicaria changes. Importing into an existing vault is recorded as a single entry
in that vault's history, so it can be undone in one step too.

## What comes across

| In Logseq / Obsidian | In formicaria |
|---|---|
| A page | A note — one page, one file |
| `[[Another Page]]` | A real link you can click through — by the page's file name, its `title::`, an `alias::`, or a day like `[[Oct 14th, 2026]]` |
| `[[A page nobody wrote]]` (an author, `[[journalArticle]]`, …) | A **tag** on the note, so you can still find everything that names it |
| `#tag`, `#[[two words]]` | A tag on the note, so you can filter and group by it |
| An image, PDF, slide deck, notebook or any other linked file | Stored with your other attachments, and searchable if it has text |
| A highlight in a PDF (`hl-page::`) | The highlight, with a link that opens the PDF at that page |
| A YouTube or other `{{video}}` | A link to the video |
| A Zotero attachment (`{{zotero-imported-file}}`) | The file, if it is in the graph; otherwise its name, saying it is kept in Zotero |
| A drawio drawing (`{{renderer :drawio}}`) | The drawing, as an image |
| An Excalidraw drawing (`draws/`) | A whiteboard note |
| A daily note / journal | A note dated the day it was written, tagged `journal` |
| `TODO` / `DONE` items | Checkboxes you can tick |
| Page properties (`key:: value`, or frontmatter) | Note properties you can sort and group by |
| A Logseq block reference | The words that block actually said, with its links and images working |

Your Logseq outline stays an outline — the bullets and their nesting come across as they are.

## What doesn't, and why

- **Pages that were never written do not become notes.** In Logseq a page exists the moment you
  link to it, so a graph usually has a lot of these — authors, kinds of paper, section names. Each
  becomes a **tag** on the note that links to it instead, so searching or filtering by it still
  finds every note that named it. (A link to a day with no daily note stays plain words.) If you'd
  rather have a note for each one, tick *"Make a note for pages that are only linked to"* — but be
  aware every one of them will then show up in your board, calendar and timeline.
- **Individual bullets are not separately addressable.** In formicaria a note is the smallest
  thing that has an identity — that's a deliberate design decision, and it's what keeps your notes
  readable as plain files. So a `TODO` bullet becomes a checkbox in the note rather than an item
  in your calendar. To schedule something, make it its own note.
- **Logseq whiteboards, Canvas files and plugin data.** These are specific to the app that made
  them and have nowhere to go here. The preview counts them so you know what's staying behind.
  (Excalidraw drawings are the exception: they become whiteboard notes.)
- **Queries.** A `{{query …}}` is shown as the query it was, not run.
- **Org-mode graphs.** Markdown only.

## Attachments: all of them, or not

By default every file a note links to comes over. You can choose **Leave some out** instead, and
then leave out files **larger than** a size you pick, and/or whole **kinds** of file — the form lists
each kind with how many there are and how much space they take, so you can see that, say, your
robot recordings are most of the folder. Anything left out is still **named** in the note that
mentions it, with the reason, so nothing disappears silently.

**Disk space.** Every attachment that comes over is **copied** into the vault — a vault holds all of
its own files, so it never depends on the folder you imported from, and that folder stays exactly
as it was. The preview shows how much space the attachments take, kind by kind, before you press
the button; leaving out large files or whole kinds is how you spend less of it.

## Doing it again later

Import the same folder into the same vault again and formicaria adds what it hasn't seen before.
What it brought across last time is left as it is — unless you tick **Replace what an earlier import
of this folder wrote**. Then those notes are brought up to date from the folder (useful when
formicaria's importer has improved), with one firm rule: **a note you have changed or deleted since
is kept exactly as you left it**. The summary names any such notes. The whole replace is one entry
in the vault's history, so it can be undone in one step.

If you start an import of a folder that is already in one of your vaults, the preview tells you and
offers **Update that vault instead**, so you don't end up with two copies by accident.

## A note on size

A few thousand notes is fine. A very large graph — tens of thousands of pages — will make lists
slower to draw, and the app will be busy while it works. The preview tells you the size before you
commit to it.

Importing reads a folder on the computer running formicaria, so it isn't available from a phone or
from a paired tablet.
