// **What a computer had lost, back on every feature page** (`ui-audit-2026-11-22.md`, U1): search (a
// field in the bar, Ctrl+K and `/` everywhere), Undo after a delete — in the tab the person lands on —
// Ctrl+Z, *Recently deleted*, and tab titles that name the page.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';
import { rememberDeleted, recentDeleted, forgetDeleted } from '../lib/lastDeleted';
import { clear } from '../lib/undo.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  mock.reset();
  clear();
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
});
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function note(title: string, body = 'Some words.') {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title, body, props: {} }],
  })) as { id: string }[];
  return made.id;
}

it('a feature page has a search field; what it finds opens in a tab', async () => {
  const id = await note('Leafcutter colonies', 'They farm fungus.');
  history.replaceState(null, '', '/f/group/Library');
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(Feature);
  const field = await screen.findByRole('searchbox', { name: 'search notes' });
  await fireEvent.input(field, { target: { value: 'leafcutter' } });
  const results = await screen.findByRole('region', { name: 'search results' });
  await vi.waitFor(() => expect(results.textContent).toContain('Leafcutter colonies'));
  await fireEvent.keyDown(field, { key: 'Enter' });
  expect(opened.mock.calls[0]?.[0]).toBe(`/f/note/${id}`);
});

it('Ctrl+K and / open the search over a note’s page; Escape puts it away', async () => {
  const id = await note('Field notes');
  history.replaceState(null, '', `/f/note/${id}`);
  render(Feature);
  await screen.findByText('Some words.');
  expect(screen.queryByRole('dialog', { name: 'search notes' })).toBeNull();
  await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
  const box = await screen.findByRole('dialog', { name: 'search notes' });
  const field = within(box).getByRole('searchbox', { name: 'search notes' });
  await fireEvent.keyDown(field, { key: 'Escape' });
  expect(screen.queryByRole('dialog', { name: 'search notes' })).toBeNull();
  await fireEvent.keyDown(document.body, { key: '/' });
  expect(await screen.findByRole('dialog', { name: 'search notes' })).toBeTruthy();
});

it('a delete in a tab that closed is offered back, with Undo, on the tab in front', async () => {
  const id = await note('Old plan');
  const gone = (await mock.handle('delete', { id })) as { id: string; vault: string; file: string };
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  await screen.findByRole('searchbox', { name: 'search notes' });
  // The note's tab writes the delete down and closes: this tab hears it through the storage.
  rememberDeleted({ ...gone, title: 'Old plan' });
  window.dispatchEvent(
    new StorageEvent('storage', { key: 'fm-last-deleted', newValue: store.get('fm-last-deleted') }),
  );
  const toast = await vi.waitFor(() => {
    const t = screen
      .getAllByRole('status')
      .find((e) => e.textContent?.includes('Deleted “Old plan”'));
    if (!t) throw new Error('no toast');
    return t;
  });
  await fireEvent.click(within(toast).getByRole('button', { name: 'Undo' }));
  await vi.waitFor(async () =>
    expect(((await mock.handle('get', { id })) as { id: string } | null)?.id).toBe(id),
  );
  expect(recentDeleted(60000)).toBeNull();
});

it('Ctrl+Z outside the text puts back the last note deleted', async () => {
  const id = await note('Shopping');
  const gone = (await mock.handle('delete', { id })) as { id: string; vault: string; file: string };
  rememberDeleted({ ...gone, title: 'Shopping' });
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  await screen.findByRole('searchbox', { name: 'search notes' });
  await fireEvent.keyDown(document.body, { key: 'z', ctrlKey: true });
  await vi.waitFor(async () =>
    expect(((await mock.handle('get', { id })) as { id: string } | null)?.id).toBe(id),
  );
  forgetDeleted();
});

it('Recently deleted is in the feature chooser', async () => {
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  await fireEvent.click(
    await screen.findByRole('button', { name: 'switch this tab to another feature' }),
  );
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Recently deleted…' }));
  expect(await screen.findByRole('dialog', { name: 'Recently deleted' })).toBeTruthy();
});

it.each([
  ['/f/new', 'New tab — formicaria'],
  ['/f/settings', 'Settings — formicaria'],
  ['/f/agenda', 'Agenda — formicaria'],
])('%s is named in its tab', async (url, title) => {
  history.replaceState(null, '', url);
  render(Feature);
  await vi.waitFor(() => expect(document.title).toBe(title));
});
