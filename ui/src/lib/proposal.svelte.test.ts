// create_proposal in the mock backend, as the app calls it. The mock has no git, so it records the
// proposal *note* the Collaboration feed lists — the same shape the server returns (a note carrying
// `proposes: branch:<name>`), so the UI can be built against it.

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { ObjectMeta } from './types';

describe('create_proposal in the mock backend', () => {
  it('creates a proposal note the Collaboration feed lists, leaving the original', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note to improve' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', { id: note.id, body: 'improved' });

    expect(prop.props?.proposes).toMatch(/^branch:proposal\//);
    expect(prop.id).not.toBe(note.id);

    const list = await mock.handle<ObjectMeta[]>('proposals', {});
    expect(list.some((p) => p.id === prop.id)).toBe(true);
  });

  it('refuses a proposal against a note that does not exist', async () => {
    await expect(mock.handle('create_proposal', { id: 'nope', body: 'x' })).rejects.toThrow();
  });
});
