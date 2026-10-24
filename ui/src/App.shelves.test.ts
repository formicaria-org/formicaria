/// **Home is the Library's shelves** (2026-10-18): a first launch opens on Library → Shelves, notes
/// stand there as books grouped by their `shelf:` tags (2026-10-24), and Library is always the way
/// back to them. The tutorials are ordinary notes on the Tutorials shelf (`fm_app::tutorials`).
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import * as mock from './lib/mock';
import { HOME_VIEW, migrateWorkspace, setFirstView } from './lib/panes';
import shelfSource from './renderers/Shelf.svelte?raw';

const store = new Map<string, string>();
beforeEach(async () => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  mock.clearFaults();
  setFirstView('home'); // what a real first launch opens on; the suite starts on Board
  await mock.handle('create_notes', {
    notes: [
      { title: 'Start here', tags: ['tutorial', 'shelf:Tutorials'], props: {} },
      { title: 'Ask Myrme', tags: ['tutorial', 'shelf:Tutorials'], props: {} },
      { title: 'A loose thought', tags: ['idea'], props: {} },
    ],
  });
});
afterEach(() => {
  setFirstView('board');
  mock.reset();
  vi.unstubAllGlobals();
});

// `HOME_VIEW` is pinned to the shipped file's `name:` on the Rust side (`views.rs`), which reads files.

test('a first launch opens on the shelves: tutorials as books, the rest on Other', async () => {
  render(App);
  const tutorials = await screen.findByRole('region', { name: 'Tutorials' });
  expect(within(tutorials).getByRole('button', { name: 'Start here' })).toBeTruthy();
  expect(within(tutorials).getByRole('button', { name: 'Ask Myrme' })).toBeTruthy();
  const other = screen.getByRole('region', { name: 'Other' });
  expect(within(other).getByRole('button', { name: 'A loose thought' })).toBeTruthy();
  // Only a `shelf:` tag makes a shelf: `tutorial` and `idea` are tags, not shelves.
  expect(screen.queryByRole('region', { name: 'tutorial' })).toBeNull();
  expect(screen.queryByRole('region', { name: 'idea' })).toBeNull();
});

test('a book opens as a note, and Library brings the shelves back', async () => {
  render(App);
  const tutorials = await screen.findByRole('region', { name: 'Tutorials' });
  await fireEvent.click(within(tutorials).getByRole('button', { name: 'Start here' }));
  await waitFor(() => expect(screen.getAllByText('Start here').length).toBeGreaterThan(1));
  const nav = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(within(nav).getByRole('button', { name: /Library/ }));
  expect(await screen.findByRole('region', { name: 'Tutorials' })).toBeTruthy();
});

test('a saved Home (0.9.1) opens as the shelves', () => {
  const { workspace } = migrateWorkspace({
    v: 3,
    cols: 1,
    layout: 'single',
    active: 0,
    panes: [{ id: 'p1', kind: 'home', colSpan: 1, rowSpan: 1 }],
  });
  expect(workspace.panes[0].kind).toBe('view');
  expect(workspace.panes[0].viewName).toBe(HOME_VIEW);
});

test('the shelf renderer names no shelf — every label comes with the view', () => {
  for (const word of ['Tutorials', "'Other'", '"Other"', 'tutorial'])
    expect(shelfSource.includes(word), word).toBe(false);
});
