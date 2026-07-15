# Overview — read this first

A compact, high-density snapshot of the repo, meant to bootstrap a working
mental model **without** reading the whole codebase. When this disagrees with
the code, the code wins — fix this file.

_Last verified: 2026-07-15 (commit `e57793f`)._

## What formicarium is

A **local-first, single-user research notebook / PKM**. Guiding principle:
**files-as-truth** — a note *is* a Markdown file with YAML frontmatter, one note
per file, that the user owns. In the worst case the notes survive as plain text
on disk/GitHub, readable without this app. Heavy media (images, PDFs, video) are
stored as **content-addressed blobs** (sha256) and referenced from notes.

The atom is the **file**, never the block — there are no per-block ids or
timestamps. This is deliberate and load-bearing.

## How it runs (the product)

**`pixi run serve`** →
[`fm-serve`](../../crates/fm-serve) (a tiny **std-only** HTTP server,
thread-per-connection, `127.0.0.1:8765`) builds + serves `ui/dist` and fronts
the `fm_app::commands` over `POST /api/<cmd>` (JSON, or raw bytes for
`resolve_asset`/`ingest`), then opens the default browser.

There is **no native window** and no cloud/account. The browser is the product
(see [decisions.md](./decisions.md) for why the Tauri window was removed).

`ui/src/lib/ipc.ts` has two backends: `import.meta.env.PROD` → HTTP;
otherwise the in-memory `mock.ts` (used by `pnpm dev` and Vitest).

## Architecture — three compile-time seams

```
fm-model   data types (Object, Kind, PropertyValue, frontmatter)
fm-query   PURE query engine — NO std::fs, NO db crate, NO paths  ← seam 1
fm-core    FileStore + BlobStore + ingest/verify/backup/git       ← seam 2 (Store trait)
fm-app     command library (commands.rs + dto.rs) over the Store  ← seam 3 (UI ⇄ core)
fm-serve   std-only HTTP server; fronts fm-app commands over /api
fm-cli     the `fm` binary (add/verify/manifest/backup/restore/check/set/show)
ui/        Svelte 5 + Vite; renderers are generic + literal-free
```

- **Seam 1 (the insurance policy):** `fm-query` must never depend on
  `rusqlite`/`std::fs`/paths. Enforced at compile time *and* by a CI grep
  (`ci/checks.sh`). The pure engine is what keeps the app testable and portable.
- **Seam 2:** everything is written against the `Store` trait; `FileStore` and
  `MemoryStore` stay behavior-equivalent (tested).
- **Seam 3:** the UI only knows `POST /api/<cmd>`. Adding a command touches
  **4 places** — see [adding a feature](../src/dev/adding-features.md).

The SQLite index (FTS5) is **disposable**, rebuilt from the files on open. Git
versioning of the notes + restic backup provide durability.

## The 15 API commands

`board` · `gallery` · `agenda` · `get` · `search` · `recent` · `capture` ·
`set_property` · `update_body` · `asset_status` · `resolve_asset` ·
`open_external` · `commit` · `backup` · `ingest`

## Views (all generic renderers over the same query layer)

- **Board** — kanban, group-by-*any*-property, drag write-back to disk
- **Agenda** — Calendar (month/week) + a List grouped under urgency bands
- **Timeline** — Logseq-style journal by creation day
- **Gallery** — asset thumbnails
- **Search** — FTS5 (also indexes extracted PDF text)

Renderers must contain **no status literals** (`todo/doing/done`) and no
scheduling literals — value/urgency styling is keyed by data attributes in the
theme, and labels live in helpers (`urgency.ts`). CI greps enforce this.

## Toolchain

Everything is pinned in **pixi** (conda-forge). `rust/node/pnpm/poppler/libvips/
restic/mdbook` are NOT on the base PATH — run via `pixi run …`. Key tasks:
`serve`, `test`, `test-ui`, `deny`, `checks`, `docs`, and **`ci`** (runs
test + test-ui + deny + checks + docs; the single CI gate).

## Current status (2026-07-15)

Browser-first **v2 is implemented and green** (`pixi run ci` exit 0, prod build,
live serve smoke). Everything in "Views" above works, plus: note read view
(marked→HTML, lazy KaTeX/Mermaid), a properties editor + body edit (byte
round-trip), asset ingest + drag-drop + `/` slash-insert + inline media
(img/PDF/video/audio), one-click restic backup (tested restore),
verify/manifest (bit-rot), debounced git auto-commit, a design-token system
(dark+light), the sidebar shell, the side-sheet NotePanel, a ⌘K command palette,
an mdBook manual, and a `.desktop` launcher.

See [known-issues.md](./known-issues.md) for what is **not** working / deferred,
and [sessions/](./sessions/) for how we got here.
