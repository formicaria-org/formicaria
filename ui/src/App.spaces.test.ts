// **The four groups**: a phone's bar is always a group's, its first control lists the groups, a
// step inside a group is not a move and a change of group is — `decisions.md#ui`, 2026-10-12.
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
  expect(screen.queryByRole('navigation', { name: 'Tasks' })).toBeNull();
  expect(screen.queryByRole('list', { name: 'tags' })).toBeNull();
  expect(await screen.findByRole('button', { name: 'open Agenda' })).toBeTruthy();
});

test('the board is in Tasks, and stepping to the agenda is not a move', async () => {
  show('fm-spaces');
  render(App);
  const bar = await screen.findByRole('navigation', { name: 'Tasks' });
  expect(within(bar).getByRole('button', { name: 'Board' }).getAttribute('aria-current')).toBe(
    'page',
  );
  // The board is the board it was: nothing is laid over it (`decisions.md#ui`, 2026-10-12).
  await screen.findByText(/GAE lambda interacts badly/);
  expect(screen.queryByRole('region', { name: 'how the board stands' })).toBeNull();

  const depth = history.length;
  await fireEvent.click(within(bar).getByRole('button', { name: 'Agenda' }));
  await waitFor(() =>
    expect(
      within(screen.getByRole('navigation', { name: 'Tasks' }))
        .getByRole('button', { name: 'Agenda' })
        .getAttribute('aria-current'),
    ).toBe('page'),
  );
  expect(history.length, 'a step inside a group adds no Back step').toBe(depth);
});

test('the first control lists the four groups, and one tap goes to another', async () => {
  seedTrackerDemo();
  show('fm-spaces', 'fm-trackers');
  render(App);
  const bar = await screen.findByRole('navigation', { name: 'Tasks' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'groups' }));
  const list = await screen.findByRole('dialog', { name: 'groups' });
  for (const name of ['Library', 'Tasks', 'Food', 'Fitness'])
    expect(within(list).getByRole('button', { name: new RegExp(`^${name}`) })).toBeTruthy();

  await fireEvent.click(within(list).getByRole('button', { name: /^Library/ }));
  const library = await screen.findByRole('navigation', { name: 'Library' });
  expect(screen.queryByRole('dialog', { name: 'groups' })).toBeNull();
  expect(
    within(library).getByRole('button', { name: 'Timeline' }).getAttribute('aria-current'),
  ).toBe('page');

  // The Library's way in by tag: a tap keeps the notes that carry it.
  const tags = await screen.findByRole('list', { name: 'tags' });
  const first = within(tags).getAllByRole('button')[0];
  await fireEvent.click(first);
  expect(first.getAttribute('aria-pressed')).toBe('true');

  // And on to a group of shipped views, the same way.
  await fireEvent.click(within(library).getByRole('button', { name: 'groups' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'groups' })).getByRole('button', {
      name: /^Food/,
    }),
  );
  expect(await screen.findByRole('navigation', { name: 'Food' })).toBeTruthy();

  // Changing group is a move: Back returns to the group before, one at a time.
  history.back();
  expect(await screen.findByRole('navigation', { name: 'Library' })).toBeTruthy();
  history.back();
  expect(await screen.findByRole('navigation', { name: 'Tasks' })).toBeTruthy();
});

test('the add control in Tasks and Library makes a note', async () => {
  show('fm-spaces');
  render(App);
  const bar = await screen.findByRole('navigation', { name: 'Tasks' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'add to Tasks' }));
  expect(await screen.findByRole('textbox')).toBeTruthy();
});

test('starter templates are notes, and a group’s add control offers the ones that name it', async () => {
  show('fm-spaces');
  render(App);
  const bar = await screen.findByRole('navigation', { name: 'Tasks' });
  await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Add starter templates' }));

  // A template for tasks carries a status, and is not itself a card on the board.
  await waitFor(async () => {
    const list = await handle<ObjectMeta[]>('templates', {});
    expect(list.map((t) => t.title).sort()).toEqual(['Meeting', 'Recipe', 'Task']);
  });
  const board = await handle<Board>('board', { groupBy: 'status' });
  expect(board.columns.flatMap((c) => c.cards).some((c) => c.tags.includes('template'))).toBe(
    false,
  );

  await waitFor(async () => {
    await fireEvent.click(within(bar).getByRole('button', { name: 'add to Tasks' }));
    expect(screen.getByRole('dialog', { name: 'add to Tasks' })).toBeTruthy();
  });
  const sheet = screen.getByRole('dialog', { name: 'add to Tasks' });
  expect(
    within(sheet).queryByRole('button', { name: 'Meeting' }),
    'that one is Library’s',
  ).toBeNull();
  const before = (await handle<ObjectMeta[]>('recent', {})).length;
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Task' }));

  await waitFor(async () => {
    const all = await handle<ObjectMeta[]>('recent', {});
    expect(all.length).toBe(before + 1);
    const made = all.find((n) => n.tags.includes('task') && !n.tags.includes('template'));
    expect(made, 'the note made carries the tag, and is not a template').toBeTruthy();
    expect(
      made!.props.group,
      'which group offers a template is the template’s own',
    ).toBeUndefined();
  });
});
