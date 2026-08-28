// Where in the Markdown source is the word you double-clicked in the read view?
//
// The rendered HTML carries no source positions (marked's one-shot `parse()`
// emits none, and `extractMath` has already shifted the offsets before marked
// even sees the text), so we locate by *ordinal* instead: the 3rd "however" you
// can see is the 3rd "however" in the file. Syntax the reader never sees can
// shift that count, which is why the caller treats the answer as a hint.

/** Non-overlapping occurrences of `word` in `text`. */
export function countOf(text: string, word: string): number {
  if (!word) return 0;
  let n = 0;
  for (let i = text.indexOf(word); i !== -1; i = text.indexOf(word, i + word.length)) n++;
  return n;
}

/**
 * Start offset of the `n`-th (0-based) non-overlapping `word` in `text`, or -1
 * when `word` does not occur at all.
 *
 * Overshooting `n` returns the *last* occurrence rather than -1: an ordinal is
 * only ever as good as the rendered text it was counted in, so when the count
 * disagrees with the source, the nearest real word beats no answer.
 */
export function nthIndexOf(text: string, word: string, n: number): number {
  if (!word) return -1;
  let at = -1;
  for (let i = text.indexOf(word); i !== -1; i = text.indexOf(word, i + word.length)) {
    at = i;
    if (n-- <= 0) break;
  }
  return at;
}

/**
 * Move an offset out of a Markdown link/image **destination**, so nothing can ever be inserted
 * inside one. Returns `index` unchanged when it is already somewhere safe.
 *
 * **This exists because the hint above is wrong often enough to corrupt notes, and it did.** Found
 * on the owner's phone (2026-08-20), where one note had been rewritten to:
 *
 * ```text
 * asset:sha256-a52bb![JPEG_20260807_111802_….jpg](asset:sha256-cd63457f…)
 * ```
 *
 * — a whole second image reference spliced five characters into the first one's hash. The app then
 * logged `asset_status: parse error: not an asset reference` on every startup, and the image was
 * permanently broken with nothing on screen explaining why.
 *
 * The mechanism is the ordinal above being a hint and the caller treating it as a caret. Rendered
 * text and source text disagree by more than "syntax the reader never sees": an image's alt text is
 * an *attribute* and contributes nothing to `textContent` — **unless the blob is missing**, when
 * the placeholder renders it as words; a `note:` chip renders a title where the source has an id;
 * and an embed renders an entire other note's body. So the count drifts by an amount that depends
 * on which blobs happen to be present, the offset lands anywhere, and the next insertion writes
 * into whatever it landed in.
 *
 * Snapping is the right shape of fix rather than making the mapping exact: an exact mapping needs
 * source positions threaded through `marked` and `extractMath`, which is a real piece of work, and
 * a caret that is a few words off is a small annoyance where a caret inside a reference is silent
 * data corruption. This removes the corruption and leaves the annoyance.
 */
export function outsideDestination(text: string, index: number): number {
  for (let open = text.indexOf(']('); open !== -1; open = text.indexOf('](', open + 2)) {
    const start = open + 2;
    const close = text.indexOf(')', start);
    if (close === -1) continue; // unterminated — not a destination we can reason about
    // `start` itself is inside: an insertion there lands between `](` and the destination.
    if (index >= start && index <= close) return close + 1;
    if (start > index) break; // destinations only move rightwards from here
  }
  return index;
}
