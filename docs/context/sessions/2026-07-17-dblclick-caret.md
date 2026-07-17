# 2026-07-17 — Double-click lands the caret where you clicked

**Outcome:** double-clicking the read view opens the editor with the caret on the
word you double-clicked (and the textarea scrolled to centre it), instead of at
offset 0. New pure helper `ui/src/lib/locate.ts` (`countOf` / `nthIndexOf`) +
`locate.test.ts`; `NotePanel.svelte` gains `clickedOffset()` and an optional
`openEditor(at?)`. No backend change, no new API command, no `render.ts` change,
no new dependency. `pixi run ci` green, prod build green.

## Why

The gesture existed but always dropped you at the top of the source — on a note
longer than a screen you then hunt for the sentence you meant to edit. `startEdit`
had the `MouseEvent` in hand and used it only for its target guard.

## The design, and the one we rejected

The rigorous fix is **source positions**: move `render.ts` off the one-shot
`marked.parse()` onto `marked.lexer()` + a custom renderer, accumulate token `raw`
lengths into `data-src-start` attributes, and thread a shift table out of
`extractMath` (which rewrites `$…$` into `<span data-math>` placeholders of a
*different byte length* before marked ever sees the text, so offsets are already
skewed). That is a large, permanent change to the module every view's reading
depends on — disproportionate for a gesture whose worst failure is a misplaced
caret.

Instead we locate by **word ordinal**: a double-click has already selected the
word under the pointer, so count which occurrence of that word it is in the
rendered text and find the same occurrence in `draft` (the literal Markdown, so
the `extractMath` skew is irrelevant — we never touch the parsed text). The whole
mapping is a pure string function, testable with no layout engine.

Two details worth keeping in mind if you touch this:

- **`clickedOffset()` must run before `editing` flips.** The read div and the
  textarea never coexist; the swap destroys the DOM the ordinal is counted in.
- **`caretXY` (`caret.ts`) is reused** to scroll the caret into view — `focus()`
  alone scrolls to the top. It returns zeros under jsdom, so that line is a
  harmless no-op in tests rather than a crash.

## Accepted imprecision (by design, not a bug)

- Text inside KaTeX / Mermaid output is not in the source → no match → caret goes
  to the **end** of the note. Same for the whitespace under a short note, where
  "append" is the right reading of the gesture anyway. Chips/links/media are still
  guarded out and keep their own behavior.
- A word that also occurs in syntax the reader never sees (a URL in a link target,
  a fence's language tag) shifts the ordinal → the caret lands on a *different
  occurrence of the same word*. `nthIndexOf` deliberately returns the **last**
  occurrence rather than -1 when the ordinal overshoots, for the same reason.

Both are strictly better than the old always-offset-0, which is what justifies the
cheap approach.

## Left not-working

**Not verified interactively** — there is no display in this environment to
double-click in. The mapping's pure half is unit-tested; the DOM half (Range
`setEnd` on the selection anchor, the scroll centring) has been reasoned through
but not exercised. Worth one `pixi run serve` pass: a long note, a note with
`$math$` above the click, a chip, and the whitespace under a short note.
