# Roadmap — note-taking & meeting organization

The owner's ask (2026-07-15): *"improve my experience of note taking and
meetings… I am currently manually checking my emails and then setting them here.
How could I sync another calendar with my knowledge?"* — plus embedding the
whiteboard in a note and exporting one PDF.

This is the **plan** layer: what we intend and why, sequenced. When a line here
ships, delete it and fold the outcome into `overview.md`/`decisions.md`. Unlike
the other files in this folder, this one describes things that do **not** exist.

_Last updated: 2026-07-15._

## The through-line

The owner retypes meeting details out of email by hand. Most of what gets
retyped **is a calendar invite** — so calendar import removes the bulk of the
"email problem" without ever touching an inbox. Sequence follows from that.

## Status

| # | Work | State |
|---|------|-------|
| 1 | **Time on `start`/`due`** | **DONE** — see `decisions.md`, `sessions/2026-07-15-time-on-dates.md` |
| 2 | Calendar import (ICS → notes) | next |
| 3 | Calendar export (notes → ICS) | with #2; cheap |
| 4 | Whiteboard embed + print-to-PDF | independent track |
| 5 | Source module + local extraction | design phase |
| 6 | The gathered rest | unsequenced |

## 2. Calendar import — ICS → notes (one-way)

**Decided:** both the NUS Microsoft 365 calendar and Google Calendar, merged into
one vault, tagged by source.

- `fm ics pull <url|path>` → one note per `VEVENT`. `DTSTART`/`DTEND` become
  `start`/`due` stamps (#1 is what makes this worth anything — a meeting rounded
  to a whole day is useless). `SUMMARY` → title, `LOCATION`/`ORGANIZER`/attendees
  → frontmatter, `tags: [meeting, <source>]`, body left **empty for the owner to
  type into**. The note exists *before* the meeting starts; that is the feature.
- **Idempotence is the whole trick.** Key each note to the event's `UID`, and
  store `ics_uid` + `ics_seq` in frontmatter. Re-running matches by `UID`:
  update times when `SEQUENCE` bumps, and **never touch the body** the owner
  wrote. A re-pull must not be able to destroy notes. `RECURRENCE-ID` identifies
  a single occurrence of a series (decide: one note per occurrence).
- **Fetching:** `fm-serve` is std-only and has no TLS. Follow the existing
  precedent — ingest shells out to `pdftotext`, so shell out to `curl`. Add
  `curl` to pixi; **zero new Rust deps**. An ICS parser is small (line-folded
  `KEY;PARAM:VALUE`, ~200 lines, no crate needed); mind RFC 5545 line unfolding
  and `DTSTART;VALUE=DATE` (all-day → a `Stamp` with no time, which #1 models
  exactly).
- **Timezones are the sharp edge.** ICS carries `TZID`/UTC; `Stamp` is naive.
  Convert to the vault's local wall clock at import and write the naive result.
- **Risk:** NUS may not permit calendar publishing. Fallback: a manually exported
  `.ics` dropped in a watched folder — same parser, no network.

## 3. Calendar export — notes → ICS (one-way, the other way)

`GET /calendar.ics` from `fm-serve` (and/or a written file), emitting every note
with a `start`/`due`, so Outlook/Google can **subscribe** and show deadlines next
to meetings. No parser, just a formatter (~100 lines). Cheap enough to ship
alongside #2.

**Two-way write-back is rejected for now** — Graph/CalDAV + OAuth + a token store
+ conflict resolution, for maybe 10% more value than #2+#3. Revisit only if
import proves itself and the owner actually wants to create events from here.

## 4. Whiteboard in a note + one PDF

- **Embed by reference, not inline.** A regular note embeds a board note
  (`![[<board-id>]]`); `render.ts` resolves it and draws a static SVG via
  Excalidraw's `exportToSvg`, click-to-edit. **Rejected:** inlining the scene
  JSON in a fenced block — it bloats the `.md`, makes every git diff unreadable,
  and can't be reused across notes. Embedding by reference matches how assets
  already embed, keeps one-file-per-atom, and matches Obsidian's Excalidraw
  plugin. A `/draw` slash entry creates the board and inserts the embed.
- **PDF = `window.print()` + an `@media print` stylesheet.** The browser's "Save
  as PDF" is the engine: zero new deps, and math/images/drawings all print
  because the read view is already static HTML/SVG. Caveat: it opens the print
  dialog rather than writing a file silently — a headless `fm export --pdf` would
  need a real PDF engine and is a much bigger fork. Take the print route first.
  Later: print a whole view (a week, a tag) as one document.

## 5. Source module + local extraction (design phase)

The owner's framing: *"a lightweight module that can latch feed from different
sources. I can then train small models to just do this job or use some
lightweight model."*

This is **not** a plugin API (see `decisions.md` — those rot). It's the sanctioned
extension seam: a `Source` trait beside the existing `Store` trait, internal and
recompiled, each source yielding normalized event/note candidates. ICS (#2) is
the first implementation and should be written so it *is* one.

Two rules to pin before any model is involved:

- **The model proposes, the file system disposes.** Extraction lands as a draft
  the owner confirms — never a silent write. A hallucinated date must not be able
  to enter the vault. This is the files-as-truth guarantee applied to inference.
- **Local and optional.** No cloud inference, no account, degrade cleanly when
  the model is absent. The `media` pixi feature (ffmpeg, tesseract) is the
  precedent for isolating heavy extras.

Cheapest real email step, independent of any model: **drag an `.eml` onto a
note** — ingest already sniffs MIME and extracts text, so an `.eml` extractor
(headers → frontmatter, body → note, attachments → blobs) needs no auth and no
network. A watched maildir is better long-term but M365 increasingly forces OAuth
on IMAP, which drags in exactly the auth problem #2/#3 were designed to dodge.

## 6. The gathered rest (unsequenced)

Roughly by value-per-effort:

- **Meeting template** — a calendar-imported note opens with attendees/agenda/
  decisions/actions ready to type into. Pairs directly with #2.
- **Backlinks** — "what links *here*". The forward half **shipped 2026-07-16** as
  `[Title](note:<ulid>)` references + the sliding-pane trail (see
  [sessions/2026-07-16-note-references-sliding-panes.md](./sessions/2026-07-16-note-references-sliding-panes.md));
  note that we **did not build `[[wikilinks]]`** and the syntax is settled — that
  entry explains why. What's left is the reverse index: scan bodies on reindex, or
  a `links` table in `index.sqlite`. It's the rest of what makes notes *knowledge*.
- **Time-of-day week view** — unlocked by #1; the calendar is still day-granular.
- **"Open today"** — the Timeline exists but nothing jumps to now.
- **Audio capture + transcription** — the `media` env already has ffmpeg; Whisper
  is on conda-forge. Fits #5's "local, optional" rule.
- **Attendees as notes** — people become first-class, linkable.

### The one open tension: inline actions

In a meeting you write `- [ ] Ravi to send the draft` inline. Making those
first-class agenda items needs **per-block identity**, which `decisions.md`
explicitly rejects — *the atom is the file*. So either actions become their own
note files (clean, but friction mid-meeting), or the agenda gains a second-class
"scan bodies for checkboxes" pass that doesn't round-trip. **Undecided — the
owner should rule on this deliberately, not discover it later.**
