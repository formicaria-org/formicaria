// The same answer is the same object (`parseReply`, 2026-10-18): what lets a refresh that read back
// what is already on screen draw nothing.
import { describe, expect, it } from 'vitest';
import { parseReply } from './ipc';

describe('parseReply', () => {
  it('hands back the object parsed last time when a whole-notebook read answers the same', () => {
    const a = parseReply('recent', {}, '[{"id":"1"}]');
    expect(parseReply('recent', {}, '[{"id":"1"}]')).toBe(a);
    const b = parseReply('recent', {}, '[{"id":"2"}]');
    expect(b).not.toBe(a);
    expect(b).toEqual([{ id: '2' }]);
  });
  it('keeps answers to different questions apart', () => {
    const one = parseReply('board', { groupBy: 'status' }, '{"columns":[]}');
    expect(parseReply('board', { groupBy: 'tags' }, '{"columns":[]}')).not.toBe(one);
  });
  it('parses everything else afresh', () => {
    const n = parseReply('get', { id: '1' }, '{"id":"1"}');
    expect(parseReply('get', { id: '1' }, '{"id":"1"}')).not.toBe(n);
    expect(parseReply('get', {}, '')).toBeUndefined();
  });
});
