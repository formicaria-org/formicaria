import { describe, expect, it } from 'vitest';
import { moveValue, orderColumns, placeValue } from './boardOrder';

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

// Where a dragged card lands inside a column: the drop position the user chose,
// not the backend's created-desc order.
describe('placeValue', () => {
  it('inserts before the named card', () => {
    expect(placeValue(['a', 'b', 'c'], 'c', 'b')).toEqual(['a', 'c', 'b']);
  });

  it('appends when there is no card to land before (dropped past the last one)', () => {
    expect(placeValue(['a', 'b', 'c'], 'a', null)).toEqual(['b', 'c', 'a']);
  });

  it('inserts a card dragged in from another column', () => {
    // The id need not already be in the list — this is the cross-column drop, and
    // it is why one function covers both moving and arriving.
    expect(placeValue(['a', 'b'], 'z', 'b')).toEqual(['a', 'z', 'b']);
    expect(placeValue(['a', 'b'], 'z', null)).toEqual(['a', 'b', 'z']);
  });

  it('appends rather than dropping the card when the target is unknown', () => {
    expect(placeValue(['a', 'b'], 'a', 'gone')).toEqual(['b', 'a']);
  });

  it('never duplicates the card it moved', () => {
    expect(placeValue(['a', 'b', 'c'], 'b', 'a')).toEqual(['b', 'a', 'c']);
  });

  it('holds still when dropped before itself, rather than sliding to the end', () => {
    const v = ['a', 'b', 'c'];
    expect(placeValue(v, 'b', 'b')).toBe(v);
  });
});

// The pure functions above were tested all along — and then the pane rewrite stopped
// importing them: `Pane.svelte` passed `onreorder={() => {}}`, so dragging a column header
// did nothing while the drag still started and the cursor still said `grab`.
//
// A unit test cannot catch a caller that stops calling. What it *can* pin is the sequence
// the caller performs, so that if the wiring is rebuilt it is rebuilt correctly: move within
// the order currently on screen, persist the whole permutation, and re-apply it on render.
describe('the sequence Pane performs when a column is dragged', () => {
  const columns = (...vs: string[]) => vs.map((value) => ({ value, label: value, cards: [] }));

  it('a drag, then a render, shows the new order', () => {
    const server = columns('a', 'b', 'c'); // whatever order the query returned
    const saved = moveValue(
      orderColumns(server, []).map((c) => c.value),
      'c',
      'a',
      true,
    );

    expect(orderColumns(server, saved).map((c) => c.value)).toEqual(['c', 'a', 'b']);
  });

  // The second drag must be computed against what the user is looking at, not against the
  // server's order — otherwise it moves the wrong column.
  it('a second drag composes with the first', () => {
    const server = columns('a', 'b', 'c');
    let saved = moveValue(server.map((c) => c.value), 'c', 'a', true); // c a b
    const visible = orderColumns(server, saved).map((c) => c.value);
    saved = moveValue(visible, 'a', 'b', false); // c b a

    expect(orderColumns(server, saved).map((c) => c.value)).toEqual(['c', 'b', 'a']);
  });

  // A column that appears later (a new status value) must not vanish because it is absent
  // from the saved order.
  it('a column the saved order has never seen still renders', () => {
    const saved = ['b', 'a'];
    const server = columns('a', 'b', 'brand-new');

    expect(orderColumns(server, saved).map((c) => c.value)).toEqual(['b', 'a', 'brand-new']);
  });
});
