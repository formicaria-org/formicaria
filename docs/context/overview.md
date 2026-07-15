# Overview — read this first

A compact, high-density snapshot of the repo, meant to bootstrap a working
mental model **without** reading the whole codebase. When this disagrees with
the code, the code wins — fix this file.

_Last verified: 2026-07-15 (uncommitted — notes+tags (no user "type"), full-screen
note panel, multi-day calendar bars, Gallery view removed; on top of note-delete,
media copy-notice, column reorder, and the SVG inline-render fix)._

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

The **desktop icon** (`packaging/`) runs `pixi run app` — the prebuilt release
binary, no rebuild, so it starts instantly and reflects your last `pixi run
build`. The launcher sets `FM_OPEN` (open the browser) and `FM_AUTO_SHUTDOWN`
(the UI heartbeats `POST /api/ping` every 3s; when the last tab closes, the
server's watchdog exits after a ~10s idle window that still survives a reload).
So **closing the tab closes the app** — no lingering daemon. A terminal `pixi run
serve` sets neither, so it stays up until Ctrl-C.

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

## The 16 API commands

`board` · `gallery` · `agenda` · `get` · `search` · `recent` · `capture` ·
`set_property` · `update_body` · `delete` · `asset_status` · `resolve_asset` ·
`open_external` · `commit` · `backup` · `ingest`

## Views (all generic renderers over the same query layer)

- **Board** — kanban, group-by-*any*-property, drag a card to another column to
  set that property (status), drag column headers to reorder (persisted per
  group-by in `localStorage`)
- **Agenda** — Calendar (month/week) + a List grouped under urgency bands. The
  calendar draws each note as a **multi-day bar** from its creation day (start)
  to its due day (end), split into flat-ended segments across week rows; pure
  helpers (`isoDate`/`clampRangeToWeek`/`assignLanes` in `calendar.ts`) do the
  geometry.
- **Timeline** — Logseq-style journal by creation day
- **Search** — FTS5 (also indexes extracted PDF text)

There is **no user-facing note "type"** (meeting/task/note): everything is a
**note**, differentiated by **tags**. The only surviving `Kind` distinction is
**asset** (a note cataloging an ingested blob); `Kind::from_str` is lenient so
legacy `type: task`/`meeting` frontmatter migrates silently to `note` on load.
The standalone **Gallery view was removed** — assets open from the notes that
reference them (the `gallery` command still exists server-side but is unused by
the UI).

Renderers must contain **no status literals** (`todo/doing/done`) and no
scheduling literals — value/urgency styling is keyed by data attributes in the
theme, and labels live in helpers (`urgency.ts`). CI greps enforce this.

## Toolchain

Everything is pinned in **pixi** (conda-forge). `rust/node/pnpm/poppler/libvips/
restic/mdbook` are NOT on the base PATH — run via `pixi run …`. Key tasks:
`serve` (debug run), `serve-release` (optimized run), `build` / `build-debug`
(compile artifacts — `target/{release,debug}/fm-serve` + `ui/dist` — without
running), `test`, `test-ui`, `deny`, `checks`, `docs`, and **`ci`** (runs
test + test-ui + deny + checks + docs; the single CI gate). The shipped binary
uses `[profile.release]` in `Cargo.toml` (strip + thin-LTO → ~3 MB, vs ~34 MB debug).

## Current status (2026-07-15)

Browser-first **v2 is implemented and green** (`pixi run ci` exit 0, prod build,
live serve smoke). Everything in "Views" above works, plus: note read view
(marked→HTML, lazy KaTeX/Mermaid), a properties editor + body edit (byte
round-trip), asset ingest + drag-drop + `/` slash-insert + inline media
(img/PDF/video/audio, incl. SVG via a content sniff), one-click restic backup
(tested restore), verify/manifest (bit-rot), debounced git auto-commit, a
design-token system (dark+light), the sidebar shell, the side-sheet NotePanel
(**toggles to full screen**), note delete (double-confirm), a ⌘K command palette,
an mdBook manual, and a `.desktop` launcher.

See [known-issues.md](./known-issues.md) for what is **not** working / deferred,
and [sessions/](./sessions/) for how we got here.
