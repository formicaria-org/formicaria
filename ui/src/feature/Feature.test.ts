// **A feature page fills a tab with one thing** (`decisions.md` 2026-11-17): a note, or a feature with
// its own views as a strip — none of the app's bars, panes or windows around it.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';

beforeEach(() => mock.reset());
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
});

it('a note fills the page, named in the tab', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Field notes', body: 'Ants carry leaves.', props: {} }],
  })) as { id: string }[];
  history.replaceState(null, '', `/f/note/${made.id}`);
  render(Feature);
  expect(await screen.findByText('Ants carry leaves.')).toBeTruthy();
  expect(document.querySelector('.topbar')).toBeNull();
  await vi.waitFor(() => expect(document.title).toBe('Field notes — formicaria'));
});

it('a feature shows its views as a strip, and a card opens its note in a tab of its own', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(Feature);
  const strip = await screen.findByRole('navigation', { name: /views/ });
  const names = [...strip.querySelectorAll('button:not(.feature-name):not(.new-note)')].map((b) =>
    b.textContent?.trim(),
  );
  expect(names).toEqual(['Board', 'Agenda']);
  await fireEvent.click(screen.getByRole('button', { name: 'Agenda' }));
  expect(screen.getByRole('button', { name: 'Agenda' }).getAttribute('aria-current')).toBe('page');
  await fireEvent.click(screen.getByRole('button', { name: 'Board' }));
  await vi.waitFor(() => expect(document.querySelector('.card')).toBeTruthy());
  await fireEvent.click(document.querySelector('.card') as HTMLElement);
  expect(opened.mock.calls[0]?.[0]).toMatch(/^\/f\/note\/[0-9A-Z]{26}$/);
});

// The owner, 2026-11-17: *"I have no idea how to check the fm settings"* — the ant's panel was the
// only door to Settings, and a hidden ant left none.
it.each(['/f/group/Tasks', '/f/group/Library', '/f/collaboration', '/f/agenda'])(
  '%s offers Settings and Help on its own line',
  async (url) => {
    history.replaceState(null, '', url);
    render(Feature);
    const strip = await screen.findByRole('navigation', { name: /views/ });
    const link = (name: string) =>
      [...strip.querySelectorAll('a')].find((a) => a.textContent?.trim() === name);
    expect(link('Settings')?.getAttribute('href')).toBe('/f/settings');
    expect(link('Help')?.getAttribute('href')).toBe('/manual/');
  },
);

// The owner on 0.52: *"how are notes added? The plus disappeared from every view"*.
it('every view offers a new note, opened in a tab of its own', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(Feature);
  await fireEvent.click(await screen.findByRole('button', { name: '＋ New note' }));
  await vi.waitFor(() => expect(opened.mock.calls[0]?.[0]).toMatch(/^\/f\/note\/[0-9A-Z]{26}$/));
});
