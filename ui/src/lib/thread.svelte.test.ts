// The discussion mock, driven exactly as the panel drives it. The mock is the dev and test
// backend, so a mock that disagrees with the server is a mock that hides a server bug —
// these assert the behaviours the Rust side is tested for, on the TypeScript side.

import { describe, expect, it, beforeEach } from 'vitest';
import * as mock from './mock';
import type { ObjectMeta, ThreadView } from './types';

const post = (id: string, body: string) => mock.handle<ObjectMeta>('reply', { id, body });
const read = (id: string) => mock.handle<ThreadView>('thread', { id });
const notes = () => mock.handle<ObjectMeta[]>('recent', {});

// A fresh root per test. The mock's store is module-level and has no reset, so every
// assertion below is relative to a count taken inside the test rather than absolute.
let root: string;
beforeEach(async () => {
  root = (await mock.handle<ObjectMeta>('capture', { body: 'the note under discussion' })).id;
});

describe('discussion in the mock backend', () => {
  it('posts a message and reads it back', async () => {
    await post(root, 'first thought');

    const t = await read(root);
    expect(t.count).toBe(1);
    expect(t.messages[0].body).toBe('first thought');
    expect(t.messages[0].depth).toBe(0);
  });

  // The natural "reply to this comment" gesture must join the same discussion rather than
  // start one hanging off a message no view can reach.
  it('re-roots a reply to a message', async () => {
    const first = await post(root, 'first');

    const second = await post(first.id, 'answering the first');

    expect(second.props.thread_of).toBe(`note:${root}`);
    expect(second.props.reply_to).toBe(`note:${first.id}`);
    const t = await read(root);
    expect(t.count).toBe(2);
    expect(t.messages[1].depth).toBe(1);
  });

  // The exclusion the whole feature rests on: `recent()` is the `/` note-picker.
  it('keeps messages out of the note list', async () => {
    const before = (await notes()).length;

    for (let i = 0; i < 20; i++) await post(root, `message ${i}`);

    expect((await notes()).length).toBe(before);
  });

  // Mirrors the server's refusal, which is what stops a board grouped by `thread_of`
  // erasing a note on a single drag.
  it('refuses to set thread structure by hand', async () => {
    for (const key of ['thread_of', 'reply_to']) {
      await expect(
        mock.handle('set_property', { id: root, key, value: 'doing' }),
      ).rejects.toThrow();
    }
  });

  it('refuses an empty message and an unknown target', async () => {
    await expect(post(root, '   ')).rejects.toThrow();
    await expect(post('01ARZ3NDEKTSV4RRFFQ69G5FAV', 'hi')).rejects.toThrow();
  });
});
