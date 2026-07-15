# 2026-07-15 — Note delete, media copy-notice, column reordering

**Outcome:** three follow-up features, verified green (`pixi run ci` exit 0, 50
UI tests, live `fm-serve` delete smoke). Built on top of commit `067e80b`.

## What we implemented

- **Delete a note (double-confirm).** `Store::delete` already existed at fm-core;
  wired it through the 4 places — `commands::delete` (`crates/fm-app/src/commands.rs`),
  a `"delete"` arm in `fm-serve`, `deleteNote` in `ipc.ts`, a `delete` case in
  `mock.ts`. UI: a **Delete** action in the `NotePanel` header that arms an
  in-panel confirm strip ("…permanently? This can't be undone. Cancel · Delete")
  — the first click never deletes. Confirm → `deleteNote` → `onsaved` (git
  auto-commit) → `onclose` (refresh). Rust tests in `crates/fm-app/tests/note.rs`;
  a mock-backed UI test in `App.features.test.ts`.
- **Media-drop copy notice.** The drop already copied bytes into the
  content-addressed blob store (`vault/blobs/sha256/ab/cd/<hash>`, original
  untouched, deduped). Added a transient `notice` in `NotePanel.onDrop` —
  "Copied <file> into the vault" / "Copied N files…" — auto-clears after 3s,
  styled like App's `.banner.notice`. (User chose "blob store + copy notice",
  not a separate human-readable media folder.)
- **Kanban column reordering.** Column order was previously emergent (query-engine
  first-seen order) and unpersisted. Added a generic pure helper
  `ui/src/lib/boardOrder.ts` (`orderColumns`, `moveValue`) + tests; App holds a
  per-`groupBy` order in `localStorage['fm-board-order']` (mirrors the theme
  pattern) and derives `displayBoard`. `Board.svelte` makes the `.column-head` a
  Pragmatic-DnD drag handle; the column drop-target disambiguates card drops
  (`source.data.id`) from column drops (`source.data.columnValue`) and computes
  the insert edge from the pointer X vs the column midpoint — **no hitbox
  dependency** (respects "no new UI runtime dependency"). Kept literal-free
  (CI grep). Moving a *card* between columns already changed status — unchanged.

## Decisions

- **Column-order persistence = `localStorage` per group-by**, not a vault file.
  It's a view preference, not note data; the vault-side `.view` mechanism stays
  deferred.
- **Delete confirm is in-panel**, not `window.confirm` (on-brand, accessible;
  the header button's accessible name is the aria-label "delete note" while the
  final confirm button's name is "Delete" — that distinction is what the UI test
  keys on).

## Notes / gotchas

- The **"GAE notes" math-not-rendering** report was investigated separately:
  KaTeX 0.17.0 renders the note's math in a browser DOM and all KaTeX assets
  serve correctly from `fm-serve` — most likely a stale server/cache. Left as
  "restart `pixi run serve` + hard-reload; report the exact symptom if it
  persists." No code change.
- Vitest prints a harmless node-level `localStorage is not available` warning;
  jsdom provides `localStorage`, tests pass.
