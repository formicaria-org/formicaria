/// **The same controls, placed by the space available.**
///
/// Wide: a panel down the left, holding everything, so the workspace gets back the height a top
/// bar used to take. Narrow: the same element as a bar along the bottom, because a top bar on a
/// phone holds actions the thumb cannot reach and nearly every app ships one anyway.
///
/// What can be tested here is the *state*, not the geometry — jsdom applies no CSS, so which side
/// of the screen the panel is on is a question only a browser can answer, and this file must not
/// pretend otherwise. What it does pin: the collapse is remembered, it survives a reload, and the
/// control describes what it will do rather than what it is.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults } from './lib/mock';

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
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

test('the panel starts open, and says what collapsing will do', async () => {
  render(App);
  const btn = await screen.findByRole('button', { name: 'collapse the panel' });
  expect(btn.getAttribute('aria-expanded')).toBe('true');
});

test('collapsing is remembered, so it is a preference and not a mood', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'collapse the panel' }));

  // The label flips to what the *next* click does — a control that says what it is rather than
  // what it does is the one people press twice to find out.
  await screen.findByRole('button', { name: 'expand the panel' });
  await waitFor(() => expect(store.get('fm-panel')).toBe('collapsed'));
});

test('a collapsed panel comes back collapsed', async () => {
  store.set('fm-panel', 'collapsed');
  render(App);
  const btn = await screen.findByRole('button', { name: 'expand the panel' });
  expect(btn.getAttribute('aria-expanded')).toBe('false');
});

test('storage that throws still gives a working panel', async () => {
  // A private window, or a WebView with site data blocked. The panel is a convenience; failing to
  // remember it must never be why the app will not start.
  vi.stubGlobal('localStorage', {
    getItem: () => { throw new Error('denied'); },
    setItem: () => { throw new Error('denied'); },
    removeItem: () => { throw new Error('denied'); },
  });
  render(App);
  const btn = await screen.findByRole('button', { name: 'collapse the panel' });
  await fireEvent.click(btn);
  await screen.findByRole('button', { name: 'expand the panel' });
});
