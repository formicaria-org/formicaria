import { describe, expect, it } from 'vitest';
import { countOf, nthIndexOf } from './locate';

// The pair backs the double-click-to-edit caret: count a word's ordinal in the
// rendered text, find that ordinal in the source. Both halves must agree on what
// "occurrence" means, so the overlap and overshoot cases below are the contract.
describe('countOf', () => {
  it('counts non-overlapping occurrences', () => {
    expect(countOf('the cat sat on the mat', 'the')).toBe(2);
    expect(countOf('one two three', 'four')).toBe(0);
  });

  it('does not double-count overlapping candidates', () => {
    // 'aaa' holds one non-overlapping 'aa', not two — nthIndexOf strides the
    // same way, so an ordinal counted here is findable there.
    expect(countOf('aaa', 'aa')).toBe(1);
    expect(countOf('aaaa', 'aa')).toBe(2);
  });

  it('counts nothing for an empty word', () => {
    expect(countOf('anything', '')).toBe(0);
  });
});

describe('nthIndexOf', () => {
  const text = 'the cat sat on the mat with the hat';

  it('finds the first, middle and last occurrence', () => {
    expect(nthIndexOf(text, 'the', 0)).toBe(0);
    expect(nthIndexOf(text, 'the', 1)).toBe(15);
    expect(nthIndexOf(text, 'the', 2)).toBe(28);
  });

  it('returns -1 for a word that is not there', () => {
    // The caller reads this as "no answer" and falls back to the end of the note.
    expect(nthIndexOf(text, 'dog', 0)).toBe(-1);
    expect(nthIndexOf(text, '', 0)).toBe(-1);
  });

  it('falls back to the last occurrence when n overshoots', () => {
    // Rendered text and Markdown source disagree often enough that an ordinal is
    // a hint: the nearest real word beats sending the caret nowhere.
    expect(nthIndexOf(text, 'the', 99)).toBe(28);
  });

  it('strides past overlaps exactly as countOf does', () => {
    expect(nthIndexOf('aaaa', 'aa', 1)).toBe(2);
  });
});
