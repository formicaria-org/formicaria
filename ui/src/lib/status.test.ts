import { describe, expect, it } from 'vitest';
import { nextStatus } from './status';

// The values here are fixtures standing in for "whatever this vault happens to
// use" — the point of every case below is that the function never knows them.
describe('nextStatus', () => {
  const known = ['todo', 'doing', 'done'];

  it('advances through the vault’s statuses in order', () => {
    expect(nextStatus('todo', known)).toBe('doing');
    expect(nextStatus('doing', known)).toBe('done');
  });

  it('passes through unset after the last one, then wraps', () => {
    // Rotating must be able to reach "no status" without a separate clear button.
    expect(nextStatus('done', known)).toBe(null);
    expect(nextStatus(null, known)).toBe('todo');
  });

  it('makes a full cycle and returns to where it started', () => {
    // The cycle is every status *plus* unset, so a full lap is known.length + 1.
    let s: string | null = 'todo';
    const seen: (string | null)[] = [s];
    for (let i = 0; i <= known.length; i++) seen.push((s = nextStatus(s, known)));
    expect(seen).toEqual(['todo', 'doing', 'done', null, 'todo']);
  });

  it('moves rather than sticking when the current value is not yet known', () => {
    // A status this browser hasn't seen in a fetch: a click must still do something.
    expect(nextStatus('archived', known)).toBe('todo');
  });

  it('stays unset when the vault has no statuses at all', () => {
    expect(nextStatus(null, [])).toBe(null);
  });

  it('is not confused by a vault with a single status', () => {
    expect(nextStatus(null, ['todo'])).toBe('todo');
    expect(nextStatus('todo', ['todo'])).toBe(null);
  });
});
