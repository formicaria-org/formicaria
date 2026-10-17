/// **Home: the first screen, and the place to come back to** (2026-10-18). A newcomer chose Agenda
/// from the first list they saw and then asked to "go back to the home page" — they did not know
/// they could start writing or move between views.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults } from './lib/mock';
import { setFirstView } from './lib/panes';
import { closeManual } from './lib/manual.svelte';

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
  // What a real first launch opens on; the rest of the suite starts on Board (`test-setup.ts`).
  setFirstView('home');
});
afterEach(() => {
  setFirstView('board');
  closeManual();
  vi.unstubAllGlobals();
});

test('a first launch opens on Home, which says what to do in a few words', async () => {
  render(App);
  expect(await screen.findByRole('heading', { name: 'Your notebook' })).toBeTruthy();
  for (const h of [
    'Write something',
    'See your notes your way',
    'Keep them safe',
    'Come back here',
  ])
    expect(screen.getByRole('heading', { name: h })).toBeTruthy();
});

test('Home is first in the list of views, and its buttons go where they say', async () => {
  render(App);
  const nav = await screen.findByRole('navigation', { name: 'views' });
  const first = within(nav).getAllByRole('button')[0];
  expect(first.textContent).toMatch(/Home/);

  await fireEvent.click(await screen.findByRole('button', { name: /Agenda/ }));
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Your notebook' })).toBeNull());

  // And back: Home is always there to return to.
  await fireEvent.click(first);
  expect(await screen.findByRole('heading', { name: 'Your notebook' })).toBeTruthy();
});

test('Home opens the manual for anything longer', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /Show me more/ }));
  const frame = (await screen.findByTitle('The manual')) as HTMLIFrameElement;
  expect(frame.getAttribute('src')).toBe('/manual/user/first-note.html');
});
