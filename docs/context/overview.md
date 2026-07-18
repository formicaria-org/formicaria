# Overview — read this first

A compact, high-density snapshot of the repo, meant to bootstrap a working
mental model **without** reading the whole codebase. When this disagrees with
the code, the code wins — fix this file.

_Last verified: 2026-07-18 — **A vault can be a repo you already have**
(`plan.md` Track V). Three of V2's four co-tenancy bugs are fixed, including both silent ones:
`.gitattributes`/`.gitignore` were **skipped when the file already existed**, so in any real
repo `*.md merge=fm` never landed (the merge driver silently never engaged — Phase 1's
disaster reintroduced by conversion) and `blobs/`+`index.sqlite` were committed and pushed;
`commit_all`'s `git add -A` staged your half-written code and your curated index every 5 s;
and `push_squashed` collapsed hand-written commits into one `backup:` (the boundary is now the
`auto:`/`backup:` message prefix — **never the author**, since we commit *as* the user). V3
shipped too: **`vault.json`** (`fm-core/src/descriptor.rs`) carries the three facts git cannot
supply — `name`, `description`, and **where the notes are** — so a project whose notes live in
`docs/` is adopted with no import step. Also decided, under the project's own principles:
**the git2 swap is rejected** and "git is a capability, not a dependency" stands (linking
libgit2 contradicts shell-out-don't-link, fails `deny.toml`'s permissive-only rule on
everything but a metadata technicality, and would silently disable the `.md` merge driver);
and **whiteboard blobs will be git-tracked** when the image-strip lands, which is deferred
because boards work today and a canvas cannot be verified without eyes on it. Before that,
**Sync is an explicit sequence now, whiteboards merge, and three
plan items were stopped before being built**
(`sessions/2026-07-18-sync-loop-and-scene-merge.md`). `ui/src/lib/sync.svelte.ts` is the loop:
`commit → push`, and on a rejection `pull → merge → push once more` — **exactly one retry**, and
**never a push after a conflicted pull** (publishing conflict markers as content is worse than not
publishing). `crates/fm-core/src/scene.rs` merges whiteboards **element-wise** before the text
merge ever sees the JSON — deletions honoured against the base (which is precisely what
Excalidraw's base-less `reconcileElements` cannot do), higher `version` wins, lower `versionNonce`
breaks ties, fractional `index` keeps the z-order — and it is **not** a second merge driver: there
is no `.excalidraw` file, `FileStore` writes `<ulid>.md` and `*.md merge=fm` already routes boards
there. Liveness split from reindex (`POST /api/alive`, a 15 s beat; the reindex beat is 15 s and
**visible-tab-only**; the watchdog idle window 10 s → 90 s, which is what actually fixes the
background-tab kill). Six live bugs fixed — the last three found by audit *this* session and closed rather than
inherited: a **duplicated `id:`** made the poll re-index and refresh forever (first path now
wins, the other is named); **no `Host` validation** left a DNS-rebinding path straight past the
CSRF guard; and a **surfaced error was wiped** by any background refresh. Plus: auto-commit committed **only the default vault**;
the "get changes" nudge pulled **without committing first** (git will not merge over a dirty tree —
the backup panel already knew; the same rule existed in two spellings); and **a stale editor could
overwrite a merge that landed under it** — `update_body` now takes the `updated` stamp the caller
last saw and refuses a superseded write. The mtime guard in `FileStore::put` could not cover that
one, because `pull` reindexes right after merging and so re-arms the very mtime the guard compares:
*noticing the change is what disarmed the protection against it*. On rejection the pane reloads and
puts the unsaved draft back **below** the merged text with markers — the `.md` driver's stance, one
level up. **Stopped on purpose,
each now a blocked task:** the **git2 swap** (`git2::merge_file` does not exist; libgit2 **cannot
invoke external merge drivers**, so porting `pull()` would silently disable the `.md` driver while
a collaborator's terminal git still honours it; and `deny.toml`'s permissive-only rule is violated
in spirit — it passes only because `libgit2-sys` under-declares its licence), the **Excalidraw
image-strip** (`blobs/` is gitignored, so stripping stops shared boards showing images, and the
two options are undecided), and **"swap in pragmatic-DnD's pointer adapter"** (2.0.1 ships no such
adapter). Before that, **Track M's host-side band shipped: there is one command surface
now, and blobs stream** (`sessions/2026-07-18-dispatch-and-blob-route.md`). **`fm_app::dispatch`**
is the single door to every command — `fm-serve`'s `api()` match, the `Vaults` state and its lock
discipline moved into `fm-app` (and `vaults.rs` moved up with them), leaving the server an HTTP
shell that parses a request into `(cmd, args, body)`, calls `dispatch`, and frames the `Output`.
That is Track M ruling 1, and it *precedes every mobile milestone*: until there was one door, each
new frontend was another copy of the dispatch table (`fm-cli` is the standing proof — it
re-implements against `fm-core` instead of calling `commands`). Three corrections the build made
to the plan: the lock could **not** move into the signature (five arms drop it before slow I/O —
`backup_status` shells out per vault), query params are **either/never both** with the JSON body
(else a `.json` asset would be read as its own arguments and file itself into another audience),
and `open_external` became a one-method **`Host` trait** rather than a `#[cfg]` ladder. Alongside
it, **`GET /api/blob/<reference>`** finally exists (ruling 7): streamed from disk, the sniffed
`Content-Type` that `resolve_asset` used to throw away, `Accept-Ranges` and real `Range` — so
`<video>`/`<iframe>` range-request instead of buffering a whole file into RAM twice. Serving blobs
from a *navigable* same-origin URL is a security change too, so anything outside an inline-safe
allowlist is sent `Content-Disposition: attachment` (blobs arrive from collaborators; an SVG
rendered as a document would run script in this origin). And **the 3 s poll stopped being
O(notes²)** — the deletion sweep tested membership against a `Vec`; 453 ms → 50 ms per quiet beat
at 10k notes, now pinned by a perf budget. The plan's cold-start `Incremental` switch was
**rejected on purpose** (mtime-only detection is blind to `restic restore`/`rsync -a`, and the
full rebuild at open is the only thing that heals them — see known-issues). `pixi run ci` gained
**`check-ui`**: `vite build` never typechecked, so a component calling an unimported function
built clean and threw in the browser — which is exactly the bug this session introduced and
shipped into `ui/dist` before catching it. Before that, **Track M — mobile was *planned*** (not built): formicaria on the
phone **itself**, overriding `MASTERPLAN:57`'s "phone = thin client" framing — one shared Rust
core, git-coordinated across devices (`sessions/2026-07-18-mobile-port-plan.md`; design in
[mobile-design.md](./mobile-design.md), rulings + the two reversals in
[decisions.md](./decisions.md), sequence as Track M in [plan.md](./plan.md)). Nothing mobile
exists yet. Before that, **Collaboration is git, exposed** (not reimplemented)
(`sessions/2026-07-18-git-collaboration-visualized.md`). One read-only `git::activity` (a
`git log --name-only` over `notes/*.md`, where a file's stem *is* its ULID) yields each note's last
editor, and that single command powers all of it: **`EditedBy` labels** ("● name · 5m ago") on
every card and the open note (person-coloured by the shared `hashHue`, delivered via the
runes-in-module store `activity.svelte.ts`, no prop-drilling); a first-class **Activity pane** (git's
log as a workspace view); a **contributor filter** (chips like the vault filter — `App.shown` gained
an author check, so hiding a person applies everywhere); and **automatic "someone pushed" awareness**
(a slow visibility-gated `remote_moved` poll → one-click `pull`). Nothing is stored — git stays the
source of truth. Deferred: creator attribution, anchored comments (need backlinks), live presence
(needs the descoped peer). Before that, **Cross-vault: create-in-vault + restrictive copy**
(`sessions/2026-07-18-cross-vault-copy.md`). You can now pick which vault a new note/board is
born in (a top-bar destination picker; `capture` gained a `vault` arg, mirroring `ingest`), and
**copy a note into another vault**. Copy is **restrictive by default** — only the prose travels;
`fm-app/refs.rs::strip_cross_vault` drops every `note:`/`asset:` reference (label and all) so a
copy can never point outside its new vault (ideas flow, artifacts don't). A copy is a **new note**
(fresh ULID); opting into "also copy the files" carries the first-degree blobs *into* the target
(content-addressed dedup, manifest refreshed) so it's self-contained; note links stay stripped
(linked-notes tier deferred). Every copy is behind a plain warning and leaves an `uncopy_note`
**Undo**. `Object.vault` is still never a field — audience is set by *which FileStore receives the
put*. Before that, **The UI is a flexible pane workspace now**
(`sessions/2026-07-18-flexible-workspace.md`). The single global view became a **CSS grid of
panes** the user opens, reorders (drag the header grip), resizes (drag the corner), and closes;
a horizontal **top bar** replaced the tall left rail (the explicit "sidebar wastes vertical
space" complaint). `panes.ts` is the pure core — `Pane`/`Workspace`, `MAX_PANES=8`, a
`Math.random`/`Date`-free `paneId()`, and the load-bearing **feed-key dedup**: N panes over M
distinct feeds cost M fetches, not N. Each pane's `onmove` carries its **own** `groupBy`, so two
boards no longer write each other's property. The whole `Workspace` persists to
`localStorage['fm-workspace']` (a view preference, never a vault). Deliberately a flat pane list
+ spans, **not** hardcoded preset splits and **not** a recursive split tree — the ask was
"rearrange them where I want," and the split tree is the one shape with no stopping point.
A note is a **pane kind** too now (`kind:'note'` → `NotePanel`, board-notes → whiteboard), so
the old side-trail is retired; `openNoteInPane` dedups by id so one note never opens two editors
over one file. Verified structurally (CI green, 164 UI tests) but **not seen** — the browser
extension is not connected, so the owner's eyes remain the layout check. Before that, **`.view`
files: saved queries, any renderer**
(`sessions/2026-07-17-view-files.md`). A `.view` (YAML, in `vault/views/`, git-tracked) is
`query + a renderer` — `MASTERPLAN:323`'s own deferred design, now built. Parsed server-side
in `fm-app/views.rs` → `fm_query::Query`; the UI sends a **name** (`list_views`/`run_view`),
so no `Query` crosses the wire — which keeps serde off the pure crates and the
`PropertyValue` `Ord` trap unreachable (the DSL has no ordered `prop` comparison; dates go
through `date:`). A `.view` **extends** a preset's filter by `Vec::extend`, so `Kind(Note)`
(the assets exclusion) is never forgotten; a broken view is listed with its parse error, not
dropped. This gives the engine's previously-unreachable predicates
(`Not`/`Any`/`TagsAll`/`TagsAny`/`DateRange`) their first callers. Before that, **the read
view sanitizes, and the note trail stopped being modal**
(`sessions/2026-07-17-ui-sanitize-demodalize.md`). `render.ts` now runs DOMPurify on
`marked`'s output before the DOM sees it — the top security item in `known-issues.md`, made
live by collaboration (a shared note body could run script in your origin and push your vault
anywhere); the config widens the URI allow-list by exactly `note:`/`asset:`/`sha256:` so our
own chips and inline media survive. And the note trail is a **peer grid column** now, not a
`z-50` overlay with a backdrop: the board stays live beside an open note (see the "Views"
note below). Before that, **A vault is now created, not invented**
(`sessions/2026-07-17-create-vault.md`). The `FM_VAULT` default (`"vault"`, *relative*) is
gone: unset means **zero vaults**, a real state that gates the whole UI on a first-run
screen, because a typo or a launcher started from another cwd used to silently create an
empty vault named after the mistake while your notes appeared to vanish. `vaults.json` gains
**its first writer** (`fm-app/src/vaults.rs`, then in `fm-serve` — value-tree merge, append-only, refuses a
file it could not parse, writes the whole live list so an `FM_VAULT` vault cannot vanish);
`check_path`/`create_vault`/`list_vaults`; `MultiStore::open(&[])` legal + `NoVaults` +
`add` (live, no restart); one mutex over the store **and** the list, since `api()` already
took them in opposite orders. Creation deliberately does **not** `git init` — `commit_all`
already does, and eager init inside a repo the user owns would `git init` a nested one
shadowing theirs. Before that, **Track C Phases 0, 1 and 2 shipped: a shared vault works, and
the plural is true** (`sessions/2026-07-17-phase-{0,1,2}.md`). Two people can now edit
different paragraphs of the same note and the merge is **clean**, across a *set* of vaults
each with its own repo and audience — verified as a real round trip through the API, not
just in tests. Phase 1 added: the **`FileStore::put` staleness guard**
(`StoreError::Conflict` — *the* lost-update bug, where a board drag rewrote the whole file
from a stale copy); **incremental reindex** + a local poll folded into the existing
`ping` heartbeat (without it a `git pull` is invisible, because the views are served from
SQLite) — *the fold was undone 2026-07-18: liveness is `POST /api/alive` now and the poll
is a separate, visible-tab-only 15 s beat*; **`git::remote_moved`** (one `ls-remote`, moves no refs) and **`pull`**; and the
**`.md` merge driver** (`fm-core/src/merge.rs`, `fm merge-md`, installed by `ensure_repo`)
which resolves `updated:`/`tags` structurally and hands the body to `git merge-file` — so a
conflict lands **in the body**, leaving the note parseable and editable. **Phase 2 shipped
too: the plural is now true** — a *set* of vaults, each its own repo and audience, under
one set of views (`Object.vault` derived from location and never serialized — *location is
the permission*; the **`candidates` seam** so `Store::query` is a default method and FTS5
federates for free; `MultiStore`; the vault list at `~/.config/formicaria/vaults.json`;
**per-vault git** — the backup panel is a list, not a form; **blobs searched across
vaults**; badges + a vault filter). **And the rename landed with it: the tool is
`formicaria` now**, the plural being the architecture, and nothing carries the old name.
One rule outlives it: `git.rs`'s `PLACEHOLDER_EMAIL` is a sentinel matched **by value**, so
changing it again would hand every vault still on the old value a "real" identity and
reopen the provenance hole Phase 0 closed (`decisions.md`).
Before that, **Phase 0 made the app survive a merge**: four fixes, each a single-user bug today and data loss the moment a vault is
shared: `reindex` **skips an unreadable note** instead of failing
`FileStore::open` (one conflicted `.md` used to brick startup — `fm-serve` names it on
stderr, though it is still invisible in-app: see known-issues); `commit_all` **refuses
mid-merge** rather than committing `<<<<<<<` as a note's content; `push_squashed`
**squashes only onto an ancestor**, closing a silent data-loss path that opens the moment
anything fetches; and a vault needs a **real committer identity before it can gain a
remote** — the backup panel asks for a name and email, but only of people git has never met
(`git::identity`, `backup_status.identity`). The `formicaria@localhost` placeholder is a
**sentinel** for audience-less vaults, matched by value (`decisions.md`). Before that, the **forward plan was consolidated** into [plan.md](./plan.md) (the program:
Track S single-user + Track C collaboration; `roadmap.md` folded in,
`collaboration-design.md` kept as its audit). Before that,
**backup is now two tiers** (`BackupPanel.svelte`):
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
(`openIds: string[]`). **The trail is a peer grid column, not a modal overlay**
(de-modalized 2026-07-17): the board stays live beside an open note, no backdrop dismisses
it — reading a note is no longer a *mode*, which is the thesis that a note *is* the task
*is* the card, made literal. `wide` (persisted) now means "the note takes the whole content
area" vs "docks beside the view". The `.app` grid is three custom-property columns
(`--rail`/`--main`/`--trail`) so rail-collapse and the trail compose without a
grid-template explosion.
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
[plan.md](./plan.md) — the sequenced program (Track S:
calendar sync, whiteboard-in-note + PDF; Track C: collaboration)._

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
`create_vault` (create + register + open, live) · `list_views` / `run_view` (**saved `.view`
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

## Current status (2026-07-18)

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
verify/manifest (bit-rot), debounced git auto-commit (**every vault**, surfacing its first
failure rather than swallowing it — commits can still lag, see known-issues), a
design-token system (dark+light), the horizontal top-bar shell, the NotePanel
(**full screen by default**, toggle to a docked side-sheet; remembered per
browser), note delete (double-confirm), a ⌘K command palette,
an mdBook manual, and a `.desktop` launcher.

See [known-issues.md](./known-issues.md) for what is **not** working / deferred,
and [sessions/](./sessions/) for how we got here.
