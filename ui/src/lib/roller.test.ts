import { describe, expect, it } from 'vitest';
import { notch, place, rolling, settle, STEP } from './roller';

describe('the roller', () => {
  it('puts the middle item highest and largest, and its neighbours down the arc on either side', () => {
    const mid = place(0);
    expect(mid).toMatchObject({ x: 0, y: 0, rotate: 0, scale: 1, opacity: 1 });
    const left = place(-1);
    const right = place(1);
    expect(left.x).toBe(-STEP);
    expect(right.x).toBe(STEP);
    expect(left.y).toBe(right.y);
    expect(left.y).toBeGreaterThan(0);
    expect(left.scale).toBeLessThan(1);
    expect(left.rotate).toBe(-right.rotate);
    expect(place(4).opacity).toBe(0);
  });

  it('lands on the nearest item when let go, never past the ends', () => {
    expect(settle(2, 0, 5)).toBe(2);
    expect(settle(2, -STEP, 5)).toBe(3);
    expect(settle(2, STEP * 0.4, 5)).toBe(2);
    expect(settle(2, STEP * 0.6, 5)).toBe(1);
    expect(settle(0, STEP * 3, 5)).toBe(0);
    expect(settle(4, -STEP * 9, 5)).toBe(4);
    expect(settle(0, 0, 0)).toBe(0);
  });

  it('follows the finger while rolling, a little past the ends only', () => {
    expect(rolling(1, -STEP / 2, 4)).toBeCloseTo(1.5);
    expect(rolling(0, STEP * 5, 4)).toBe(-0.4);
    expect(rolling(3, -STEP * 5, 4)).toBe(3.4);
  });

  // `known-issues.md`, *A gesture control must not remember what you did last*: an arbitrary,
  // lopsided run of notches lands exactly where its sum says.
  it('moves one item per notch, either way, whatever came before', () => {
    const run = [1, 1, -1, 1, 1, 1, -1, -1, 1, -1, -1, -1, 1];
    let at = 3;
    let want = 3;
    for (const d of run) {
      at = notch(at, d * 120, 10);
      want = Math.max(0, Math.min(9, want + d));
      expect(at).toBe(want);
    }
    expect(notch(0, -3, 10)).toBe(0);
    expect(notch(9, 3, 10)).toBe(9);
    expect(notch(4, 0, 10)).toBe(4);
  });
});
