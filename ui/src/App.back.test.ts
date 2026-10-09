// **Back walks what you opened, and panels close first** — `decisions.md` 2026-10-09.
//
// The phone's Back key reaches the page as the browser's own Back (`popstate`): Tauri's `AppPlugin`
// calls `goBack()` while the WebView has history, and closes the app only when it has none. So these
// drive `history.back()` and check where the app lands.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset } from './lib/mock';
import * as nav from './lib/nav.svelte';

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
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
});

/** The view the bar marks as showing — a hidden window stays mounted, so this is what counts. */
const showing = () =>
  screen.getByRole('navigation', { name: 'open views' }).querySelector('[aria-current="page"]')
    ?.textContent ?? '';
const noteOpen = () => (/GAE|lambda/i.test(showing()) ? true : null);

test('Back from a note returns to the board it was opened from', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await waitFor(() => expect(noteOpen()).toBeTruthy());

  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('Back walks several steps in order: agenda → note → board', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await waitFor(() => expect(noteOpen()).toBeTruthy());
  await fireEvent.click(await screen.findByRole('button', { name: 'open Agenda' }));
  await waitFor(() => expect(showing()).toMatch(/Agenda/));

  history.back();
  await waitFor(() => expect(noteOpen()).toBeTruthy());
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('Back closes an open panel first, and stays where it was', async () => {
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  await fireEvent.click(screen.getByLabelText('settings'));
  const dialog = await screen.findByRole('heading', { name: 'Settings' });
  expect(dialog).toBeTruthy();

  history.back();
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Settings' })).toBeNull());
  expect(screen.getByText(/GAE lambda interacts badly/)).toBeTruthy();
});

test('Back first ends editing, then leaves the note', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  const plus = await screen.findByRole('button', { name: 'note options' });
  await fireEvent.click(plus);
  const opts = await screen.findByRole('dialog', { name: 'note options' });
  await fireEvent.click(within(opts).getByRole('button', { name: 'Edit' }));
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();

  history.back();
  await waitFor(() => expect(screen.queryByLabelText('note body (Markdown)')).toBeNull());
  expect(noteOpen()).toBeTruthy();
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('the first screen is stamped, so there is nowhere earlier inside the app', async () => {
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  expect((history.state as { fm?: number; nav?: { kind: string } }).nav?.kind).toBeTruthy();
});

test('a view picked from a menu stays picked, and Back returns from it', async () => {
  // The other way a menu leads somewhere. Closing the menu must not step back onto the place just
  // left (the v0.6.2 phone bug, there through the window list).
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Agenda' }));
  await waitFor(() => expect(showing()).toMatch(/Agenda/));
  await new Promise((r) => setTimeout(r, 100));
  expect(showing()).toMatch(/Agenda/);
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});
