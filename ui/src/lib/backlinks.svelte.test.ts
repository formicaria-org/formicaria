// Backlinks in the mock backend, driven as the "Linked from" panel drives it. The mock scans note
// bodies for `note:<target>` — the same reverse pass as the server (`commands::backlinks`) — so
// dev and CI see what production does.

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { ObjectMeta } from './types';

describe('backlinks in the mock backend', () => {
  it('lists notes that reference the target, never the target itself', async () => {
    const target = await mock.handle<ObjectMeta>('capture', { body: 'a target note' });
    const linker = await mock.handle<ObjectMeta>('capture', {
      body: `see [it](note:${target.id})`,
    });
    await mock.handle<ObjectMeta>('capture', { body: 'an unrelated note' });

    const back = await mock.handle<ObjectMeta[]>('backlinks', { id: target.id });
    expect(back.map((n) => n.id)).toEqual([linker.id]);
    expect(back.some((n) => n.id === target.id)).toBe(false);
  });
});
