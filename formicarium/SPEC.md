# Formicarium — the whole plan, one page

**2026-07-14 · no code written yet**

Project: `formicarium` · Binary: **`fm`**

A formicarium is the apparatus you build so that an emergent structure becomes observable. You provide the medium and the glass; the colony digs the tunnels. Notes accumulate; views reveal the structure that formed. Don't impose the taxonomy — build the glass.

(`fm`, not `formica` — that's a countertop. And never `ant` — Apache Ant has owned that command for twenty-five years.)

## What

Local tool. Capture ideas fast, retrieve them years later, see what you're working on, keep every figure, deck, paper and recording attached to the thought that produced it.

**It can only fail two ways: capture is slow, or retrieval doesn't work.** Everything else is decoration.

## The one idea

Objects with typed properties. **Views are queries plus renderers, not features.**

- Kanban = group by `status`
- Gallery = filter `type = asset`
- Agenda = `status != done AND due <= +7d`, list renderer, **zero new code**

**The test: the board renderer must not know what `status` is.** Point it at `type` and you should get a board of notes/tasks/assets. Grep renderer source for `todo|doing|done` in CI — **fail the build if found**. Those are values in your data, not concepts in your code.

Logseq spent three years refactoring toward this. Obsidian shipped it as Bases. Both started with kanban as a plugin; both moved it to core.

## Three invariants

1. **The atom is the file, never the block.** No per-bullet IDs or timestamps — that is exactly what forced Logseq's rewrite. Want per-entry timestamps? Make entries *files*: capture creates a new small file, and file mtime is entry mtime. Free.
2. **Status is a property. Tags are tags.** A column is exclusive; a tag isn't. Conflate them and one task sits in three columns forever.
3. **The notes repo is text only.** A pre-commit hook rejecting anything over 5 MB is what makes this real rather than aspirational.

## Three seams

- **The query engine never touches the filesystem.** Storage stays swappable. This is the difference between a backend swap and Logseq's three-year rewrite.
- **Assets are blobs on their own volume, outside the repo.** Notes stay tiny, git stays fast, forever.
- **Every format problem is shelled out to the OS.** You write the catalog. Nothing else.

## Stack

| | |
|---|---|
| **Go + `modernc.org/sqlite`** | Pure Go, FTS5 included, genuinely static binary. Chosen for **10-year longevity** — a Go binary from 2026 still runs in 2036; fifteen pinned pip deps will not. |
| **Markdown + YAML frontmatter, in git, auto-commit** | Buys **undo**, not a readable history. Don't try to fix that. |
| **SQLite FTS5 over notes *and* `pdftotext` output** | **The killer feature.** Search inside every paper you've ever read, not just your notes about them. |
| **HTMX + plain `<textarea>`** | No npm, no bundler, no `node_modules`. CodeMirror would force a build step and destroy the entire reason for HTMX. |
| **sha256 content-addressed blobs** | Verifiable with `sha256sum` forever. Ingest big files with `cp --reflink=auto` — instant, zero extra bytes. |
| **restic · `xdg-open`** | Dedup, encryption and backup, free. Every OS ships a better viewer than you'd build, so **build none**. |

## Build order

**S0** type → disk → reload · **S1** search · **S2** properties · **S3** board

**S4 — gallery. This is the checkpoint.** A *second* renderer over the same query layer must cost a weekend. Anyone can build one board. **If S4 hurts, the query layer isn't real, you're building features, and you must stop there** — it only gets more expensive.

**S5** assets · **S6** `verify` + a **tested** restic restore *(an untested backup is not a backup)*

**Stop if:** 8 weekends without a working board — or two weeks without opening it.

## Cut

**Calendar** (do you actually have deadlines?) · **whiteboard** (Logseq built one and abandoned it) · **graph view** (everyone builds it, nobody opens it after week two) · **plugin API** — *never*: a plugin API lets strangers extend your software without touching your source, and **you have no strangers, you have git** · OCR · backlinks · semantic search · file watcher (below).

## Two questions block everything

**1. Do you edit notes outside the app?** (Vim, `git pull`, Syncthing)
If **no** → the app is the sole writer, it updates the index in the same transaction, there is no drift, and **the file watcher does not exist.** That removes the single largest risk in the design.

**2. Name five things in your work with real due dates.**
If you can't → **cut the calendar permanently.** It's cargo-culted from Notion.

## First three commits

1. `Store` interface + `Query` struct + `MemoryStore` + a query-engine test suite that passes with **zero filesystem access**. That test isn't checking correctness — it's the architecture defending itself against you in eight months, tired and in a hurry. Text search is just another predicate: FTS5 in `FileStore`, naive substring scan in `MemoryStore`.
2. `.githooks/pre-commit` — the 5 MB guard.
3. Perf budgets as CI assertions, *now*, while they're trivially easy to pass: capture interactive <200 ms · search <100 ms @ 10k · keystroke→paint <16 ms · reindex <10 s. A budget added after it's already violated gets deleted, not fixed.

## Before any of this

**Run SilverBullet for two weeks.** Highest-return item on the page. It's close enough to this spec that knowing precisely why it *isn't* enough would sharpen everything above — and if it turns out to be enough, you just saved six months and went back to doing Meta-RL.
