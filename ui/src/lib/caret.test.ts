// **The mirror must be the same size as the textarea it stands in for.**
//
// `caretXY` measures the caret in a hidden copy of the textarea. On an Android phone whose *Font
// size* is not the default, that copy came out bigger than the original — the phone's text zoom is
// already in the computed font size, and copying it applied the zoom a second time — so the caret
// of a long note was measured far below where it was and the editor scrolled past it. No desktop
// browser has a text zoom, which is why this was only ever seen on the owner's phone.
//
// jsdom has neither layout nor text zoom, so what is pinned here is the arithmetic of the
// correction: given what was asked for and what it turned into, what to ask for instead.
import { describe, expect, it } from 'vitest';
import { undoubled } from './caret';

describe('undoing a text zoom that was applied twice', () => {
  it('leaves a desktop browser alone, where what is set is what is rendered', () => {
    expect(undoubled(14, 14)).toBeNull();
    expect(undoubled(22.4, 22.4)).toBeNull();
  });

  it('asks for less by exactly the factor the mirror grew', () => {
    // A phone at 1.3x: the textarea reports 18.2px, and a mirror set to 18.2px renders 23.66px.
    const ask = undoubled(18.2, 23.66)!;
    expect(ask).toBeCloseTo(14, 5); // set this, and the zoom brings it back to 18.2
    expect(ask * 1.3).toBeCloseTo(18.2, 5);
    // The same for a smaller-than-default setting, where the mirror would have come out too small.
    expect(undoubled(11.9, 10.115)! * 0.85).toBeCloseTo(11.9, 5);
  });

  it('gives up rather than divide by nonsense', () => {
    expect(undoubled(NaN, 14)).toBeNull();
    expect(undoubled(14, 0)).toBeNull();
    expect(undoubled(0, 14)).toBeNull();
  });
});
