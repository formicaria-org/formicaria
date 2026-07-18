# Known issues, gaps & traps — what is *not* working

Honest status of rough edges, deferred work, and things that will bite you.
Keep this current: when you fix something, delete its entry; when you hit a new
trap, add one. Newest concerns first within each section.

_Last verified: 2026-07-18, after the dispatch extraction, the blob route, the sync loop and the
scene merge (`sessions/2026-07-18-dispatch-and-blob-route.md`,
`sessions/2026-07-18-sync-loop-and-scene-merge.md`). Prior: the mobile-port code audit
(`sessions/2026-07-18-mobile-port-plan.md`); 2026-07-16 (assets/status/kanban/slash-menu/
edit-gesture)._

> **Looking for what to work on?** This file is the honest *description* of rough edges,
> including ones we have decided to live with. The ranked **work queue** — with what "done"
> looks like for each — is [outstanding.md](./outstanding.md).

## Known gaps / not fully working

- **~~Unsanitized `innerHTML` in `render.ts`~~ — FIXED 2026-07-17.** `render.ts` now runs
  DOMPurify on `marked`'s output before the DOM sees it (`sanitize()`), so a collaborator's
  `<img onerror>` / `<script>` is stripped. The config widens DOMPurify's URI allow-list by
  **exactly** `note:`/`asset:`/`sha256:` — our own pipeline speaks those three, and stripping
  them would silently kill every asset image and note chip (the trap this fix had to avoid).
  Math (`span[data-math]`) and Mermaid (`code.language-mermaid`) placeholders survive because
  the resolve passes run *after* sanitize; tests pin both the stripping and the survival.
  Mermaid's own SVG sink still relies on its `securityLevel: 'strict'`, documented in place.
  `fm-serve`'s header was corrected from "single user".
- **~~You are only told someone pushed if you open the backup panel~~ — FIXED.** A visibility-
  gated 45 s `remote_moved` poll feeds a top-bar chip with one-click pull, and that pull now
  goes through `sync.svelte.ts`'s `pullVault` (commit first — git will not merge over a dirty
  tree — then name any conflicted notes rather than throwing a string). Still true: a
  conflicted note is surfaced only by name, and the `.md` driver puts the markers in the note
  *body*, so it opens and resolves in the ordinary editor.
- **Reindex still stats every file, on every beat.** `Reindex::Incremental` re-*reads* only
  what moved (Phase 1), but the scan itself is still O(n) `stat`s, and the `ping` heartbeat
  runs it on every beat (15 s, and only while the tab is visible). Now gated by a perf-budget test at 10k notes
  (`fm-core/tests/perf.rs`) — which is what caught the deletion sweep being O(n²) against a
  `Vec` (453 ms → 50 ms per quiet beat, 2026-07-18). Before reaching for a watcher (inotify)
  note that a watcher is a dependency and a per-platform behaviour, which is why polling won
  on the way in. `FileStore::open` is still a **full** rebuild by design (the
  disposable-index escape hatch), and switching it to `Incremental` is **not** a drop-in:
  mtime-only detection is blind to every mtime-preserving writer (`restic restore`,
  `rsync -a`, `cp -p`, `tar -x`), and the full rebuild at open is currently the only thing
  that heals them. Doing it needs an index-format version gate (nothing in CI enforces the
  bump), an `objects(path)` index — `forget_path` full-scans today, so Incremental can be
  *slower* than Full after a big pull — and cold-start tests that do not exist. Wanted for
  mobile (Android kills backgrounded apps, so every relaunch pays a full rebuild); worth
  little on desktop, which starts once.
- **A stale editor can no longer overwrite a merge — but only where `updated` moves.**
  Fixed 2026-07-18: `update_body` takes the `updated` stamp the caller last saw and returns
  the new one; a mismatch is `StoreError::Conflict`, and `NotePanel` reloads and puts the
  unsaved draft back *below* the merged text with markers rather than discarding it.
  **Why the existing mtime guard could not cover this:** `FileStore::put` refuses a write
  whose file moved since we indexed it — but `pull` merges and then *reindexes* (a merge is
  invisible until it does), which records the post-merge mtime and stands the guard down
  exactly when it was needed. The staleness lives in the client, so the client declares its
  base.
  **The residual gap:** the check is on `updated`, so a writer that changes a body *without*
  bumping `updated` is still invisible to it — hand-editing a note in Vim is the realistic
  case (a real merge always bumps it, because the `.md` driver resolves `updated` to the
  later of the two). Narrow, but real: Vim-edit a note that is also open in the app, and the
  app's next save still wins. Closing it needs a content hash rather than a timestamp.
