# Known issues, gaps & traps — what is *not* working

Honest status of rough edges, deferred work, and things that will bite you.
Keep this current: when you fix something, delete its entry; when you hit a new
trap, add one. Newest concerns first within each section.

_Last verified: 2026-07-16 (assets/status/kanban/slash-menu/edit-gesture)._

## Known gaps / not fully working

- **Unsanitized `innerHTML` in `render.ts`.** The read view sets `el.innerHTML`
  from `marked.parse()` with **no DOMPurify** — raw HTML / `<img onerror>` in a
  note body reaches the DOM. A characterization test pins this *current*
  behavior. Accepted as low-risk for a single-user local tool; adding a
  sanitizer is a filed follow-up, not done.
- **You are only told someone pushed if you open the backup panel.** `git::remote_moved`
  (one `ls-remote`, moves no refs) is computed in `backup_status`, so nothing surfaces
  "Ravi pushed" on its own. The plan's automatic 15–30 s poll needs a timer and somewhere
  in the chrome to show it. Same for conflicted notes: `backup_status.conflicts` lists them,
  but only in that panel — though the `.md` driver does put markers in the note *body*, so
  a conflicted note opens and resolves in the ordinary editor.
- **Reindex still stats every file, every 3 s.** `Reindex::Incremental` now re-*reads*
  only what moved (Phase 1), but the scan itself is still O(n) `stat`s, and the `ping`
  heartbeat runs it on every beat. Fine at this scale and far cheaper than the full
  re-parse it replaced; if the vault reaches ~10k notes, gate it behind a perf-budget test
  before reaching for a watcher (inotify) — a watcher is a dependency and a per-platform
  behaviour, which is why polling won on the way in. `FileStore::open` is still a **full**
  rebuild by design (the disposable-index escape hatch).
- **No per-view object cache.** Board/Agenda/Timeline each YAML-parse the whole
  corpus via `load_all` per request. Same scale caveat as above.
- **Inline media buffers whole blobs into memory.** `resolve_asset` returns full
  bytes → a typed `Blob` object URL. Fine for local single-user. Planned
  enhancement: a streaming `GET /api/blob/<hash>` in `fm-serve` (correct
  Content-Type, `Accept-Ranges`, honor `Range`) so `<video>`/`<iframe>`
  range-request instead of buffering.
- **Missing media is a warning, never a crash** — by design. A missing blob
  renders the `.asset-missing-inline` placeholder; don't "fix" it into an error.
- **The mock can drift from the real contract silently.** `mock.ts` returns `… as T`,
  which casts the type check away — so it kept a top-level `restic_repo` long after restic
  became per-vault, and `tsc` said nothing. `backup_status` now builds a typed
  `BackupStatus` first; the other arms are still bare casts. If a UI test passes against a
  shape the Rust doesn't send, this is why.
- **Auto-commit is best-effort and silent** (audited 2026-07-17). `scheduleCommit`
  (`App.svelte:342`) debounces 5s and every GUI write reaches it (4 App call sites
  + `onsaved` from NotePanel's five write paths), but: it **swallows every error**
  (`commit(...).catch(() => {})`), the timer is a browser `setTimeout` that **dies
  with the tab** — and with `FM_AUTO_SHUTDOWN` closing the tab *is* how you quit,
  so "edit, then close" skips that commit — and **`fm-cli`/Vim writes never
  commit** (no `fm commit` subcommand). Nothing surfaces "you have uncommitted
  edits". Saving grace: `commit_all` is `git add -A`, so a missed change rides
  along in the next commit, and files are already on disk via atomic temp+rename.
  So: **files are never at risk; commits can lag.** Don't restate this as "history
  is always safe" — it isn't. Also, the spec (`MASTERPLAN.md:350`) says
  "500 ms→disk, 30 s/blur→commit": the code is 5 s with **no blur handler**, and
  `MASTERPLAN.md:391` still lists auto-commit as *not built* while `:426` lists it
  as shipped.
- **A backup destination may be local and that is not an error.** A git remote can
  be a path/`file://`, and `FM_RESTIC_REPO` is a bare path when local. `reachOf`
  (`destination.ts`) classifies both; the panel must keep saying which. Never
  report a local destination as "off this machine".
- **Board column *and card* order are client-side** (`localStorage['fm-board-order']`
  and `['fm-card-order']`, both keyed by group-by; card order additionally by
  column value) — view preferences, per-browser, **not** in the vault, so they
  don't sync across machines. Intentional; the vault-side `.view` file would
  change that (still deferred). Column DnD is mouse-only (like card DnD).
- **The caret-anchored `/` menu is unverified in headless.** `caret.ts` measures
  with a mirror div, and **jsdom has no layout** — `caretXY` returns zeros there,
  so the menu degrades to the editor's top-left and the tests can't see the real
  placement. Only a real browser proves it; re-check by eye after touching the
  editor's font/padding, since the mirror clones exactly those properties.

- **Whiteboard (Excalidraw) caveats.** (1) Excalidraw fetches its hand-drawn
  fonts from a CDN unless `window.EXCALIDRAW_ASSET_PATH` points at locally-served
  copies — offline, boards still work but fall back to system fonts. Local-font
  bundling (copy `@excalidraw/excalidraw/dist/prod/fonts` into the build, ~14 MB,
  mostly CJK) is **deferred**. (2) The canvas renders only in a real browser, so
  it's **unverified in headless CI** (build, code-split, and the board round-trip
  are tested; the visual editor is not). (3) A board can't yet be **embedded in a
  note** — it's a standalone note you open on its own. Planned in
  [plan.md](./plan.md) (Track S #4).

## Deferred (intentionally not built yet)

- Global capture hotkey (was window-only; needs rethinking for the browser).
- `.view` declarative config files (layer-2 extensibility; the renderers are
  hardcoded commands for now).
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

- **`.desktop` has no relative `Exec`** — it must be absolute, so the entry cannot be a
  static file in the repo. It was one, carrying `/home/baljinder/...`, which meant every
  clone got a launcher into a stranger's home *and* a rename silently rewrote the path to
  somewhere that didn't exist (the file looked right and launched nothing).
  `packaging/install.sh` now **generates** it from the real checkout path. Moving the repo
  = re-run `install.sh`; there is no fixing it from inside the file.

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
- **`fm-serve` env vars:** `FM_VAULT` (default `vault`), `FM_UI_DIST` (default
  `ui/dist`), `FM_ADDR` (default `127.0.0.1:8765`), `FM_OPEN` (xdg-open the
  browser), `FM_RESTIC_REPO` / `RESTIC_PASSWORD` (the **media** backup tier only —
  the notes tier needs neither).
- **An unreadable `.md` disappears from the app with only a stderr line.** Since
  Phase 0 the vault opens and serves the rest (`FileStore::skipped()`, warned about
  at `fm-serve` startup), which is the right trade — but a user in the browser sees
  the note **silently missing**, and the terminal is the only place that says why.
  The in-app list is Phase 1's "conflict surfacing" (`plan.md`).
- **`fm-serve` has no tests at all.** The entire production transport is unverified,
  and every UI test runs against `mock.ts` — so the real HTTP path is only ever
  exercised by hand. Found by the 2026-07-17 audit.
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
- **Assets are excluded from `board`/`agenda`/`recent`** (`Predicate::Kind`), so a
  board grouped by `type` has only a `note` column *by design* — don't "fix" it.
  Keep `search`/`gallery` seeing assets: search is the only way to find a PDF by
  its extracted text, and the `/` menu's asset insertion rides on it.
- **`pixi run ci` does NOT typecheck Svelte.** It is `test, test-ui, deny,
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
