# 2026-07-18 — One command surface, and blobs that stream

**Outcome:** Track M's host-side band, executed. Three things landed, all of which are
desktop wins today and none of which need a phone to be worth having:

1. **`fm_app::dispatch` — the one command surface** (Track M ruling 1). `fm-serve`'s
   `api()` match, the `Vaults` state and its lock discipline moved into `fm-app`;
   `crates/fm-app/src/vaults.rs` moved up with them. `fm-serve` is now an HTTP shell:
   parse → `dispatch(cmd, args, body, app, host)` → frame the `Output`.
2. **`GET /api/blob/<reference>`** (ruling 7) — streamed from disk, sniffed
   `Content-Type`, `Accept-Ranges`, real `Range`. The UI's inline media points at it;
   object URLs survive only for the mock backend. Closes the top item in
   `known-issues.md`.
3. **The 3-second poll stopped being O(n²)** — the deletion sweep tested membership
   against a `Vec`. 453 ms → 50 ms per quiet beat at 10k notes, pinned by a perf test.

`pixi run ci` green. Verified against a running server, not just tests (see below).

## Why

The plan sequences Track M as **spikes first, host-side, no phone** — and the phone half
is blocked here anyway: no Android SDK/NDK on this machine, and the plan's own toolchain
section calls provisioning it the top longevity risk and a break with the pixi-only rule.
The owner scoped this session to the host-side band with no new dependencies, which
excludes the `git2` swap (spike ii) and the PAT clone spike (iii).

Ruling 1 goes first because it *precedes every mobile milestone*: until there is one
door, every new frontend is a new copy of the dispatch table. `fm-cli` is the standing
proof — it re-implements against `fm-core` rather than calling `commands`, so the surface
had already forked once.

## What the design got wrong, and what changed

An adversarial audit (four parallel code audits, each refuted by a second agent) caught
three things worth recording:

- **`dispatch(&mut Vaults)` — the plan's own proposed signature — would have been a
  regression.** Five arms deliberately drop the lock before slow I/O; `backup_status`
  shells out to `git ls-remote` per vault, and holding the lock across that stalls every
  3 s `ping` behind a network round trip. `App` owns the `Mutex` instead. (The comment
  claiming this also starved the shutdown watchdog was stale — liveness is refreshed
  before dispatch. Corrected in place rather than copied forward.)
- **Folding query params onto the JSON body is a confused deputy.** `ingest`'s body is
  the raw file, so a `.json` asset containing `{"vault":"lab"}` would have been read as
  its own arguments and filed itself into another audience. The rule is now **either,
  never both**: a query string means the body is payload. Verified live — a JSON upload
  naming another vault lands where the query says.
- **A blob at a navigable same-origin URL is a security change, not just a speed one.**
  Note bodies (so their blobs) arrive from collaborators; an SVG or HTML attachment
  rendered as a top-level document runs its script in the app's origin, where the whole
  `/api` surface is in reach. Hence `X-Content-Type-Options: nosniff` always, and
  `Content-Disposition: attachment` for everything outside an inline-safe allowlist
  (`image/*` minus SVG, `video/*`, `audio/*`, `application/pdf`). Subresource loads
  ignore the disposition, so inline SVG images still render.

## The cold-start reindex switch was rejected

The plan also wanted `FileStore::named` to open `Reindex::Incremental` instead of `Full`.
**Not done, deliberately** — the audit made the case against it and the code agrees:

- mtime-only detection is blind to every mtime-preserving writer (`restic restore`,
  `rsync -a`, `cp -p`, `tar -x`). The full rebuild at open is currently the *only* thing
  that heals them, and this repo's own restic restore preserves mtimes. "Restore a
  backup, open the app" is a real flow that would silently stop working.
- It needs an index-format version gate that nothing in CI enforces, plus an
  `objects(path)` index — `forget_path` full-scans today, so Incremental can be *slower*
  than Full after a big pull.
- Cold-start tests for any of this do not exist.
- The benefit is **mobile-only**: Android kills backgrounded apps so every relaunch pays
  a full rebuild. Desktop starts once.

What was hiding underneath was a real live bug: the deletion sweep's `Vec::contains` made
every quiet 3 s beat O(notes²) — ~10⁸ string comparisons on a 10k vault, three times a
minute, to conclude nothing had been deleted. Fixed, measured, and pinned.

## A CI hole, found by falling into it

`pixi run ci` did not typecheck the UI. `vite build` strips types and emits, so a
`.svelte` file calling a function it never imported **builds clean** and throws
`ReferenceError` in the browser. That is exactly the bug this session introduced —
`streamsBlobs`/`assetUrl` used in `NotePanel` without the import — and it passed `pixi run
ci`, passed Vitest, and shipped into `ui/dist`. It was caught only by grepping the built
bundle for the new route and finding it absent.

`ui/package.json` had a `check` script (`svelte-check`) all along; nothing ran it. It is
now a pixi task (`check-ui`) in the `ci` gate. Confirmed it catches this class: 2 errors,
non-zero exit. The tree is otherwise clean (0 errors, 1 pre-existing CSS warning).

## Verified

Not just green tests — driven against a real `pixi run serve` over the actual vault:

- blob route: `200` + `Content-Type: image/png` (the MIME `resolve_asset` used to throw
  away), `Accept-Ranges`, `nosniff`; `Range: bytes=0-9` → `206`, `Content-Range: bytes
  0-9/48531`, exactly 10 bytes; absent hash → `404`.
- command surface through the shell: `list_vaults`, `ping`, `recent`, `board?groupBy`,
  unknown-command → `500` with the message, and `ingest` over the query-args path
  (created an asset note in the named vault).
- the confused-deputy probe above.
- `fm-serve` gained its first HTTP-layer tests (real sockets, port 0): whole-blob,
  range, 416, SVG-is-a-download, absent-is-404.

## Left not-working

- **Not seen in a browser.** The extension is not connected, so inline media rendering
  through the new URL is verified at the HTTP layer and by `svelte-check`, not by eye.
  The `<video>` seek path in particular has never been exercised by a real media element.
- **`?kind=thumb` is not served by the route** — full blobs only. `resolveAsset(ref,
  'thumb')` has no callers in the UI today, so thumbs stay on the byte path.
- **Duplicate-`id:` notes make the poll flap forever** (new `known-issues.md` entry) —
  found by the audit, not fixed.
- **No `Host` header validation** in `fm-serve` (pre-existing DNS-rebinding gap, now
  recorded).
- **Track M M0–M8 remain blocked** on the Android toolchain; spikes (ii) `git2` merge and
  (iii) PAT clone are untouched, being new-dependency and network work respectively.
- One stray `r/` git repo appeared in the repo root during the first full test run and
  did not reproduce on two subsequent `pixi run ci` runs. Deleted; cause unidentified.
