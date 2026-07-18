# 2026-07-17 — Sanitize the read view; de-modalize the note trail

**Outcome:** two UI-track changes, both shipped, `pixi run ci` green (149 UI tests, full
Rust suite, seams, docs). (1) `render.ts` sanitizes untrusted note bodies with DOMPurify —
the top security item in `known-issues.md`, closed. (2) The note trail stopped being a modal
overlay and became a peer grid column: the board stays live beside an open note.

This session began as *"a customizable multi-pane workspace — pick the number of splits, any
view in any cell."* Three independent designs plus an adversarial critic were run against the
codebase. **The critic won**, and the reason reframed the whole thing:
`MASTERPLAN.md:323` — *"Five generic renderers = query + a renderer (`.view` config files
remain planned)"* — means the ask **is already the deferred spec**. So: don't build a bespoke
layout engine that pre-empts `.view` and gets built twice. Split the ask into *layout*
(de-modalize) and *content* (`.view`, a later phase), and the hard part — putting `Query` on
the wire, which would expose the `PropertyValue` variant-order `Ord` trap to a client and
force serde onto `fm-model` where `Object.vault`'s absence is load-bearing — **disappears**,
because a `.view` is parsed server-side and the UI sends a name.

## Sanitizer (Phase 1)

`render.ts` now runs `DOMPurify.sanitize` on `marked`'s output before `innerHTML`. The trap
this had to avoid: three URI schemes are **ours** and are **not** in DOMPurify's default
allow-list — `note:`, `asset:`, `sha256:` — so a naive call strips them and every note chip
and asset image silently vanishes. The config widens the URI regexp by exactly those three
and nothing else. Math (`span[data-math]`) and Mermaid (`code.language-mermaid`) placeholders
survive because the resolve passes run *after* sanitize. Mermaid's own SVG sink keeps relying
on `securityLevel: 'strict'` (documented in place; DOMPurify would drop its `foreignObject`).
Tests: the old characterization test that *pinned* the unsafe behaviour is flipped to assert
stripping; added a scheme-preservation test and a math/mermaid-survival test. `fm-serve`'s
"single user" header and the `known-issues.md` entry were corrected. DOMPurify ships as a
~10 KB-gz chunk.

## De-modalize (Phase 2)

`.trail` is the **third column** of the `.app` grid, a peer of `.main`. Deleted `.overlay`
(`position:fixed; z-index:50`) and `.backdrop`; `NotePanel` lost `100vh`/`100vw` for `100%`.
The grid is three custom-property columns (`--rail`/`--main`/`--trail`) so rail-collapse and
the trail compose without a `grid-template-columns` explosion; the `<900px` media query sets
`--rail` rather than re-declaring the whole template. `wide` (persisted, `fm-note-wide`) now
means "note takes the whole content area" vs "docks beside the view". Closing: the ✕ button
or Escape (`NotePanel.onPaneKey`, focus-gated by `ownsKeys()`) — the backdrop's
click-outside-to-close is gone, which is the point. Two App tests closed the note via the
backdrop's `aria-label="close note"`; updated to the panel's own `aria-label="close"`, which
reflects the real, intended close path.

## Decisions recorded (decisions.md)

Two new entries at the top: the sanitizer, and *"the note trail is a peer column, not a modal
overlay"* — the latter carries the **rejected pane grid** and its stopping line (*"the content
area holds the current view and, when open, the note trail beside it — two regions, fixed. A
pane cannot contain a pane. A saved arrangement is a `.view` file."*).

## Left not-working / deliberately out

- **The layout is unverified by eye.** `pixi run ci` does not render geometry (jsdom has no
  layout; the e2e tree was deleted). The *functional* note flow is tested (open → edit →
  close, 149 UI tests) and the built bundle carries `trail-open`/`trail-wide`, but whether it
  *looks* right — docked width, wide fill, the board staying interactive beside it — needs a
  human at `pixi run serve`. This is the known cost of a layout change here, not an oversight.
- **Keyboard left conservative.** `App.onGlobalKey`'s `if (openIds.length) return` still
  suppresses `1/2/3`/`c`/`/` while a note is open, so they don't drive the view beside it.
  Making them focus-aware is the pane-grid rabbit hole the design rejected.
- **`onMove`/`onReorder` still read the global `groupBy`.** Left alone on purpose: the bug is
  only reachable with two boards on screen (the rejected pane grid), so fixing it now is
  speculative work for a feature we chose not to build.

## Next

Phase 3 — **`.view` files** (`vault/views/*.view`, YAML — `serde_yaml_ng` already parses
frontmatter, so zero new deps; a deliberate deviation from MASTERPLAN's TOML). Parsed
server-side → `fm_query::Query`; the UI sends a name. `.view` **extends** the presets via
`Vec::extend`, never replaces; `Kind(Note)` stays in Rust. This gives the boolean-complete
engine's unreachable predicates (`Not`/`Any`/`TagsAll`/`TagsAny`/`DateRange`) their first
production callers.
