// Templates in the mock backend, as the palette's "New from …" actions drive it. A template is
// just a note tagged `template` — the mock filters on the tag exactly as the server's `TagsAll`
// does — and "New from" copies its body into a fresh, untitled note (never another template).

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { NoteDetail, ObjectMeta } from './types';

describe('templates in the mock backend', () => {
  it('lists only notes tagged `template`', async () => {
    const tpl = await mock.handle<ObjectMeta>('capture', { body: '# Weekly\n- [ ] items' });
    await mock.handle<ObjectMeta>('set_property', { id: tpl.id, key: 'tags', value: 'template' });
    await mock.handle<ObjectMeta>('capture', { body: 'an ordinary note' });

    const list = await mock.handle<ObjectMeta[]>('templates', {});
    expect(list.some((n) => n.id === tpl.id)).toBe(true);
    expect(list.every((n) => n.tags.includes('template'))).toBe(true);
  });

  it('a new note from a template carries its body but not the `template` tag', async () => {
    const tpl = await mock.handle<ObjectMeta>('capture', { body: '## Scaffold\ntext' });
    await mock.handle<ObjectMeta>('set_property', { id: tpl.id, key: 'tags', value: 'template' });

    // What "New from template" does: read the template's body, capture a fresh note with it.
    const detail = await mock.handle<NoteDetail>('get', { id: tpl.id });
    const fresh = await mock.handle<ObjectMeta>('capture', { body: detail!.body });

    // A distinct note, and NOT itself a template — only the body travels, never the tag.
    expect(fresh.id).not.toBe(tpl.id);
    expect(fresh.tags.includes('template')).toBe(false);
    // The fresh note's body is the template's body (both read back through the same `get`).
    const reread = await mock.handle<NoteDetail>('get', { id: fresh.id });
    expect(reread!.body).toBe(detail!.body);
  });
});
