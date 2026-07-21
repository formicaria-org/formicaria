# 2026-07-21 — Rich editing: read-view tap-widgets + a select-to-format toolbar

The owner wanted to edit "in the nicer rendered version," not just plain Markdown. A **3-agent
analysis** (simplicity/clarity · modularity/maintainability · longevity/efficiency, each reading the
actual seams) set the frame, and it held all the way through:

- The load-bearing constraint is **byte-for-byte round-trip** (`MASTERPLAN.md:327`). It splits the
  space cleanly: **Tier 1a** = surgical span patches from *closed* vocabularies (byte-safe, ship);
  **Tier 1b** = free-text structural edits like table cells ("Tier 3 in costume" — a re-serializer;
  do only constrained + fixture-gated); **Tier 2** = a CodeMirror live-preview editor (a real v2
  project, forks the read path); **Tier 3** = re-serializing WYSIWYG (breaks the round-trip *and* the
  `.md` merge driver — rejected).
- Tiers 1 and 2 are **orthogonal, not a ladder** — Tier 1 buys nothing toward CM6.
- Never `contenteditable` (a second sanitize sink); never a widget *registry* (the no-plugin-API
  line — data SELECTS behaviour from a closed set, never SUPPLIES it); locate by an **exact token
  ordinal**, not `locate.ts`'s deliberately-fuzzy word-ordinal.

## What shipped (all CI-green, both devices)

Read-view **tap to change what's there**:
- **Checkboxes** (`5965658`) — tap a GFM `- [ ]` to flip the `[ ]`↔`[x]` byte; strike-through on done.
- **Callout-type picker** (`721128e`) — each callout renders a type badge; tap it → pick from the
  closed `CALLOUT_TYPES`, rewriting just the `[!type]` word.
- **Status picker** (`d272cbd`) — kept rotate as the primary tap; added a picker on `contextmenu`
  (right-click / long-press, one event for both platforms). Extracted `clickOutside` to a shared action.
- **Table cells** (`3d7dc80`) — the constrained Tier-1b: tap a cell → an input overlays it, prefilled
  with the cell's *source* text; commit rewrites only that cell. Mapped by table ordinal × (header/
  body row, `cellIndex`); **bails** (leaves the cell read-only) on anything it can't map exactly
  (non-bordered rows, ragged rows, a column past the source); a typed `|` is escaped, newlines
  flattened, so a table can't break. Fixture tests: round-trip byte-exact + pipe-escape.

Editor **select to author** — a floating toolbar over the selection:
- Inline wraps (`92ff940`): Bold/Italic/Highlight/Code, **Colour** (closed `TEXT_TOKENS` →
  `[…]{.token}`), Link — a real toggle (tap again to unwrap). `pointerdown` is prevented on the bar
  so a press keeps the textarea selection (the whole trick).
- **¶ block menu** (`fe02578`): Heading 1/2/3 (re-levels), bullet/numbered lists, quote, callout
  (drops a `[!note]` the read-view badge then retypes — the two features chain).

Every edit is a plain-Markdown/closed-vocab byte patch — no new HTML sink (the CI sink-grep stays
green), no re-serialization, no plugin surface, no `contenteditable`.

Shared discipline across all widgets: **embedded notes' widgets stay inert** (another file's atom is
read-only in an embed) and are excluded from the ordinal counts.

## Honest caveat recorded

The read-view toggles update the tapped element **in place**, but `saveTask` reassigns `note`, so the
read `$effect` (`NotePanel.svelte:240`, reads `note.body`) **does re-render the whole note** once the
save resolves (re-running KaTeX/Mermaid). Correct, but not the "no re-render" the checkbox commit
claimed. Truly decoupling render-source from save-state (a `renderSrc` the effect reads, updated only
on load / edit-exit) is a deferred refinement — not done mid-arc to avoid churning the core render path.

## Tutorial note

A single interactive tutorial note lives in the `vault` (`01KY20GXD54…`, "📖 Tutorial — editing",
status `doing`): live checkboxes, all six callouts, an editable table, the toolbar walkthrough, the
status-on-the-card note, and links/embeds/geo — the one place a user can learn every gesture by doing.
Created/maintained via the **server API** (never the CLI on a live vault — the reindex/commit lessons).

## Deferred (named, not started)
- Tier 2 CodeMirror live-preview editor (v2). Tag/date chips in the read view. Section folding / drag
  reorder. Decoupling render-source from save-state (the re-render refinement above).
