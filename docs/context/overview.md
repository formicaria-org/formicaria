# Overview — read this first

A compact, high-density snapshot of the repo, meant to bootstrap a working
mental model **without** reading the whole codebase. When this disagrees with
the code, the code wins — fix this file.

_Last verified: 2026-07-17 — **backup is now two tiers** (`BackupPanel.svelte`):
the button opens a panel that sets the vault's **git remote in-app** and pushes
the **notes** by default (no app-held secret — ambient ssh/credential-helper),
squashing the unpushed window into one `backup:` commit; **restic (media
included) is an opt-in checkbox**. The panel states what each tier does and does
**not** carry, and reports whether the data actually **left the machine**
(`destination.ts` — a local path is a legitimate destination but must never be
called off-site). Before that, a **double-click in the read view opens the editor
with the caret on the word you clicked** (`locate.ts` maps by word *ordinal*, not
by source positions — see the session note for why). Before that,
**five UX fixes**: assets are
filtered out of `board`/`agenda`/`recent` **in the query layer** (`search`/`gallery`
still see them — that is how you find a PDF, and how `/` inserts one); a
**StatusChip** rotates status through the vault's own values from a card or the
open note; a dropped board card **keeps its position** in the column
(`fm-card-order` in localStorage, like column order); the `/` menu opens **at the
caret** and seeds with **recent notes**; and a note opens its editor on
**double-click** as well as from the **Edit button** (which stays on every note —
the gesture is a shortcut, not a replacement), with **Ctrl+S** to save. The
double-click now **lands the caret on the word you clicked** (`locate.ts`). Before
that: **notes reference notes**: `[Title](note:<ulid>)`
(deliberately *not* `[[wikilinks]]` — see the session note), inserted by the same
`/` menu as assets, rendered as a live title+status chip, and clicking one opens
the target as a **pane to the right** so the trail you followed stays on screen
(`openIds: string[]`; the overlay/backdrop moved from NotePanel up to App).
Backlinks are still not built. Before that: notes+tags (no user "type"), optional settable
`start`+`due` **stamps that now carry an optional time** (`2026-07-20T14:30`), so
a meeting is expressible; calendar bars start→due, note panel full-screen
by default, red brand accent matching the app icon, launcher UX (release icon,
reopen, close-tab-quits), and **board notes** (freeform Excalidraw whiteboard,
lazy-loaded) — now **treated exactly like notes** (a "Details" props editor over
the canvas → agenda/calendar/board-trackable). **Sidebar is search-first**: the
persistent Search field is the primary input, with New note / New board buttons
below (quick-capture box removed). Gallery removed. On top of note-delete, media
copy-notice, column reorder, SVG fix. Next work is planned in
[roadmap.md](./roadmap.md) (calendar sync, whiteboard-in-note + PDF)._

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

## The 20 API commands

`board` · `gallery` · `agenda` · `get` · `search` · `recent` · `capture` ·
`set_property` · `update_body` · `delete` · `asset_status` · `resolve_asset` ·
`open_external` · `commit` · `push` · `backup` · `backup_status` ·
`set_git_remote` · `ingest` · `ping` (liveness heartbeat → auto-shutdown)

## Views (all generic renderers over the same query layer)

- **Board** — kanban, group-by-*any*-property, drag a card to another column to
  set that property (status) **and to a chosen position within it** (the drop
  index is self-measured from the card rects in `Board.svelte`); drag column
  headers to reorder. Both orders persist per group-by in `localStorage`
  (`fm-board-order` / `fm-card-order`) — view preferences, not vault data.
