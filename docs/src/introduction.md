# formicarium

formicarium is a **local-first, single-user research notebook**. Your notes are
plain Markdown files with YAML frontmatter — one note per file — that you own,
search, organize, and back up. Heavy media (images, PDFs, video) are stored as
content-addressed blobs and referenced from notes.

It runs as a **local web app**: a tiny server (`fm-serve`) builds and serves the
UI and answers a small HTTP API over your vault, which it opens in your default
browser. There is no cloud, no account, and no native window to fight with.

## Why it exists

- **Files-as-truth.** A note *is* a Markdown file; in the worst case your notes
  survive as plain text on GitHub, readable without this app.
- **Fast capture, reliable retrieval.** Full-text search (SQLite FTS5) indexes
  every note — and even the extracted text of ingested PDFs.
- **Durable.** A disposable per-machine index rebuilt from the files; git
  versioning of the notes; restic backup with a tested restore.
- **Built to last and to be maintained.** A small Rust core behind a compile-time
  seam, a generic renderer model, and no plugin API to rot.

The canonical design spec is `formicarium/MASTERPLAN.md` in the repository; this
manual is the practical user + developer guide and complements the spec rather
than duplicating it.

## Where to go next

- New here? Start with [Running formicarium](./user/running.md).
- Want to extend it? Read [Architecture](./dev/architecture.md) then
  [How to add a feature](./dev/adding-features.md).
