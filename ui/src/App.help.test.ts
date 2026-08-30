/// **Help has to exist on the phone.**
///
/// The manual is baked into `fm-serve` at `/manual/`. On the phone the UI is served by the Tauri
/// shell instead, so that path resolves to nothing — and rather than ship a button that 404s, the
/// Help button was hidden there entirely. Correct in the small and wrong in the large: the result
/// was a phone with no help at all, in an app whose first non-technical tester's verdict on Help
/// was that it was "difficult to find and click on".
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults } from './lib/mock';
import { asPhone, asDesktop } from './lib/harness';

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
  asDesktop();
  vi.unstubAllGlobals();
  clearFaults();
});

test('the help button is there on the desktop and opens something readable', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'help' }));
  const dialog = await screen.findByRole('dialog', { name: /how this works/i });
  // The things a newcomer is actually stuck on, not a table of contents.
  expect(dialog.textContent).toMatch(/New note/);
  expect(dialog.textContent).toMatch(/Timeline/);
  expect(dialog.textContent).toMatch(/Back up/);
  // Where the full book exists, say so.
  expect(screen.getByRole('button', { name: /open the full manual/i })).toBeTruthy();
});

test('the phone gets the same help, without a link to a manual it cannot serve', async () => {
  // The real phone bridge the rest of the suite uses, not a bare marker: `isPhone()` only reads
  // `__TAURI_INTERNALS__`, but setting that without a bridge sends every command into nothing.
  asPhone();
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'help' }));
  const dialog = await screen.findByRole('dialog', { name: /how this works/i });
  expect(dialog.textContent).toMatch(/New note/);
  // `/manual/` is not served by the Tauri shell; offering it would be the 404 button again.
  expect(screen.queryByRole('button', { name: /open the full manual/i })).toBeNull();
});
