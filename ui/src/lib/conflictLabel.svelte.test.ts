// A conflict message should name the NOTE (its title), not a ULID file path — so the user knows
// which note to open. conflictLabels resolves `notes/<id>.md` to the note's title via the backend.

import { describe, expect, it } from 'vitest';
import { conflictId, conflictLabels } from './conflictLabel';
import * as mock from './mock';

describe('conflict labels', () => {
  it('conflictId extracts the ULID from a conflict path', () => {
    expect(conflictId('notes/01ARZ3NDEKTSV4RRFFQ69G5FAV.md')).toBe('01ARZ3NDEKTSV4RRFFQ69G5FAV');
    expect(conflictId('weird/path.txt')).toBe('weird/path.txt'); // no match → the raw path
  });

  it('names a conflicted note by its title, not its id', async () => {
    const n = await mock.handle<{ id: string }>('capture', { body: 'body' });
    await mock.handle('set_property', { id: n.id, key: 'title', value: 'Trip' });
    expect(await conflictLabels([`notes/${n.id}.md`])).toEqual(['“Trip”']);
  });
});
