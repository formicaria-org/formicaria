# Overview — read this first

A compact, high-density snapshot of the repo, meant to bootstrap a working
mental model **without** reading the whole codebase. When this disagrees with
the code, the code wins — fix this file. This file plus [features.md](./features.md)
are the **always-read** layer; everything else is pulled on demand via the router below.

_Last verified 2026-07-24. The running "what shipped when" narrative that used to live here has
moved to [archive/overview-last-verified-narrative-through-2026-07-24.md](./archive/overview-last-verified-narrative-through-2026-07-24.md);
current per-feature status is [features.md](./features.md), the dated log is [sessions/](./sessions/)._

## Router — read this BEFORE you touch a hot area

The rule this project keeps re-learning: **an on-demand doc no one is pointed to is invisible.**
So before editing one of these areas, open the named file first. `decisions.md#<subject>` means
grep `decisions.md` for that subject tag — its index is at the top of the file.

| Touching… | Read first |
|---|---|
| A **compile-time seam** (`fm-query` purity, `Store`/`candidates`, `dispatch`) | the "Architecture" section below + `decisions.md#seams` |
| **git / sync / merge / proposals** (`vcs`, `git`, `git_native`, `merge.rs`) | `decisions.md#git`, `decisions.md#sync`; `sessions/2026-07-24-proposals-on-the-phone.md` |
| The **phone / Android** build (mobile shell, jniLibs, foreground service) | `mobile-design.md`; `decisions.md#track-m` |
| The **study assistant / agent** (`fm-agent*`, models, RAG) | `ai-agents-plan.md`; **`model-selection-research-2026-07-24-grounded.md`** (grounding-first, current) |
| **Audio transcription** (`transcribe`, whisper) | `audio-asr-research-2026-07-23.md`; `sessions/2026-07-24-proposals-on-the-phone.md` |
| **Model sizing / device RAM / per-component footprint** | `device-resources.md` (measured numbers, not guesses) |
| **Who may see which vault** (`Scope`, `Scoped`, the share/pairing gate, a new read path) | `decisions.md#vault` — the enforcement points are not obvious and one of them is `Vaults::config` |
| Anything, before you assume it works | `known-issues.md` (durable traps) · `outstanding.md` (the work queue) |

## What formicaria is

A **local-first research notebook / PKM**, single-user by default and shareable per
vault. Guiding principle:
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
**`fm_app::dispatch`** — the single command surface — over `POST /api/<cmd>` (JSON, or raw
bytes for `resolve_asset`/`ingest`), plus two routes that are deliberately *not* commands
(`GET /api/blob/<reference>`, `POST /api/alive`), then opens the default browser.

**`127.0.0.1`, plus a second listener if you share it.** Loopback stays plain HTTP and exactly as
it was — the desktop's browser, `fm-cli`, curl and the study agent all arrive there. Turning on
*Settings → Share with another device* adds a **separate** listener on the next port, over TLS
(a self-signed leaf whose fingerprint the user compares by hand; `tls` feature, default on), so a
tablet on the same network can open the real app (the UI is entirely origin-relative — nothing is
streamed). A non-loopback caller is a different kind of
caller: it must present a token from a pairing code, it reaches only the **vaults that token was
paired for** (`fm_app::Scope` → `fm_core::Scoped`), and host-bound commands are refused outright.
Loopback keeps exactly its old behaviour, which is why `fm-cli`, curl and the study agent needed
no changes. See `decisions.md#vault`.

There is **no native window** and no cloud/account. The browser is the product
(see [decisions.md](./decisions.md) for why the Tauri window was removed).

The **desktop icon** (`packaging/`) runs `pixi run app` — the prebuilt release
binary, no rebuild, so it starts instantly and reflects your last `pixi run
build`. The launcher sets `FM_OPEN` (open the browser) and `FM_AUTO_SHUTDOWN`
(the UI heartbeats `POST /api/alive` every 15s; when the last tab closes, the
server's watchdog exits after a ~90s idle window. The window is sized against a
*throttled* beat, not a nominal one: browsers throttle a hidden tab's timers to about
once a minute, and the old 3s-beat/10s-window pairing killed the app out from under
anyone who left it in a background tab).
So **closing the tab closes the app** — no lingering daemon. A terminal `pixi run
serve` sets neither, so it stays up until Ctrl-C.

`ui/src/lib/ipc.ts` has two backends: `import.meta.env.PROD` → HTTP;
otherwise the in-memory `mock.ts` (used by `pnpm dev` and Vitest).

## Architecture — three compile-time seams

```
fm-model   data types (Object, Kind, PropertyValue, frontmatter)
fm-query   PURE query engine — NO std::fs, NO db crate, NO paths  ← seam 1
fm-core    FileStore/MultiStore + BlobStore + ingest/verify/backup/git/merge  ← seam 2
fm-app     command library (commands.rs + dto.rs) + vaults.rs,
           behind ONE door: dispatch.rs  ← seam 3 (frontend ⇄ core)
fm-serve   std-only HTTP server; a transport shell over fm_app::dispatch
fm-cli     the `fm` binary (add/verify/manifest/backup/restore/check/set/show)
ui/        Svelte 5 + Vite; renderers are generic + literal-free
```

