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

  // The marker-skipping branch had no test at all, which is how a note came to be called
  // “=======” in three user-facing notices without anything going red. A hunk whose *ours* side is
  // empty — one device deleted the sentence, the other rewrote it — puts `=======` first, and the
  // filter skipped only `<<<<<<<` and `>>>>>>>`. Sentence-granular markers make that hunk ordinary.
  it('never names a note after a conflict marker', async () => {
    // `capture` only records a preview — the mock invents a body for anything without an
    // override — so the body has to be written through `update_body` for this branch to run
    // at all. That is why the branch had no test.
    const n = await mock.handle<{ id: string }>('capture', { body: 'placeholder' });
    await mock.handle('update_body', {
      id: n.id,
      body: '<<<<<<< ours\n=======\nNew first sentence.\n>>>>>>> theirs\nRest of it.',
    });
    expect(await conflictLabels([`notes/${n.id}.md`])).toEqual(['“New first sentence.”']);
  });
});