- **Agenda** — Calendar (month/week) + a List grouped under urgency bands. A note
  carries an optional **`start`** and **`due`** *stamp* — a day plus an **optional
  wall-clock time** (`2026-07-20` or `2026-07-20T14:30`; both unset by default,
  both user-settable). The calendar draws a bar from `start`→`due`, or a
  single-day marker on `due` when there's no start (creation date is never assumed
  as a start). Bars split into flat-ended segments across week rows; pure helpers
  (`clampRangeToWeek`/`assignLanes` in `calendar.ts`) do the geometry, and a bar
  with times on both ends is labelled with its window (`14:30–15:00`).
  **The grid and the urgency bands are day-granular**: a time is presentation, not
  priority. `stamp.ts` (`parseStamp`/`dayOf`/`toStamp`/`formatStamp`) is the only
  UI module that knows the wire format — see [decisions.md](./decisions.md) for
  why a `Stamp` and not a `Date`/`DateTime` pair.
- **Timeline** — Logseq-style journal by creation day
- **Search** — FTS5 (also indexes extracted PDF text)
- **Board notes (whiteboard)** — a note with `view: board` whose body is an
  **Excalidraw** scene (JSON). `NotePanel` renders the canvas (`Whiteboard.svelte`,
  which lazily imports Excalidraw/React — a separate ~744 KB-gz chunk, loaded only
  when a board opens). "New board" is both a command-palette entry and a sidebar
  button. No new `Kind`, no backend change — files-as-truth via the `.excalidraw`
  JSON. A board is **treated exactly like a note**: its header Edit button opens a
  **"Details"** panel with the same props editor (Status/Start/Due/Hard/Title/Tags)
  above the live canvas, so a dated board shows up in Agenda/Calendar and a
  statused board groups on the Board — it's a first-class note that happens to draw.

There is **no user-facing note "type"** (meeting/task/note): everything is a
**note**, differentiated by **tags**. The only surviving `Kind` distinction is
**asset** (a note cataloging an ingested blob); `Kind::from_str` is lenient so
legacy `type: task`/`meeting` frontmatter migrates silently to `note` on load.
The standalone **Gallery view was removed** — assets open from the notes that
reference them (the `gallery` command still exists server-side but is unused by
the UI).

**Assets appear in no view.** `board`/`agenda`/`recent` filter to
`Predicate::Kind(vec![Kind::Note])` — an asset is a blob a note *references*, not
something you plan. `search` and `gallery` still see assets deliberately: search
is how you find a PDF (its extracted text is indexed), and it is what backs the
`/` menu's asset insertion. Because grouping is generic, a board grouped by `type`
can now only answer `note` — that is the pinned behavior, not a bug.

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

## Current status (2026-07-17)

Browser-first **v2 is implemented and green** (`pixi run ci` exit 0, prod build,
live serve smoke). Everything in "Views" above works, plus: note read view
(marked→HTML, lazy KaTeX/Mermaid) that you edit via the header **Edit** button or
by **double-clicking** it — which opens the editor **with the caret on the word
you clicked**, scrolled into view (**Ctrl+S** saves and returns to reading, Escape
leaves the editor, and the view is focusable so Enter is the keyboard equivalent,
opening at the top; `.read` is `flex: 1 0 auto` so the gesture works in the
whitespace under a short note, not just on the text — which lands the caret at the
**end**, i.e. append), a properties editor +
body edit (byte round-trip), a **click-to-rotate status chip** on cards and in the
note header (`status.ts`'s `nextStatus` over the vault's own values — no literal),
asset ingest + drag-drop + inline media
(img/PDF/video/audio, incl. SVG via a content sniff), **note→note references**
(`[Title](note:<ulid>)` → a title+status chip → opens a pane in the trail),
a `/` slash-insert menu **anchored at the caret** (`caret.ts`, mirror-div) that
seeds with `recent` notes and searches **both notes and assets**, **two-tier backup**
(push notes by default / tick to add restic media; tested restore),
verify/manifest (bit-rot), debounced git auto-commit (**best-effort and silent —
see known-issues**), a
design-token system (dark+light), the sidebar shell, the NotePanel
(**full screen by default**, toggle to a docked side-sheet; remembered per
browser), note delete (double-confirm), a ⌘K command palette,
an mdBook manual, and a `.desktop` launcher.

See [known-issues.md](./known-issues.md) for what is **not** working / deferred,
and [sessions/](./sessions/) for how we got here.