- **Seam 1 (the insurance policy):** `fm-query` must never depend on
  `rusqlite`/`std::fs`/paths. Enforced at compile time *and* by a CI grep
  (`ci/checks.sh`). The pure engine is what keeps the app testable and portable.
- **Seam 2:** everything is written against the `Store` trait; `FileStore` and
  `MemoryStore` stay behavior-equivalent (tested). A backend implements **`candidates`**
  (narrow + the residual filter), not `query` — `query` is a default method that runs the
  pure engine **once** over the result. That is what lets `MultiStore` federate by
  concatenating: you cannot union already-sorted/grouped/paginated results and recover
  `sort`/`limit`/`total` from them.
- **Seam 3 (the one door):** every command is reached through **`fm_app::dispatch`** —
  one `match` from a command name to a function, owning the vault list and the lock
  discipline. A frontend supplies only its *framing*: `fm-serve` parses a request into
  `(cmd, args, body)`, calls `dispatch`, and turns the returned `Output` (`Json` or
  `Bytes`) back into HTTP. Adding a command touches **4 places** — see
  [adding a feature](../src/dev/adding-features.md); adding a *frontend* means writing a
  shell, not a second dispatch table. The lock lives **inside** `App`, not in the
  signature: five arms drop it before slow I/O (`backup_status` shells out to `git
  ls-remote` per vault), so a `&mut Vaults` parameter would stall every `ping` behind
  a network round trip. The single genuinely platform-bound command, `open_external`, is
  the one-method **`Host`** trait the shell implements.
- **Not a command: `GET /api/blob/<reference>`.** Blob bytes stream from disk with the
  sniffed `Content-Type`, `Accept-Ranges` and `Range` — a command answers with a
  `Vec<u8>`, which is the shape that forces a whole video into memory. Non-inline-safe
  types are sent `Content-Disposition: attachment`, because a blob is now at a URL the
  browser can navigate to and blobs come from collaborators.

The SQLite index (FTS5) is **disposable**, rebuilt from the files on open. Git
versioning of the notes + restic backup provide durability.

## The commands — all of them through `fm_app::dispatch`

`board` · `gallery` · `agenda` · `get` · `search` · `recent` · `capture` ·
`set_property` · `update_body` · `delete` · `asset_status` · `resolve_asset` ·
`open_external` (the one platform-bound arm — a `Host` trait the transport implements) ·
`activity` · `commit` · `push` · `backup` · `backup_status` ·
`set_git_remote` · `ingest` · `pull` · `copy_note` / `copy_status` / `uncopy_note` ·
`list_vaults` (the audiences; **`[]` is the
first-run signal** — deliberately not `backup_status`, which shells out per vault) ·
`check_path` (what creating a vault here would do; the surface owns the verdict) ·
`forget_vault` (unregister one — the fourth verb, and the only one that removes; it never deletes a
file) · `create_vault` / `clone_vault` / `restore_vault` (**three ways a vault comes into being**:
start empty, git-clone a shared one, restic-restore a backup — identical registration, differing
only in what fills the folder first; all three route through `acquire::naturalise`) ·
`list_views` / `run_view` (**saved `.view`
files** — `query + a renderer`, parsed server-side, so the UI sends a *name* and no `Query`
ever crosses the wire) · `ping` (the **local poll**: its `{changed}` is how a `git pull` or a
Vim edit ever becomes visible, since every view is served from the index — 15 s, and only
while the tab is visible)

**Two `/api` routes are deliberately not commands**, because a command is the wrong shape
for them and putting them in `dispatch` would push a transport concern into the shared
surface:

- **`GET /api/blob/<reference>`** — blob bytes, streamed, with `Range`. A command answers
  with a `Vec<u8>`, which is exactly what forces a whole video into memory.
- **`POST /api/alive`** — liveness only, for *this* server's auto-shutdown watchdog. No
  lock, no filesystem, no dispatch. A frontend without a watchdog would never call it.

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
  **No new `Kind` and no new file type**: the scene JSON is the body of the note's own
  `<ulid>.md`. The one backend addition is `fm-core/src/scene.rs` — an element-level 3-way
  merge, so two people drawing at once get both their shapes instead of a mangled scene.
  It runs under the existing `*.md merge=fm` driver; there is no `.excalidraw` file and no
  second driver.

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
running), `test`, `test-ui`, **`check-ui`** (svelte-check — `vite build` does *not*
typecheck, so without this a component calling an unimported function builds clean and
throws in the browser), `deny`, `checks`, `docs`, and **`ci`** (runs
test + test-ui + check-ui + deny + checks + docs; the single CI gate). The shipped binary
uses `[profile.release]` in `Cargo.toml` (strip + thin-LTO → ~3 MB, vs ~34 MB debug).

## Current status

Per-feature status lives in [features.md](./features.md) (the always-read index) — shipped vs.
partial vs. planned, one line each, with a pointer to the detail. What is **not** working / deferred
is in [known-issues.md](./known-issues.md); the ranked work queue is [outstanding.md](./outstanding.md);
the dated log of how we got here is [sessions/](./sessions/).
