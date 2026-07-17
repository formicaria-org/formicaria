# Assets & media

Assets — images, PDFs, video, audio, any file — are stored once as
**content-addressed blobs** (keyed by the SHA-256 of their bytes) and referenced
from notes. Identical files are stored once; the hash *is* the identity.

## Adding an asset while writing

In a note's editor you have two fast paths:

- **Drag a file in.** Drop it onto the editor; formicaria ingests it (hashing,
  MIME sniffing, text extraction, thumbnail) and inserts a reference at your
  cursor.
- **Type `/`.** A menu opens at your cursor listing your **most recent notes**;
  keep typing to search by name (and extracted text) across everything. The
  search covers **notes and assets together** and labels each with its type — see
  [Linking notes](./notes.md#linking-notes-together). Assets appear once you type
  a query, which is also the only place they show up: they get no card of their
  own in the Board, Agenda or Timeline.

Both insert a Markdown image reference like:

```markdown
![my figure](asset:sha256-<hash>)
```

You can also ingest from the command line: `fm add path/to/file.pdf`.

## How media renders

In the read view, a referenced asset renders with the native browser element for
its type:

- **Images** — inline.
- **PDFs** — a scrollable, multi-page inline viewer (the browser's own).
- **Video / audio** — with native playback controls.
- **Anything else** — a labelled link to open it.

If the blob isn't present (e.g. a fresh clone before the media has synced), the
reference degrades to a small **"not available"** placeholder — media absence is
a warning, never a broken page. The extracted text of a PDF travels with the
note (in git), so a paper is searchable even before its blob arrives.

## Where blobs live

Blobs are written under `vault/blobs/` and are **git-ignored** — they sync
out-of-band (backup / a file-sync tool), keeping the notes repository small and
clonable. Thumbnails are regenerated under `vault/derived/`.
