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

  it('a group offers the templates that name it', () => {
    const all = [
      { title: 'Task', props: { group: 'Tasks' } },
      { title: 'Meeting', props: { group: 'Library' } },
      { title: 'Loose', props: {} },
    ] as unknown as ObjectMeta[];
    expect(templatesOf(all, 'Tasks').map((t) => t.title)).toEqual(['Task']);
  });

  it('the starter templates carry structure and no number', () => {
    const s = starters('todo');
    expect(s.map((t) => t.title)).toEqual(['Task', 'Meeting', 'Diet program', 'Training program']);
    expect(s.every((t) => t.tags?.includes('template'))).toBe(true);
    for (const t of s)
      for (const v of Object.values(t.props)) expect(typeof v, t.title).toBe('string');
  });
});