- **The phone shell has never run on a phone.** The reflow and the tap→move menu are verified
  at a narrow viewport, by `pointer: coarse`, and by component tests — not on a device. Chrome's
  touch emulation is *actively misleading* here: it synthesises PointerEvents but does not
  reproduce Android's `dragstart` suppression, so emulation can hide the very bug the menu
  exists to work around. One real device is needed once, to confirm card drag is genuinely dead
  there, the menu is reachable, and the targets are hittable.
- **~~A surfaced error is not durable~~ — FIXED 2026-07-18.** `refresh()` now clears only the
  errors *it* raised (a feed that failed to load, which the next successful refresh genuinely
  resolves). Anything about the user's data — a failed sync, a refused save — is reported
  through `report()`, survives background activity, and has a dismiss button like `notice`.
- **~~Two files carrying the same `id:` make the poll flap forever~~ — FIXED 2026-07-18.**
  `objects.id` is the primary key and `index_object` is `INSERT OR REPLACE`, so a duplicated
  note used to collapse to one row whose `path` alternated: each beat re-indexed whichever
  path was currently missing, reported `updated: 1`, and the UI refreshed forever on a vault
  nobody was touching. Now the **first path wins and the other is named** in `skipped` —
  serving one file's content under another's id is worse than serving neither, and the
  unreadable-note skip already sets that discipline. Pinned by a test that asserts three
  consecutive quiet polls report nothing.
- **No per-view object cache.** Board/Agenda/Timeline each YAML-parse the whole
  corpus via `load_all` per request. Same scale caveat as above.
- **~~`fm-serve` never validates the `Host` header~~ — FIXED 2026-07-18.** Binding to
  127.0.0.1 keeps other *machines* out but does not decide which *name* a browser used, so a
  hostname an attacker controls, resolved to 127.0.0.1, arrived same-origin with itself and
  sailed past the CSRF guard. Requests now must carry a Host we actually serve
  (`127.0.0.1`/`localhost`/`::1`) or get a 403. **A missing Host still passes** — that is
  HTTP/1.0 or a hand-rolled client, not a browser, so not this vector, and refusing it would
  break curl for no gain.
- **Missing media is a warning, never a crash** — by design. A missing blob
  renders the `.asset-missing-inline` placeholder; don't "fix" it into an error.
- **The mock now mirrors one guard deliberately.** `mock.ts`'s `update_body` throws the same
  conflict the server does, because a mock that quietly accepts a write the backend would
  refuse is how the UI's rejection path stays untested until a user finds it.
- **The mock can drift from the real contract silently.** `mock.ts` returns `… as T`,
  which casts the type check away — so it kept a top-level `restic_repo` long after restic
  became per-vault, and `tsc` said nothing. `backup_status` now builds a typed
  `BackupStatus` first; the other arms are still bare casts. If a UI test passes against a
  shape the Rust doesn't send, this is why.
