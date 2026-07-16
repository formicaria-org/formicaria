# 2026-07-16 — Assets out of views · status chip · kanban drop position · caret `/` menu · edit gesture

**Outcome:** five owner-reported frictions, each fixed where it belonged.
`board`/`agenda`/`recent` now filter to `Kind::Note` in the **query layer**, so an
ingested PDF never gets a card again. A **StatusChip** rotates a note's status
through the vault's own values from the board, the timeline, or the open note. A
card dropped on the board **stays where you dropped it**. The `/` menu opens **at
the caret** and seeds with your **recent notes**. And a note opens its editor on
**double-click** as well as from the **Edit button**, with Ctrl+S to save.

`pixi run ci` exit 0, `svelte-check` 0 errors, `pnpm -C ui build` clean, 119 UI
tests (12 new). Verified beyond headless with a **live `fm-serve` against the
fixture vault**: `recent`/`board`/`agenda` return `types: ['note']`, a board
grouped by `type` has only a `note` column, and `search` still returns the asset
by its extracted caption text — the exclusion holds without making a PDF
unfindable.

## Decisions the owner made

- **Asset exclusion is backend, not per-view.** One predicate in the query layer
  applies to every view including ones not written yet. `search` and `gallery`
  deliberately still see assets — that is how you find a PDF, and it is what lets
  the `/` menu insert an asset reference at all.
- **Card order is a view preference → `localStorage`**, keyed by group-by and
  column value (`fm-card-order`), exactly mirroring the existing
  `fm-board-order`. Rejected: an `order` property in frontmatter — it would churn
  a note file on every drag for something that is not knowledge.
- **The status cycle is learned from the vault**, never a literal. `nextStatus`
  walks `App.knownStatuses` (already collected by `learnStatuses`) plus unset, so
  the chip works for any workflow and the `ci/checks.sh` literal grep stays green.
- **The Edit button stays on every note; double-click is *additive*.** The owner
  first asked for the button to go ("the only button on top right, apart from
  close and resize, is the delete"), then **reversed it mid-session**: "edit
  should be there everywhere, and double click is just a redundant way of pushing
  it without going there". So the gesture is a shortcut, never the only way in.
  On a board the button is genuinely the only way — Excalidraw's canvas owns
  double-click — which is why it reads `Details` there.

## Things worth knowing (found while building)

- **`set_property(id, "type", "asset")` flips `obj.kind`** (`fm-core/src/edit.rs:23`),
  and `Object::get` maps `"type"` → the kind. So the old
  `the_same_board_grouped_by_type_falsifies_the_thesis` test was asserting the
  behavior this change removes. It is now `the_board_shows_notes_only_never_assets`;
  the "group by any property" claim it used to carry is still covered by
  `board_by_a_custom_property_works_and_props_flow_through`.
- **The `(none)` column in `e2e_vault.rs` only existed because of the asset** —
  every fixture note has a status. That assertion was inverted, not deleted.
- **Every pane in the trail mounts its own `<svelte:window onkeydown>`.** Naively
  adding Escape-exits-edit meant pane 1 closed the trail while you were leaving
  pane 2's editor. `ownsKeys()` fixes it: when focus is inside *some* pane, only
  that pane acts; with focus outside all of them they all act (the old behavior).
  Any future pane-scoped shortcut must go through the same gate.
- **The timeline row was a `<button>`**, so the chip (also a button) could not
  nest inside it — invalid HTML. It is now `div role="button"`, the pattern
  `Card.svelte` already used. Its `type` pill was also dead the moment assets left
  `recent` (it could only ever read "note"), so the chip took that slot.
- **`placeValue(v, id, id)`** — dropping a card before itself — would have
  teleported it to the end via the append fallback. Unreachable from a drag
  (`dropBefore` skips the dragged card) but guarded and tested anyway.
- **`.read` was content-height inside a `height: 100vh` flex column**, so a short
  note left a tall dead zone that *looks* like the note but is the `.panel`
  behind it. Double-clicking there hit nothing — the owner reported exactly this
  and no test could see it (jsdom has no layout, so every element is 0×0 and
  `fireEvent.dblClick` on `.read` always "works"). Fixed with `flex: 1 0 auto`.
  **Any click-anywhere gesture needs the target to actually fill its area** —
  headless will never tell you it doesn't.
- **The owner was running a stale binary.** `pixi run app` (the desktop launcher)
  runs the **prebuilt** `target/release/fm-serve` and never recompiles, so the
  asset filter looked unimplemented for an hour. `pixi run serve` rebuilds; the
  icon does not. Run `pixi run build` before asking the owner to check anything.

## Left not-working / deliberately not done

- **The caret-anchored `/` menu is unverified in headless.** `caretXY` uses the
  mirror-div technique and **jsdom has no layout**, so it returns zeros there and
  the menu degrades to the editor's top-left. The measurement itself only proves
  out in a real browser — the owner confirms visually.
- **Card order is per-browser** and does not sync, same caveat as column order.
- The `/` menu still has no scroll-into-view for the active row when arrowing
  past the visible few (pre-existing; results are capped at 8).
- `docs/src/user/notes.md` still documents a **Capture box and a type dropdown**
  that no longer exist. Pre-existing staleness, untouched here beyond the edit
  gesture and status rows — worth a pass of its own.
