import { describe, expect, it } from 'vitest';
import { moveValue, orderColumns } from './boardOrder';

const cols = (...values: string[]) => values.map((value) => ({ value, label: value }));
const vals = (columns: { value: string }[]) => columns.map((c) => c.value);

describe('orderColumns', () => {
  it('returns columns unchanged when the order is empty', () => {
    const c = cols('a', 'b', 'c');
    expect(orderColumns(c, [])).toBe(c);
  });

  it('reorders by the saved order', () => {
    expect(vals(orderColumns(cols('a', 'b', 'c'), ['c', 'a', 'b']))).toEqual(['c', 'a', 'b']);
  });

  it('appends columns not covered by the saved order, in natural order', () => {
    // 'd' is new (not in the saved order) → keeps its place after the ordered ones.
    expect(vals(orderColumns(cols('a', 'b', 'c', 'd'), ['c', 'a']))).toEqual(['c', 'a', 'b', 'd']);
  });

  it('skips saved values that no longer have a column', () => {
    expect(vals(orderColumns(cols('a', 'b'), ['gone', 'b', 'a']))).toEqual(['b', 'a']);
  });

  it('handles the empty-string ("(none)") column value', () => {
    expect(vals(orderColumns(cols('a', ''), ['', 'a']))).toEqual(['', 'a']);
  });
});

describe('moveValue', () => {
  it('moves a value before a target', () => {
    expect(moveValue(['a', 'b', 'c'], 'c', 'a', true)).toEqual(['c', 'a', 'b']);
  });

  it('moves a value after a target', () => {
    expect(moveValue(['a', 'b', 'c'], 'a', 'b', false)).toEqual(['b', 'a', 'c']);
  });

  it('moves a value to the end (after the last)', () => {
    expect(moveValue(['a', 'b', 'c'], 'a', 'c', false)).toEqual(['b', 'c', 'a']);
  });

  it('is a no-op when moving onto itself', () => {
    const v = ['a', 'b', 'c'];
    expect(moveValue(v, 'b', 'b', true)).toBe(v);
  });

  it('is a no-op when the target is absent', () => {
    const v = ['a', 'b'];
    expect(moveValue(v, 'a', 'zzz', true)).toBe(v);
  });
});
