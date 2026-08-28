import { describe, expect, it } from 'vitest';
import { countOf, nthIndexOf, outsideDestination } from './locate';

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

// **The caret must never land inside a link or image destination.**
//
// Built from a real corruption on the owner's phone (2026-08-20). One note had been rewritten to
// `asset:sha256-a52bb![JPEG_….jpg](asset:sha256-cd63457f…)` — a whole second image reference
// spliced five characters into the first one's hash — and the app logged
// `asset_status: parse error: not an asset reference` on every startup thereafter, with a
// permanently broken image and nothing on screen to explain it.
//
// The cause is `clickedOffset` treating `locate`'s ordinal as a caret. The ordinal is a hint and
// says so; what was missing was any floor under how wrong it may be.
describe('outsideDestination', () => {
  const REF = '![figure](asset:sha256-abcdef0123456789)';

  it('leaves an offset in ordinary prose alone', () => {
    const t = `Some prose here.\n\n${REF}\n\nMore prose.`;
    expect(outsideDestination(t, 5)).toBe(5);
    expect(outsideDestination(t, t.length - 3)).toBe(t.length - 3);
  });

  it('pushes an offset inside the hash out past the reference', () => {
    // Exactly the phone's case: five characters into the hash.
    const at = REF.indexOf('asset:sha256-') + 'asset:sha256-'.length + 5;
    const out = outsideDestination(REF, at);
    expect(out).toBe(REF.length);
    // The property that matters: inserting there cannot land inside the destination.
    const spliced = REF.slice(0, out) + '![second](asset:sha256-ffff)' + REF.slice(out);
    expect(spliced.startsWith(REF)).toBe(true);
    expect(spliced).not.toMatch(/asset:sha256-[0-9a-f]*!\[/);
  });

  it('guards both boundaries of the destination', () => {
    const open = REF.indexOf('](') + 2; // first character of the destination
    const close = REF.lastIndexOf(')');
    expect(outsideDestination(REF, open)).toBe(REF.length);
    expect(outsideDestination(REF, close)).toBe(REF.length);
    // Just past the closing paren is already safe and must not move.
    expect(outsideDestination(REF, close + 1)).toBe(close + 1);
  });

  it('leaves the label editable', () => {
    // Inside `[...]` is prose, not a reference — inserting there is harmless and should not snap.
    const inLabel = REF.indexOf('figure') + 2;
    expect(outsideDestination(REF, inLabel)).toBe(inLabel);
  });

  it('picks the right reference when a note has several', () => {
    const a = '![one](asset:sha256-aaaa)';
    const b = '![two](note:M0CK0000000000000000000001)';
    const t = `${a} and then ${b} end`;
    const insideB = t.indexOf('note:') + 6;
    expect(outsideDestination(t, insideB)).toBe(t.indexOf(b) + b.length);
    const between = t.indexOf(' and then ') + 3;
    expect(outsideDestination(t, between)).toBe(between);
  });

  it('does not hang or move on an unterminated destination', () => {
    const t = '![broken](asset:sha256-abc';
    expect(outsideDestination(t, t.length - 2)).toBe(t.length - 2);
  });
});
