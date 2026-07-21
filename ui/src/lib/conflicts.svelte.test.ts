// Conflicts in the mock backend, as the Collaboration surface lists them. A note is in conflict iff
// its body carries both merge markers — the same rule as the server (`commands::conflicts`).

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { ObjectMeta } from './types';

describe('conflicts in the mock backend', () => {
  it('lists a note with both markers, not a clean one', async () => {
    const bad = await mock.handle<ObjectMeta>('capture', {
      body: 'a\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> theirs\nb',
    });
    await mock.handle<ObjectMeta>('capture', { body: 'a clean note' });

    const list = await mock.handle<ObjectMeta[]>('conflicts', {});
    expect(list.some((n) => n.id === bad.id)).toBe(true);
    expect(list.every((n) => n.id !== undefined)).toBe(true);
  });
});
