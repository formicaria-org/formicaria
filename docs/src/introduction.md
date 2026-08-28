# formicaria

formicaria is a **local-first research notebook — single-user by default, shareable
per vault**. Your notes are plain Markdown files with YAML frontmatter — one note per
file — that you own, search, organize, and back up. Heavy media (images, PDFs, video)
are stored as content-addressed blobs and referenced from notes.

The name is the plural of *formicarium*, one colony's nest, and it is the architecture:
a **set** of vaults, each its own git repo and its own audience. Which people can see a
note is decided by which repo holds it — not by a permission field, and not by us.

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
- **Shared without a service.** A vault is a git repo, so sharing one is adding a
  remote — no account, no server of ours in the middle. Two people editing different
  paragraphs of one note merge cleanly, because a `.md` merge driver resolves the
  frontmatter structurally and leaves any genuine disagreement in the note's *body*,
  where it still opens and still reads. Not real-time: minutes apart, not cursors.
- **Built to last and to be maintained.** A small Rust core behind a compile-time
  seam, a generic renderer model, and no plugin API to rot.

## Where to go next

**If you have just downloaded formicaria**, read these three, in order. They are all you need to
be using it properly:

1. [Starting formicaria](./user/running.md) — unpack it, double-click, and know where your notes
   are kept.
2. [Creating & organizing notes](./user/notes.md) — writing, linking, tagging, and getting things
   back out.
3. [Views](./user/views.md) — boards, agendas and panes: the same notes, arranged the way the
   question you are asking needs them.

After that, read whichever answers a question you actually have:
[Assets & media](./user/assets.md), [Backup & versioning](./user/backup.md),
[Collaboration](./user/collaboration.md), [The study assistant](./user/assistant.md), or
[The vault format](./user/vault.md) if you already have a folder of notes you want formicaria to
open where it stands.

**If you are here to work on formicaria itself**, the Developer guide and Reference sections below
are for you, and the rest of this section is not: start at [Architecture](./dev/architecture.md),
then [How to add a feature](./dev/adding-features.md). The canonical design spec is
`formicaria/MASTERPLAN.md` in the repository — it ships with the source, not with the app — and
this manual complements it rather than duplicating it.
