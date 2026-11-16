import { describe, expect, it } from 'vitest';
import { accepted, appended, replaced } from './bridge.svelte';

const self = {};
const other = {};
const O = 'http://127.0.0.1:8765';
const ID = '01JD0000000000000000000000';
const at = (m: unknown) =>
  accepted({ source: self, origin: O, data: { formicaria_bridge: m } }, self, O);

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
    expect(at({ n: 1, type: 'open', id: '../etc' })).toBeNull();
    expect(at({ type: 'insert', text: 'x' })).toBeNull();
    expect(at({ n: 2, type: 'insert', text: '   ' })).toBeNull();
    expect(at({ n: 3, type: 'delete', id: ID })).toBeNull();
    expect(at({ n: 4, type: 'replace', note: ID, from: '', to: 'x' })).toBeNull();
    expect(at({ n: 5, type: 'toast', text: '' })).toBeNull();
  });

  it('reads an insert with its page and its alternative', () => {
    expect(
      at({
        n: 6,
        type: 'insert',
        text: '> a quote',
        what: 'the passage',
        page: { title: 'Ants', url: 'https://a.org/' },
        alt: { label: 'Link only instead', token: '3' },
      }),
    ).toEqual({
      n: 6,
      type: 'insert',
      text: '> a quote',
      what: 'the passage',
      page: { title: 'Ants', url: 'https://a.org/' },
      alt: { label: 'Link only instead', token: '3' },
      quiet: false,
    });
    expect(at({ n: 6, type: 'insert', text: 'x', quiet: true })).toMatchObject({ quiet: true });
    expect(at({ n: 7, type: 'replace', note: ID, from: 'a', to: 'b' })).toEqual({
      n: 7,
      type: 'replace',
      id: ID,
      from: 'a',
      to: 'b',
    });
  });

  it('reads a request to ask the assistant, with its page', () => {
    expect(
      at({
        n: 8,
        type: 'ask',
        text: 'Ants carry leaves.',
        page: { title: 'Ants', url: 'https://a.org/' },
      }),
    ).toEqual({
      n: 8,
      type: 'ask',
      text: 'Ants carry leaves.',
      page: { title: 'Ants', url: 'https://a.org/' },
    });
    expect(at({ n: 9, type: 'ask', text: '' })).toBeNull();
  });

  it('adds a passage after a blank line, whatever the note ends with', () => {
    expect(appended('', 'x')).toBe('x\n');
    expect(appended('a', 'x')).toBe('a\n\nx\n');
    expect(appended('a\n', 'x')).toBe('a\n\nx\n');
    expect(appended('a\n\n', ' x ')).toBe('a\n\nx\n');
  });

  it('replaces what came in, or takes it out again (Undo)', () => {
    const body = appended('Start.', '![x](asset:sha256-ab)');
    expect(replaced(body, '![x](asset:sha256-ab)', '[Image](https://a.org/x.jpg)')).toBe(
      'Start.\n\n[Image](https://a.org/x.jpg)\n',
    );
    expect(replaced(body, '![x](asset:sha256-ab)', '')).toBe('Start.\n');
    expect(replaced('Start.', 'missing', '')).toBeNull();
  });
});
