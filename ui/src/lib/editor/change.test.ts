import { describe, expect, it } from 'vitest';
import { smallestChange } from './change';

const apply = (was: string, c: { from: number; to: number; insert: string } | null) =>
  c ? was.slice(0, c.from) + c.insert + was.slice(c.to) : was;

describe('the smallest change between two texts', () => {
  it('is nothing when they are the same', () => {
    expect(smallestChange('abc', 'abc')).toBeNull();
  });

  it('touches only what differs, wherever it is', () => {
    // Two insertions are one replacement of what lies between them.
    expect(smallestChange('one two three', 'one **two** three')).toEqual({
      from: 4,
      to: 7,
      insert: '**two**',
    });
    expect(smallestChange('one two', 'one big two')).toEqual({ from: 4, to: 4, insert: 'big ' });
    expect(smallestChange('hello world', 'hello')).toEqual({ from: 5, to: 11, insert: '' });
    expect(smallestChange('', 'new')).toEqual({ from: 0, to: 0, insert: 'new' });
  });

  it('always gets from the one to the other', () => {
    const pairs: [string, string][] = [
      ['aaa', 'aa'],
      ['aa', 'aaa'],
      ['abcabc', 'abc'],
      ['line\r\nline', 'line\nline'],
      ['- [ ] task', '- [x] task'],
      ['x', ''],
    ];
    for (const [was, now] of pairs) expect(apply(was, smallestChange(was, now))).toBe(now);
  });
});
