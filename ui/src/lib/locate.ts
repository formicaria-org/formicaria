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
