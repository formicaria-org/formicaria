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
  const names = [
    ...strip.querySelectorAll('button:not(.feature-name):not(.new-note):not(.view-as button)'),
  ].map((b) => b.textContent?.trim());
  expect(names).toEqual(['Board', 'Agenda']);
  await fireEvent.click(screen.getByRole('button', { name: 'Agenda' }));
  expect(screen.getByRole('button', { name: 'Agenda' }).getAttribute('aria-current')).toBe('page');
  await fireEvent.click(screen.getByRole('button', { name: 'Board' }));
  await vi.waitFor(() => expect(document.querySelector('.card')).toBeTruthy());
  await fireEvent.click(document.querySelector('.card') as HTMLElement);
  expect(opened.mock.calls[0]?.[0]).toMatch(/^\/f\/note\/[0-9A-Z]{26}$/);
});

// Help from every view. Settings is the window's, at the right end of the browser's tab row, on every
// tab (the owner, 2026-11-18: *"why not simply add settings on top right … why embed it only on one
// page?"*; `browser/kit/formicaria.cfg`, checked by `browser-check`) — and the page's own only while
// the window cannot show it (2026-11-19: after an update the owner found Settings nowhere).
const linkIn = (strip: HTMLElement, name: string) =>
  [...strip.querySelectorAll('a')].find((a) => a.textContent?.trim() === name);

it.each(['/f/group/Tasks', '/f/group/Library', '/f/collaboration', '/f/agenda'])(
  '%s offers Help, and leaves Settings to a window that has it',
  async (url) => {
    mock.setBrowserRunning(true, true);
    history.replaceState(null, '', url);
    render(Feature);
    const strip = await screen.findByRole('navigation', { name: /views/ });
    // The answer about the window has come (a tick), and still no Settings of its own.
    await new Promise((r) => setTimeout(r, 0));
    expect(linkIn(strip, '⚙ Settings')).toBeUndefined();
    expect(linkIn(strip, 'Help')?.getAttribute('href')).toBe('/manual/');
  },
);

it.each([
  ['formicaria’s browser still on an older kit', true, false],
  ['another browser showing the page', false, false],
])('Settings is on the page itself when the window has none: %s', async (_, running, kit) => {
  mock.setBrowserRunning(running, kit);
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  const strip = await screen.findByRole('navigation', { name: /views/ });
  await vi.waitFor(() =>
    expect(linkIn(strip, '⚙ Settings')?.getAttribute('href')).toBe('/f/settings'),
  );
});

// The owner on 0.52: *"how are notes added? The plus disappeared from every view"*.
it('every view offers a new note, opened in a tab of its own', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(Feature);
  await fireEvent.click(await screen.findByRole('button', { name: '＋ New note' }));
  await vi.waitFor(() => expect(opened.mock.calls[0]?.[0]).toMatch(/^\/f\/note\/[0-9A-Z]{26}$/));
});

// **Leaving a page never closes the browser** (2026-11-19). Settings opened on its own (restored at
// start: one entry in its history) closed the window when it was left, and a darkened layer over the
// whole page, the update notice included, left it on any click. The owner: *"the update lines were
// out but not clickable, as I tried to click on a random place it crashed"*.
it('Settings on its own page: no layer over the page, and leaving it goes to the notes', async () => {
  const went: string[] = [];
  const closed = vi.spyOn(window, 'close').mockImplementation(() => {});
  vi.spyOn(history, 'length', 'get').mockReturnValue(1);
  history.replaceState(null, '', '/f/settings');
  render(Feature, { props: { go: (u: string) => went.push(u) } });
  const x = await screen.findByRole('button', { name: 'close' });
  expect(screen.queryByRole('button', { name: 'close settings' })).toBeNull();
  await fireEvent.click(x);
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(closed).not.toHaveBeenCalled();
  expect(went[0]).toBe('/f/home');
});

it('a note opened on its own, closed, goes to the notes, not out of the browser', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Alone', body: 'One entry in its history.', props: {} }],
  })) as { id: string }[];
  const went: string[] = [];
  const closed = vi.spyOn(window, 'close').mockImplementation(() => {});
  vi.spyOn(history, 'length', 'get').mockReturnValue(1);
  history.replaceState(null, '', `/f/note/${made.id}`);
  render(Feature, { props: { go: (u: string) => went.push(u) } });
  await screen.findByText('One entry in its history.');
  const close = document.querySelector<HTMLButtonElement>(
    'button[aria-label="close"], button[title="Close"]',
  );
  if (close) await fireEvent.click(close);
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(closed).not.toHaveBeenCalled();
});
