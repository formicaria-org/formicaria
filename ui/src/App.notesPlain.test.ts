/// **Notes without shelves or tag chips** (`decisions.md#ui`, 2026-11-25, *the shelves go*): the
/// Shelves view and the row of tags over the Timeline were unused — finding a note is typing in
/// search. Notes opens on its Timeline, and the Timeline is the notes alone.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test } from 'vitest';

import App from './App.svelte';
import * as mock from './lib/mock';
import { migrateWorkspace, setFirstView } from './lib/panes';
import sections from './lib/sections.json';

beforeEach(async () => {
  mock.clearFaults();
  setFirstView('timeline');
  await mock.handle('create_notes', {
    notes: [
      { title: 'Start here', tags: ['tutorial', 'shelf:Tutorials'], props: {} },
      { title: 'A loose thought', tags: ['idea'], props: {} },
    ],
  });
});
afterEach(() => {
  setFirstView('board');
  mock.reset();
});

test('Notes has no Shelves view, and opens on its Timeline', () => {
  const notes = sections.find((s) => s.key === 'notes')!;
  expect(notes.views.some((v) => 'view' in v && v.view === 'Shelves')).toBe(false);
  expect(notes.views[0]).toEqual({ builtin: 'timeline' });
});

test('the Timeline shows the notes with no row of tags above them', async () => {
  render(App);
  expect(await screen.findByText('A loose thought')).toBeTruthy();
  expect(screen.queryByRole('list', { name: 'tags' })).toBeNull();
});

test('a saved Home from 0.9.1 opens as the Library timeline', () => {
  const { workspace } = migrateWorkspace({
    v: 3,
    cols: 1,
    layout: 'single',
    active: 0,
    panes: [{ id: 'p1', kind: 'home', colSpan: 1, rowSpan: 1 }],
  });
  expect(workspace.panes[0].kind).toBe('timeline');
});
