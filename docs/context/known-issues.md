# Known issues, gaps & traps — what is *not* working

Honest status of rough edges, deferred work, and things that will bite you.
Keep this current: when you fix something, delete its entry; when you hit a new
trap, add one. Newest concerns first within each section.

_Last verified: 2026-07-15 (uncommitted follow-ups on commit `067e80b`)._

## Known gaps / not fully working

- **Unsanitized `innerHTML` in `render.ts`.** The read view sets `el.innerHTML`
  from `marked.parse()` with **no DOMPurify** — raw HTML / `<img onerror>` in a
  note body reaches the DOM. A characterization test pins this *current*
  behavior. Accepted as low-risk for a single-user local tool; adding a
  sanitizer is a filed follow-up, not done.
- **Reindex is always a full rebuild.** `fm-core::file::reindex` ignores its
  `_mode` arg and re-reads/re-parses **every** note on each process open (CLI +
  server). The `mtime_ns` column and an `Incremental` variant exist but are
  **unused**. O(n), fine now; gate an incremental path behind a perf-budget test
  before the vault reaches ~1–2k notes.
- **No per-view object cache.** Board/Agenda/Timeline each YAML-parse the whole
  corpus via `load_all` per request. Same scale caveat as above.
- **Inline media buffers whole blobs into memory.** `resolve_asset` returns full
  bytes → a typed `Blob` object URL. Fine for local single-user. Planned
  enhancement: a streaming `GET /api/blob/<hash>` in `fm-serve` (correct
  Content-Type, `Accept-Ranges`, honor `Range`) so `<video>`/`<iframe>`
  range-request instead of buffering.
- **Missing media is a warning, never a crash** — by design. A missing blob
  renders the `.asset-missing-inline` placeholder; don't "fix" it into an error.
- **Board column order is client-side** (`localStorage['fm-board-order']`, keyed
  by group-by) — a view preference, per-browser, **not** in the vault, so it
  doesn't sync across machines. Intentional; the vault-side `.view` file would
  change that (still deferred). Column DnD is mouse-only (like card DnD).

## Deferred (intentionally not built yet)

- Global capture hotkey (was window-only; needs rethinking for the browser).
- `.view` declarative config files (layer-2 extensibility; the renderers are
  hardcoded commands for now).
- Optional mlua scripting hatch.
- **v2:** CM6 live-preview editor, backlinks panel, watched inbox, OCR, video
  posters, semantic search.

## Historical / no longer relevant

- The entire "desktop window rendering / blank WebKitGTK / gui-env / global
  shortcut / custom-protocol / screenshot recipe" saga is **dead** — the window
  was removed 2026-07-15. Ignore those notes for how-to-run; kept only as the
  reason behind the browser pivot ([decisions.md](./decisions.md)).

## Traps for whoever works here next

- **Sandboxed Bash fails** in this environment with a seccomp/`setgroups`
  error. Run shell commands with `dangerouslyDisableSandbox: true`.
- **Toolchain is not on PATH.** `cargo/node/pnpm/mdbook/restic` live in
  `.pixi/envs/default/bin`. Use `pixi run <task>` or
  `pixi run -e default <cmd>`; a bare `cargo`/`pnpm` in a background shell will
  be "command not found".
- **`fm-serve` env vars:** `FM_VAULT` (default `vault`), `FM_UI_DIST` (default
  `ui/dist`), `FM_ADDR` (default `127.0.0.1:8765`), `FM_OPEN` (xdg-open the
  browser), `FM_RESTIC_REPO` / `RESTIC_PASSWORD` (backup).
- **Path traversal** on the static route is rejected with 400 — verify with
  `curl --path-as-is` (plain curl normalizes `../` client-side and hides it).
- **Renderers:** no `todo/doing/done`, no scheduling literals — CI grep
  (`ci/checks.sh`) will fail the build.
- **Do not add fs/db to `fm-query`** — compile-time + CI enforced.
- **Commit discipline:** solo repo, work on `main`, no branch/PR ceremony — but
  commit **only when the user asks**.
- **`vault/` is gitignored** (the knowledge vault is its own repo). Test
  fixtures under `crates/*/tests/fixtures/` are NOT the root `/vault/` and are
  tracked.