- **Auto-commit is best-effort; commits can lag** (audited 2026-07-17, half-fixed
  2026-07-18). `scheduleCommit` debounces 5s and every GUI write reaches it (4 App call
  sites + `onsaved` from NotePanel's five write paths). **No longer silent, and no longer
  default-vault-only**: it commits *every* vault (it used to call `commit()` with no vault
  argument, so on a multi-vault install exactly one repo had a history) and it states the
  first failure in the notice banner instead of `.catch(() => {})`. What is still true: the
  timer is a browser `setTimeout` that **dies with the tab** — and with `FM_AUTO_SHUTDOWN`
  closing the tab *is* how you quit, so "edit, then close" can skip that commit — and
  **`fm-cli`/Vim writes never commit** (no `fm commit` subcommand). Nothing surfaces "you
  have uncommitted edits". **And since 2026-07-18 a Vim edit is never committed by us at
  all**: `commit_all` stages exactly the paths `put`/`delete` recorded, so a note you are
  hand-editing is not swept in mid-sentence — the flip side being that it is not versioned by
  the app either, and is yours to commit. Files are still never at risk (atomic temp+rename);
  what lags is *our* commits of *our* writes.
  So: **files are never at risk; commits can lag.** Don't restate this as "history is
  always safe" — it isn't. Also, the spec (`MASTERPLAN.md:350`) says "500 ms→disk,
  30 s/blur→commit": the code is 5 s with **no blur handler**, and `MASTERPLAN.md:391`
  still lists auto-commit as *not built* while `:426` lists it as shipped.
- **A backup destination may be local and that is not an error.** A git remote can
  be a path/`file://`, and `FM_RESTIC_REPO` is a bare path when local. `reachOf`
  (`destination.ts`) classifies both; the panel must keep saying which. Never
  report a local destination as "off this machine".
- **Board column *and card* order are client-side, and column reorder is currently inert
  in the pane workspace** (`localStorage['fm-board-order']`
  and `['fm-card-order']`, both keyed by group-by; card order additionally by
  column value) — view preferences, per-browser, **not** in the vault, so they
  don't sync across machines. Intentional; the vault-side `.view` file would
  change that (still deferred). Column DnD is mouse-only (like card DnD).
- **The caret-anchored `/` menu is unverified in headless.** `caret.ts` measures
  with a mirror div, and **jsdom has no layout** — `caretXY` returns zeros there,
  so the menu degrades to the editor's top-left and the tests can't see the real
  placement. Only a real browser proves it; re-check by eye after touching the
  editor's font/padding, since the mirror clones exactly those properties.

- **Whiteboard (Excalidraw) caveats.** (1) ~~Fonts come from a CDN~~ **fixed
  2026-07-17**: `ui/scripts/copy-excalidraw-fonts.mjs` self-hosts them at build time and
  `index.html` sets `window.EXCALIDRAW_ASSET_PATH`. The old note deferred this as "~14 MB,
  mostly CJK" — that number was doing all the work and was misleading: **13 of the 14 MB is
  one font** (Xiaolai, CJK). Everything else, Excalifont included, is ~390 KB, so the real
  cost was 362 KB on a 12 MB binary — for making "nothing phones home" true. Xiaolai is
  skipped, so CJK text in a whiteboard uses a system font. **Lesson: a deferral justified by
  a single number deserves the number re-measured.** (2) The canvas renders only in a real
  browser, so
  it's **unverified in headless CI** (build, code-split, and the board round-trip
  are tested; the visual editor is not). (3) A board can't yet be **embedded in a
  note** — it's a standalone note you open on its own. Planned in
  [plan.md](./plan.md) (Track S #4).

## Deferred (intentionally not built yet)

- Global capture hotkey (was window-only; needs rethinking for the browser).
- Optional mlua scripting hatch.
- **v2:** CM6 live-preview editor, backlinks panel, watched inbox, OCR, video
  posters, semantic search. (Forward note references + the sliding-pane trail
  **shipped 2026-07-16**; only the *backlinks* half is still deferred, and it
  needs a link index — see [plan.md](./plan.md) (Track C, Phase 3).)
- **Calendar sync, whiteboard-in-a-note + PDF export, and the `Source`/local-model
  ingest module** are *planned, with the design decided* — see
  [plan.md](./plan.md) (Track S) rather than re-deriving them.

## Historical / no longer relevant

- The entire "desktop window rendering / blank WebKitGTK / gui-env / global
  shortcut / custom-protocol / screenshot recipe" saga is **dead** — the window
  was removed 2026-07-15. Ignore those notes for how-to-run; kept only as the
  reason behind the browser pivot ([decisions.md](./decisions.md)).

## Traps for whoever works here next

- **Line endings are LF, and both halves matter.** `from_file` tolerates CRLF because a
  Windows editor produces it; the repo's `.gitattributes` (`* text=auto eol=lf`) stops git
  producing it in the first place; and `ensure_repo` writes `*.md merge=fm text eol=lf`
  into every vault for the same reason. Remove any one and Windows breaks *silently*: the
  loader is deliberately tolerant, so unparseable notes don't error — they vanish, and the
  vault opens empty. That is exactly how CI found it (all eight `e2e_vault` tests at once,
  because they're the only ones reading committed fixtures rather than writing their own).

- **`.desktop` has no relative `Exec`** — it must be absolute, so the entry cannot be a
  static file in the repo. It was one, carrying `/home/baljinder/...`, which meant every
  clone got a launcher into a stranger's home *and* a rename silently rewrote the path to
  somewhere that didn't exist (the file looked right and launched nothing).
  `packaging/install.sh` now **generates** it from the real checkout path. Moving the repo
  = re-run `install.sh`; there is no fixing it from inside the file.

- **`fm-cli` does NOT front `fm-app` — the command logic is forked in two.** Despite the
  overview's "one command library behind two frontends" framing, `crates/fm-cli/Cargo.toml` has
  **no `fm-app` dependency** and `fm-cli/src/main.rs` reimplements the commands against `fm-core`
  directly (e.g. `Cmd::Add` at `main.rs:131-146` rebuilds ingest instead of calling
  `commands::ingest`). Only **`fm-serve`** is a thin frontend over `fm-app::commands`; and
  `fm-serve::api()` itself is not "thin over `MultiStore`" — it dispatches over `Mutex<Vaults{
  MultiStore + Vec<VaultConfig>}>` with a documented single-lock discipline (`main.rs:31-37`) and
  a dozen arms that reach around the `Store` trait to per-vault paths. So there are **two** command
  surfaces today, and any new frontend (the planned mobile bridge) is a **third**. *(Half
  fixed 2026-07-18: `fm_app::dispatch` shipped and `fm-serve` is now a transport shell over
  it — so there are **two** surfaces, not three-in-waiting. `fm-cli` still re-implements
  against `fm-core`, which is the remaining fork.)* Track M's
  ruling 1 (extract `fm_app::dispatch`) exists to collapse these; until it lands, a change to a
  command's behaviour must be made in **both** `fm-app` and `fm-cli` or they drift. Found by the
  2026-07-18 mobile-port audit.
- **`pixi run build` must build `fm`, not just `fm-serve`.** `ensure_repo` installs the
  `.md` merge driver by pointing git at the `fm` binary **beside the running one**, and
  deliberately installs nothing when it can't find one. So a build task that ships only
  `fm-serve` makes the merge driver silently never install — every concurrent edit then
  conflicts on the `updated:` line, i.e. the single thing Phase 1 exists to prevent, with
  no error anywhere. It was like this for a whole phase and every test passed, because the
  tests build `fm-cli` themselves. **Only a clean release build finds this.** If you ever
  split the workspace or trim the build task, this is what breaks first and quietest.

- **Sandboxed Bash fails** in this environment with a seccomp/`setgroups`
  error. Run shell commands with `dangerouslyDisableSandbox: true`.
- **The desktop launcher runs a PREBUILT binary and never recompiles.**
  `pixi run app` execs `target/release/fm-serve` + the built `ui/dist` as they are
  on disk — that's what makes the icon start instantly. So a backend change you
  just made is **invisible** to the owner until someone runs `pixi run build`
  (this bit us on 2026-07-16: a query-layer filter looked unimplemented for an
  hour). `pixi run serve` rebuilds debug and hides the trap. **Run `pixi run build`
  before asking the owner to verify anything**, and check
  `stat target/release/fm-serve` against your edit when a fix "didn't work".
- **Toolchain is not on PATH.** `cargo/node/pnpm/mdbook/restic` live in
  `.pixi/envs/default/bin`. Use `pixi run <task>` or
  `pixi run -e default <cmd>`; a bare `cargo`/`pnpm` in a background shell will
  be "command not found".
- **`fm-serve` env vars:** `FM_VAULT` (no default — unset means the first-run screen), `FM_UI_DIST` (default
  `ui/dist`), `FM_ADDR` (default `127.0.0.1:8765`), `FM_OPEN` (xdg-open the
  browser), `FM_RESTIC_REPO` / `RESTIC_PASSWORD` (the **media** backup tier only —
  the notes tier needs neither).
- **An unreadable `.md` disappears from the app with only a stderr line.** Since
  Phase 0 the vault opens and serves the rest (`FileStore::skipped()`, warned about
  at `fm-serve` startup), which is the right trade — but a user in the browser sees
  the note **silently missing**, and the terminal is the only place that says why.
  The in-app list is Phase 1's "conflict surfacing" (`plan.md`).
- **`fm-serve` is only partly tested.** The blob route now has real response-path tests
  (a listener on port 0, a live socket: sniffed type, the `nosniff`/attachment allowlist,
  `Range`/206/416, 404) and the query-args split has unit tests. Everything else — the CSRF
  guard, the `Host` guard, static serving, the watchdog — is still exercised only by hand,
  and every UI test runs against `mock.ts`. Narrower than it was; not closed.
- **A backgrounded tab can shut the app down.** The heartbeat is 3s
  (`App.svelte:229`) but browsers throttle background timers to ~1/min, while the
  watchdog idles out at 10s (`main.rs:91`). Only bites with `FM_AUTO_SHUTDOWN`
  (i.e. the desktop launcher). Found by the 2026-07-17 audit.
- **SQLite has no `busy_timeout`/WAL**, and every `fm` CLI command takes an
  exclusive write lock — so the CLI races a running server. Found by the
  2026-07-17 audit.
- **The API silently accepts malformed JSON.** `api()` does
  `serde_json::from_slice(body).unwrap_or(Value::Null)` (`fm-serve/src/main.rs:164`)
  and `s(k)` then `unwrap_or("")`, so a client bug arrives as an **empty-string
  arg**, not an error. This bit during verification (a bad test body became
  `set_git_remote("")`, and `git remote add origin ""` *succeeds*, leaving a
  remote whose `get-url` reports its own name). `set_remote` now refuses a blank
  URL; other commands are still exposed to this.
- **Path traversal** on the static route is rejected with 400 — verify with
  `curl --path-as-is` (plain curl normalizes `../` client-side and hides it).
- **Renderers:** no `todo/doing/done`, no scheduling literals — CI grep
  (`ci/checks.sh`) will fail the build.
- **Every NotePanel pane mounts its own `<svelte:window onkeydown>`**, so a key
  press is heard by *all* panes in the trail. Pane-scoped shortcuts must go
  through `ownsKeys()` (focus inside some pane → only that pane acts), or you get
  the bug Escape-exits-edit had: pane 1 closing the trail while you leave pane 2's
  editor.
- **The note trail is now a peer grid column, not a modal overlay** (de-modalized
  2026-07-17). The board stays live beside an open note and no backdrop dismisses it —
  close with the ✕ button or Escape (`NotePanel.onPaneKey`). Deliberately **left
  conservative**: `App.onGlobalKey`'s `if (openIds.length) return` still suppresses the
  app-level `1/2/3`/`c`/`/` shortcuts while a note is open, so those don't drive the view
  beside it. Making them focus-aware is the *pane-grid* rabbit hole the workspace design
  explicitly rejected — do not open it without a reason the two-region layout can't meet.
- **Assets are excluded from `board`/`agenda`/`recent`** (`Predicate::Kind`), so a
  board grouped by `type` has only a `note` column *by design* — don't "fix" it.
  Keep `search`/`gallery` seeing assets: search is the only way to find a PDF by
  its extracted text, and the `/` menu's asset insertion rides on it.
- **~~`pixi run ci` does NOT typecheck Svelte~~ — FIXED 2026-07-18** (`check-ui` runs
  svelte-check, and `vite build` still does not typecheck, which is why the task exists).
  The original entry: it was `test, test-ui, deny,
  checks, docs` — no `svelte-check`, and no `vite build` either. A `.svelte` file
  can be type-broken with CI green. After component work run
  `pixi run pnpm -C ui check` (and `pixi run pnpm -C ui build`) by hand.
- **`mock.ts` state leaks across tests in a file.** `bodyOverrides` and the `seq`
  counter are module-scope, and vitest isolates per *file*, not per test — so
  `App.flow.test.ts`'s edit walk rewrites the GAE note's body for every test
  after it. Anchor a later test to a note the walk doesn't touch, or add a reset.
- **jsdom has no layout.** `scrollTo`/`getBoundingClientRect`/`IntersectionObserver`
  are absent or stubs, so guard them (`el?.scrollTo?.(…)`) the way the
  `localStorage` reads are guarded — an unguarded call is an unhandled rejection
  in the suite and a real crash in any browser that lags the API.
- **Do not add fs/db to `fm-query`** — compile-time + CI enforced.
- **Commit discipline:** solo repo, work on `main`, no branch/PR ceremony — but
  commit **only when the user asks**.
- **`vault/` is gitignored** (the knowledge vault is its own repo). Test
  fixtures under `crates/*/tests/fixtures/` are NOT the root `/vault/` and are
  tracked.
