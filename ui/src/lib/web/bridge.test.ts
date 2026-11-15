import { describe, expect, it } from 'vitest';
import { accepted, appended } from './bridge.svelte';

const self = {};
const other = {};
const O = 'http://127.0.0.1:8765';
const ID = '01JD0000000000000000000000';

describe('the browser bridge', () => {
  it('acts only on what this window posted, from its own origin', () => {
    const data = { formicaria_bridge: { n: 1, type: 'open', id: ID } };
    expect(accepted({ source: self, origin: O, data }, self, O)).toEqual({
      n: 1,
      type: 'open',
      id: ID,
    });
    // A page formicaria opened can post to this window, but from its own.
    expect(accepted({ source: other, origin: O, data }, self, O)).toBeNull();
    expect(accepted({ source: self, origin: 'https://evil.example', data }, self, O)).toBeNull();
  });

  it('refuses malformed requests', () => {
    const at = (m: unknown) =>
      accepted({ source: self, origin: O, data: { formicaria_bridge: m } }, self, O);
    expect(at({ n: 1, type: 'open', id: '../etc' })).toBeNull();
    expect(at({ type: 'insert', text: 'x' })).toBeNull();
    expect(at({ n: 2, type: 'insert', text: '   ' })).toBeNull();
    expect(at({ n: 3, type: 'delete', id: ID })).toBeNull();
    expect(at({ n: 4, type: 'insert', text: '> a quote' })).toEqual({
      n: 4,
      type: 'insert',
      text: '> a quote',
    });
  });

  it('adds a passage after a blank line, whatever the note ends with', () => {
    expect(appended('', 'x')).toBe('x\n');
    expect(appended('a', 'x')).toBe('a\n\nx\n');
    expect(appended('a\n', 'x')).toBe('a\n\nx\n');
    expect(appended('a\n\n', ' x ')).toBe('a\n\nx\n');
  });
});
