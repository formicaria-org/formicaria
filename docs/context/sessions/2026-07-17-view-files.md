# 2026-07-17 — `.view` files: saved queries, any renderer

**Outcome:** a `.view` file (YAML, `vault/views/*.view`, git-tracked) defines `query + a
renderer` and shows up in the sidebar. Parsed server-side → `fm_query::Query`; the UI sends a
name. `pixi run ci` green (6 new `fm-app` tests + full suite), driven end-to-end through the
served build: `list_views` surfaces a good view and a broken one with its parse error, and
re-running a board view after tagging a note reflects the change (1→2 cards).

This is `MASTERPLAN.md:323`'s *"five generic renderers = query + a renderer (`.view` config
files remain planned)"* — Phase 3 of the UI track, and the payoff of the workspace exploration
that chose de-modalize + `.view` over a pane grid.

## What landed

- **`crates/fm-app/src/views.rs`** — the whole feature's core. `Renderer` enum; a `ViewFile`
  DTO deserialized from YAML; a `PredDto` (struct-of-optionals, not a serde enum — YAML enums
  are ugly to hand-write and this lets a bad conjunct be named precisely); `lower()` →
  `fm_query::Query` extending a per-renderer `base()`; `list_views(vault)` and
  `run_view(store, vault, name)`. 6 tests.
- **`fm-serve`** — `list_views` (aggregates across vaults; a view is git-tracked *in* its
  vault, the query runs against the whole `MultiStore`) and `run_view` (reads the file from
  whichever vault holds it, runs against the store, surfaces the parse error as the 500 body).
- **UI** — `types.ts`/`ipc.ts`/`mock.ts` (`ViewInfo`/`ViewResult`, `listViews`/`runView`) and
  an *additive* `App.svelte` integration: `views`/`activeView` state, a sidebar list under the
  built-in nav, and a stage branch that renders a view through the **same** renderers as the
  built-ins. `refresh()` re-runs the active view; `viewTitle` shows its name.
- **Docs** — a user-facing schema page (`docs/src/user/views.md` — the file *is* the language,
  so its docs are its UX), `decisions.md`, `overview.md` (23→25 commands), this entry.

## Load-bearing decisions (full text in decisions.md)

- **No `Query` on the wire.** Server-side parse, name on the wire. Avoids serde on
  `fm-query`/`fm-model` (where `Object.vault`'s absence from the wire is a security property)
  and keeps `PropertyValue`'s variant-order `Ord` trap unreachable.
- **The DSL has no ordered `prop` comparison** — `eq`/`ne`/`exists` only; dates via `date:`
  (`DateRange`). The `Ord` trap is structurally unreachable, not just discouraged.
- **`.view` extends, never replaces** — `Vec::extend` onto the renderer's base, so `Kind(Note)`
  (assets exclusion) can't be forgotten and a board view stays notes-only.
- **A broken view is named with its error, never dropped.**
- This gives the engine's previously-unreachable predicates (`Not`/`Any`/`TagsAll`/`TagsAny`/
  `DateRange`) their first production callers.

## Left not-working / deliberately small

- **The sidebar/stage rendering is unverified by eye** (jsdom has no geometry — the same limit
  as the de-modalize work). The data path, the parser, and the served API are all verified; the
  *visual* — the view list in the rail, clicking one, the stage swapping — needs a human at
  `pixi run serve`.
- **Custom board views: cross-column status drag works; within-column ordering does not** (the
  `cardOrders` localStorage is keyed for the built-in board only). A deliberate v1 cut.
- **`base()` duplicates the five preset filters** that also live in `commands.rs`. Not yet
  unified into a shared `presets` module — flagged so `Kind(Note)`/`"done"` don't drift; the
  right refactor is to have `commands::{board,agenda,…}` call `views::base`, later.
- **Duplicate view names across vaults**: both list; `run_view` picks the first. Fine for now.
- `docs/src/reference/commands.md` is stale (missing the vault + view commands) — a
  pre-existing gap from the create-vault work, not chased here.

## Next (per plan.md)

The UI track's three phases (sanitize, de-modalize, `.view`) are done. Remaining forward work
is **Track V's V2** — the four live co-tenancy bugs (two silent) that gate converting a repo
into a vault — and Track S / Track C items. None of it is UI.
