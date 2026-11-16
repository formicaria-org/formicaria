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
  const names = [...strip.querySelectorAll('button:not(.feature-name)')].map((b) =>
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
