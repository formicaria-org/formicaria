# 2026-07-15 — Browser-first v2

**Outcome:** shipped the merged "ultraplan" as commit `e57793f`. Verified green:
`pixi run ci` (exit 0), prod `pnpm build`, and a live `fm-serve` curl smoke.

## What we set out to do

Reassess the Rust+Tauri implementation against the vision (organize notes /
ideas / meetings; a front-end with agenda, kanban, search, render files incl.
multimedia; git backup; graceful missing-media). The native window stayed blank
on the user's machine, so the decision was made to **remove the window and make
the browser the product**, then add asset authoring + inline media, a real "New
note" flow, a beautiful space-efficient UI, a double-click launcher, and
rendered docs. Two plans (a local refinement pass + a downloaded remote
`/ultraplan`) were **merged locally** and executed as WS-A..E.

## What we implemented, by workstream

- **WS-A — retire the window.** Deleted the fm-app binary, `build.rs`,
  `tauri.conf.json`, `capabilities/`, `icons/`; fm-app is now a library. Dropped
  the pixi `gui` env, the `e2e/` tree, and the deny.toml Tauri waivers
  (cargo-deny stays green). `fm-serve` holds the store lock across `commit`.
- **WS-B — asset authoring + inline media.** `blob::put_bytes` (one-shot sha256,
  dedup-before-write, `.incoming-<pid>-<seq>` temp names), `ingest::ingest_bytes`
  + `sniff_mime`, `commands::ingest`, a binary `POST /api/ingest?name=` upload
  (413 over 512 MB). Editor: drag-file-to-attach + Notion-style `/` slash menu
  (FTS → insert asset). `render.ts` → typed object URLs, native element per MIME
  (img / scrollable-PDF iframe / video / audio / link). "New note" opens the
  editor.
- **WS-C — the UI.** Three-layer design tokens (scales → dark+warm-light
  semantics → legacy aliases = zero-renderer-edit reskin), persisted light/dark
  toggle, sidebar+main shell (composer distinct from search, `aria-current` nav,
  icon rail <900px), right-docked NotePanel side-sheet with an "Open wide" page
  mode + measure-capped reading column, lazy ⌘K command palette + keyboard map,
  shared `Icon`/`EmptyState`, Agenda grouped under urgency bands (labels from
  `urgency.ts`). Hover/elevation token-ized across Board/Card/Gallery/Timeline.
- **WS-D — launcher.** `packaging/formicarium.{sh,desktop}` double-click into the
  browser via `FM_OPEN`; terminal path stays `pixi run serve`.
- **WS-E — docs.** mdBook manual (`docs/src`, user+dev+reference, incl. "how to
  add a feature" + extensibility how-tos), built in CI via the `docs` task, plus
  a root `README.md`. `MASTERPLAN.md` updated to browser-first.

## Decisions made (see decisions.md for the durable versions)

- Browser is the product; window removed — the reversal that shaped everything.
- Merge strategy: reconcile local ⊕ remote plans locally (no origin connected to
  the assistant, by the user's choice); the remote's Phase 0 was already
  satisfied, its "keep the window" assumption was overridden.
- A new command touches **4** places now (no Tauri wrapper): `commands.rs` →
  `fm-serve` `api()` arm → `mock.ts` → `ipc.ts`.

## What was left not-working / deferred

Recorded in [../known-issues.md](../known-issues.md): unsanitized innerHTML,
always-full reindex, no object cache, whole-blob-in-memory media. Deferred:
global hotkey, `.view` files, virtualization, and the v2 list.

## Notable gotchas hit

- Sandboxed Bash needed `dangerouslyDisableSandbox`.
- `$TMPDIR` was empty in a subshell → temp paths resolved to `/…`; use the
  scratchpad path or `pixi run` (cargo/pnpm aren't on the base PATH).
- Traversal test only shows 400 with `curl --path-as-is`.
