// Conflicts in the mock backend, as the Collaboration surface lists them.
//
// A note is in conflict iff its body carries both merge markers — the same rule as the server
// (`commands::conflicts`). Since 2026-07-31 each entry also says **what kind** of conflict it is,
// because the marker rule only ever described one of git's seven: a note deleted on one device and
// edited on another has no markers at all, and telling the user to "keep the text you want" there is
// advice that cannot be followed. The mock cannot invent that kind (it has no git), which is exactly
// why `setConflicts` exists — see `App.conflicts.test.ts`.

import { beforeEach, describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { ConflictInfo, ObjectMeta } from './types';

beforeEach(() => mock.clearFaults());

describe('conflicts in the mock backend', () => {
  it('lists a note with both markers, not a clean one', async () => {
    const bad = await mock.handle<ObjectMeta>('capture', {
      body: 'a\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> theirs\nb',
    });
    await mock.handle<ObjectMeta>('capture', { body: 'a clean note' });

    const list = await mock.handle<ConflictInfo[]>('conflicts', {});
    expect(list.some((c) => c.note.id === bad.id)).toBe(true);
    expect(list.every((c) => c.note.id !== undefined)).toBe(true);
  });

  it('reports a marker conflict as one that has markers', async () => {
    const bad = await mock.handle<ObjectMeta>('capture', {
      body: 'a\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> theirs\nb',
    });
    const list = await mock.handle<ConflictInfo[]>('conflicts', {});
    const found = list.find((c) => c.note.id === bad.id)!;
    // The distinction the UI branches on: with markers, the note itself is where it gets fixed.
    expect(found.has_markers).toBe(true);
    expect(found.code).toBe('UU');
    expect(found.path).toBe(`notes/${bad.id}.md`);
  });

  it('carries an injected marker-less conflict through untouched, and drops it when resolved', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'the template' });
    const dm: ConflictInfo = {
      note,
      path: 'notes/deleted-here.md',
      vault: 'personal',
      code: 'DU',
      what: 'Deleted here, edited on the other device.',
      has_markers: false,
    };
    mock.setConflicts([dm]);

    let list = await mock.handle<ConflictInfo[]>('conflicts', {});
    expect(list.find((c) => c.path === dm.path)?.has_markers).toBe(false);

    await mock.handle('resolve_conflict', { vault: 'personal', path: dm.path, keep: 'theirs' });
    list = await mock.handle<ConflictInfo[]>('conflicts', {});
    // Resolving stops it being unmerged, so the surface must empty by itself rather than needing a
    // separate "mark as done" — the same way the real one drops off git's unmerged list.
    expect(list.find((c) => c.path === dm.path)).toBeUndefined();
  });
});
