// **Only a deadline can be overdue** (2026-11-23): a meeting that ended, or a plan replaced by
// another, sat in the agenda and on Home as *overdue*. Invented notes.
import { render, screen } from '@testing-library/svelte';
import { expect, it } from 'vitest';
import { isDeadline, isOverdue, urgencyOf } from './urgency';
import { appToday } from './appTime';
import { addDays } from './civil';
import Agenda from '../renderers/Agenda.svelte';
import type { ObjectMeta } from './types';

const past = addDays(appToday(), -10);
const card = (title: string, over: Partial<ObjectMeta>): ObjectMeta =>
  ({
    id: title.replace(/\W/g, '').padEnd(26, '0').slice(0, 26),
    title,
    preview: '',
    tags: [],
    created: '',
    updated: '',
    type: 'note',
    kind: 'note',
    status: null,
    due: null,
    start: null,
    hard: false,
    ...over,
  }) as unknown as ObjectMeta;

it('a task with a due date is a deadline; a meeting or a plan, with a start, is a span', () => {
  expect(isDeadline({ due: past })).toBe(true);
  expect(isDeadline({ due: past, start: past, status: 'todo' })).toBe(true);
  expect(isDeadline({ due: `${past}T11:00`, start: `${past}T10:00` })).toBe(false);
  expect(isOverdue({ due: past })).toBe(true);
  expect(isOverdue({ due: past, status: 'done' })).toBe(false);
  expect(isOverdue({ due: past, start: addDays(past, -5) })).toBe(false);
  expect(urgencyOf({ due: past, start: addDays(past, -5) })).toBeNull();
});

it('the agenda list: a past meeting is not under Overdue; a missed deadline is', () => {
  render(Agenda, {
    cards: [
      card('Send the draft', { due: past }),
      card('Lab meeting', { start: `${past}T10:00`, due: `${past}T11:00` }),
    ],
    onopen: () => {},
  });
  expect(screen.getByText('Send the draft')).toBeTruthy();
  expect(screen.queryByText('Lab meeting')).toBeNull();
});
