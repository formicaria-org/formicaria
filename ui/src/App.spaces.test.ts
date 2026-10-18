// **The four groups**: the panel lists them, a group's views are tabs at the top of its window, a
// step inside a group is not a move and a change of group is — and the plus is the app's one
// plus, whose meaning follows the view in front (`decisions.md#ui`, 2026-10-12).
//
// Shown everywhere; a device can still turn it off (`fm-spaces` = 0) and have the chrome it had.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, handle, reset, seedTrackerDemo } from './lib/mock';
import type { Board, ObjectMeta } from './lib/types';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';
import * as spaces from './lib/spaces.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  clearFaults();
  nav.reset();
  trackers.reload();
  spaces.reload();
});
const show = (...keys: string[]) => {
  for (const k of keys) store.set(k, '1');
  trackers.reload();
  spaces.reload();
};
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
  clearUndo();
  spaces.reload();
});

test('a device can turn the group chrome off, and has what it had before', async () => {
  store.set('fm-spaces', '0');
  spaces.reload();
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  expect(screen.queryByRole('group', { name: 'Tasks' })).toBeNull();
  expect(screen.queryByRole('list', { name: 'tags' })).toBeNull();
  expect(await screen.findByRole('button', { name: 'open Agenda' })).toBeTruthy();
});

test('the board is in Tasks, and stepping to the agenda is not a move', async () => {
  render(App);
  const tabs = await screen.findByRole('group', { name: 'Tasks' });
  expect(within(tabs).getByRole('button', { name: 'Board' }).getAttribute('aria-current')).toBe(
    'page',
  );
  // The board is the board it was: nothing is laid over it (`decisions.md#ui`, 2026-10-12).
  await screen.findByText(/GAE lambda interacts badly/);
  expect(screen.queryByRole('region', { name: 'how the board stands' })).toBeNull();

  const depth = history.length;
  await fireEvent.click(within(tabs).getByRole('button', { name: 'Agenda' }));
  await waitFor(() =>
    expect(
      within(screen.getByRole('group', { name: 'Tasks' }))
        .getByRole('button', { name: 'Agenda' })
        .getAttribute('aria-current'),
    ).toBe('page'),
  );
  expect(history.length, 'a step inside a group adds no Back step').toBe(depth);
});

test('a group is opened from the list of views, and Back returns to the one before', async () => {
  seedTrackerDemo();
  render(App);
  await screen.findByRole('group', { name: 'Tasks' });
  const rail = await screen.findByRole('navigation', { name: 'views' });

  await fireEvent.click(within(rail).getByRole('button', { name: 'open Library' }));
  // The Library opens on its shelves; its timeline, with the tags to browse by, is the next tab.
  const library = await screen.findByRole('group', { name: 'Library' });
  await fireEvent.click(within(library).getByRole('button', { name: 'Timeline' }));
  const tags = await screen.findByRole('list', { name: 'tags' });
  const first = within(tags).getAllByRole('button')[0];
  await fireEvent.click(first);
  expect(first.getAttribute('aria-pressed')).toBe('true');

  await fireEvent.click(within(rail).getByRole('button', { name: 'open Food' }));
  expect(await screen.findByRole('group', { name: 'Food' })).toBeTruthy();

  history.back();
  expect(await screen.findByRole('list', { name: 'tags' })).toBeTruthy();
  history.back();
  await waitFor(() =>
    expect(
      within(screen.getByRole('group', { name: 'Tasks' }))
        .getByRole('button', { name: 'Board' })
        .getAttribute('aria-current'),
    ).toBe('page'),
  );
});

test('there is one plus, and what it makes follows the view in front', async () => {
  seedTrackerDemo();
  render(App);
  await screen.findByRole('group', { name: 'Tasks' });
  // Over the board it opens the menu of things to make.
  await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
  expect(await screen.findByRole('menuitem', { name: 'New note' })).toBeTruthy();
  await fireEvent.click(document.querySelector('.menu-backdrop') as HTMLElement);

  // Over a view of Food the same control logs a food — and says so.
  const rail = screen.getByRole('navigation', { name: 'views' });
  await fireEvent.click(within(rail).getByRole('button', { name: 'open Food' }));
  await screen.findByRole('group', { name: 'Food' });
  expect(screen.queryByRole('button', { name: 'make something new' })).toBeNull();
  await fireEvent.click(await screen.findByRole('button', { name: 'add to Food' }));
  expect(await screen.findByRole('dialog', { name: 'log a food' })).toBeTruthy();
  expect(document.querySelectorAll('.plus-btn'), 'one plus, in one place').toHaveLength(1);
});

test('starter templates are notes, and the plus offers a note made from each', async () => {
  render(App);
  await screen.findByRole('group', { name: 'Tasks' });
  await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Add starter templates' }));

  await waitFor(async () => {
    const list = await handle<ObjectMeta[]>('templates', {});
    expect(list.map((t) => t.title).sort()).toEqual(['Meeting', 'Recipe', 'Task']);
  });
  // A template for tasks carries a status, and is not itself a card on the board.
  const board = await handle<Board>('board', { groupBy: 'status' });
  expect(board.columns.flatMap((c) => c.cards).some((c) => c.tags.includes('template'))).toBe(
    false,
  );

  const before = (await handle<ObjectMeta[]>('recent', {})).length;
  await waitFor(async () => {
    if (!screen.queryByRole('menu'))
      await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
    expect(screen.getByRole('menuitem', { name: 'New from “Task”' })).toBeTruthy();
  });
  await fireEvent.click(screen.getByRole('menuitem', { name: 'New from “Task”' }));

  await waitFor(async () => {
    const all = await handle<ObjectMeta[]>('recent', {});
    expect(all.length).toBe(before + 1);
    const made = all.find((n) => n.tags.includes('task') && !n.tags.includes('template'));
    expect(made, 'the note made carries the tag, and is not a template').toBeTruthy();
  });
  // …and it opens with something in it, not as a blank page.
  const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
  await waitFor(() => expect(body.value).toContain('## Steps'));
});
