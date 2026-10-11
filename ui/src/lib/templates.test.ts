// What a template hands on, and what it keeps (`decisions.md#ui`, 2026-10-12).
import { describe, expect, it } from 'vitest';
import { draftFrom, starters, templatesOf } from './templates';
import type { NoteDetail, ObjectMeta } from './types';

const note = (over: Partial<NoteDetail>): NoteDetail =>
  ({
    id: 'T1',
    type: 'note',
    title: 'Task',
    preview: '',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-10-12T00:00:00Z',
    updated: '2026-10-12T00:00:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: 'personal',
    body: '',
    version: 'v',
    ...over,
  }) as NoteDetail;

describe('a note made from a template', () => {
  it('gets the body, the tags but for `template`, the status and the template’s own properties', () => {
    const d = draftFrom(
      note({
        body: '- [ ] first',
        tags: ['template', 'task'],
        status: 'todo',
        props: { program: 'food', kcal: 2000 },
      }),
    );
    expect(d).toEqual({
      title: '',
      body: '- [ ] first',
      tags: ['task'],
      props: { program: 'food', kcal: 2000, status: 'todo' },
    });
  });

  it('never gets what said which note the template was, when, or where it is offered', () => {
    const d = draftFrom(
      note({
        start: '2026-10-01',
        due: '2026-10-02',
        props: {
          group: 'Tasks',
          log: 'food',
          start: '2026-10-01',
          due: '2026-10-02',
          thread_of: 'X',
          'conflict-status': 'done',
          half: 1.5,
          flag: true,
        },
      }),
    );
    expect(d.props).toEqual({});
    expect(d.title).toBe('');
  });

  it('a recipe made from a template gets a word of its own, never the template’s', () => {
    const tpl = note({
      tags: ['template', 'recipe'],
      props: { recipe: 'the-templates', servings: 1 },
    });
    const a = draftFrom(tpl).props.recipe;
    const b = draftFrom(tpl).props.recipe;
    expect(typeof a).toBe('string');
    expect(a).not.toBe('the-templates');
    expect(a).not.toBe(b);
  });

  it('a group offers the templates that name it', () => {
    const all = [
      { title: 'Task', props: { group: 'Tasks' } },
      { title: 'Meeting', props: { group: 'Library' } },
      { title: 'Loose', props: {} },
    ] as unknown as ObjectMeta[];
    expect(templatesOf(all, 'Tasks').map((t) => t.title)).toEqual(['Task']);
  });

  it('a starter template has something to see, and carries no target', () => {
    const s = starters('todo');
    expect(s.map((t) => t.title)).toEqual(['Task', 'Meeting', 'Recipe']);
    for (const t of s) {
      expect(t.tags?.includes('template'), t.title).toBe(true);
      // A note made from it must not open as a blank page.
      expect(t.body?.startsWith('# '), t.title).toBe(true);
      expect(t.body, t.title).toMatch(/\n## /);
    }
    // The one number is a recipe's servings, which a recipe cannot be read without.
    const numbers = s.flatMap((t) =>
      Object.entries(t.props).filter(([, v]) => typeof v === 'number'),
    );
    expect(numbers).toEqual([['servings', 1]]);
  });
});
