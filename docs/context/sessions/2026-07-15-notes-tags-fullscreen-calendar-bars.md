# 2026-07-15 — Notes+tags (no "type"), full-screen note, calendar bars, remove Gallery

**Outcome:** four owner-requested changes, verified green (`pixi run ci` exit 0 —
29 Rust suites + 61 UI tests, prod UI build, and a live `fm-serve` smoke covering
capture / recent / board / `set_property type=asset` / `delete` + on-disk
frontmatter). Built on top of the delete / media-notice / column-reorder / SVG
session. Sequenced subtractive-first: **F4 → F1 → F2 → F3**. Uncommitted.

## What we implemented

- **F1 — Removed the note "type" concept (kept note vs asset).** Reduced the
  enum to `Kind { Note, Asset }` (`crates/fm-model/src/lib.rs`); dropped
  `Task`/`Meeting`. `FromStr` is now **lenient**: only `"asset"` → `Asset`,
  everything else (incl. legacy `type: task`/`meeting`) → `Note`, so old vault
  notes migrate silently on load. Frontmatter still stores `type: note|asset`
  internally (the asset distinction is load-bearing) — it's just no longer
  user-settable or shown for plain notes. UI: removed the capture-bar type
  picker + `newType`, the editable Type `<select>` in `NotePanel`, the `type`
  group-by suggestion, and the meeting seed in `mock.ts` (now a plain note with a
  `meeting` **tag**); the header/search type pill shows **only for assets**.
  Notes are differentiated by **tags**. Tests: retargeted `Kind::Task` → `Note`
  in the arbitrary spots and → `Asset` in the kind-distinguishing ones
  (fm-core/fm-query search + seam), and reframed the "group by type falsifies the
  thesis" board test around note/asset.
- **F2 — Full-screen note panel.** Repurposed the `NotePanel` `wide` toggle from a
  centered page into a true fill: `.overlay.wide` stretches, `.panel.wide` is
  `100vw × 100vh`, no border/radius, keeps the `page-in` animation. The prose
  column stays centered/readable via the existing `.read { max-width: --measure;
  margin: 0 auto }`. Toggle label/glyph → "Full screen" / "Exit full screen".
- **F3 — Multi-day calendar bars (created → due).** No backend change — `created`
  (RFC3339) and `due` (`YYYY-MM-DD`) already ride on every agenda card. Added
  pure, unit-tested helpers to `ui/src/lib/calendar.ts`: `isoDate` (RFC3339 →
  local day, bare dates untouched so no UTC day-shift), `clampRangeToWeek`
  (intersect a `[start,end]` range with a Monday-first week → 0-based
  `{startCol,endCol,continuesLeft,continuesRight}` via lexicographic ISO
  compares), and `assignLanes` (greedy lane packing so overlapping bars stack).
  `Calendar.svelte` refactored from "cells own pills" into **per-week rows**: a
  day-number strip + a 7-col bars grid where each note is a bar placed by
  `grid-column: startCol+1 / endCol+2` and `grid-row: lane+1`. Bars are flat-ended
  where they run into an adjacent week; still colored by derived `urgency`, keep
  the `hard` ◆, and open on click. Month + week ranges reuse the same code.
- **F4 — Removed the Gallery view.** UI-only: deleted `renderers/Gallery.svelte`,
  the nav item, the `'gallery'` view + its command-palette entry + refresh
  branch, and the `gallery` Icon (keyboard number map is now 1–4). The Rust
  `gallery` command and its tests **stay** (still used by asset tests); the mock
  `gallery` case is harmless. Assets are reached from the notes that link them.

- **Build/run tasks (release + debug).** There was a `[profile.release]` in
  `Cargo.toml` but no task used it — `pixi run serve` was a *debug* `cargo run`.
  Added `pixi run build` (release artifacts: `target/release/fm-serve` **2.7 MB**
  stripped/thin-LTO + `ui/dist`), `build-debug` (`target/debug/fm-serve` ~34 MB),
  and `serve-release` (optimized run). Documented as a new **Build & run** section
  in the root `README.md` (debug vs release, the `FM_*` env knobs) and corrected
  the size estimate (was "~10-25 MB") in `Cargo.toml`/README/overview to the real
  ~3 MB. The `.desktop` launcher still uses `serve` (debug); README notes how to
  switch it to `serve-release`.

## Decisions

- **Reduce the enum, don't add a flag.** Every layer already speaks `Kind`/`type`,
  so dropping `Task`/`Meeting` was deletion, not new plumbing — and lenient
  `FromStr` is the whole migration story (no data rewrite).
- **Keep `asset` as a `Kind`, not a tag.** Assets carry blob refs + extracted
  text and are queried as a class (`Predicate::Kind(vec![Kind::Asset])`) — a real
  kind, unlike the meeting/task labels which were just strings.
- **Calendar bar geometry lives in pure helpers**, mirroring `urgency.ts` /
  `boardOrder.ts` — the renderer stays a thin, literal-free view.

## Notes / gotchas

- A `<input type="date">` syncs `bind:value` on the **input** event but writes on
  **change**; `fireEvent.change` alone (as the old select-based test did) leaves
  the binding stale and writes `''`. The retargeted property-round-trip test in
  `App.features.test.ts` fires **both** input + change (a real date-picker does).
- `clampRangeToWeek`/`weekOf` are timezone-stable: `new Date(y, m-1, d)` yields the
  same civil date in any TZ, and comparisons are on local `ymd` strings — the
  suite passes identically under the dev box (UTC+8) and a UTC CI.
