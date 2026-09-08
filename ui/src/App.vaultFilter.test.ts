// **A filter that hides everything must not be able to hide its own control.**
//
// `hiddenVaults` lives in `localStorage` keyed by vault *name* and drops every note whose vault is
// listed (`App.svelte`'s `shown`). The trigger that undoes it — and the "N of M vaults" label whose
// own comment says it exists so that "the answer to 'where is that note?' is on the button" — sat
// behind `{#if allVaults.length > 1}`.
//
// So: hide one of two vaults, then remove the other. `allVaults.length` drops to 1, the whole block
// stops rendering, and the filter keeps hiding everything with no chip, no menu and no reset
// anywhere in the UI. The app looks empty. **This is what happened to the owner on 2026-09-08**,
// and it is why the vault they had *not* removed appeared to have lost its notes.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset } from './lib/mock';

const store = new Map<string, string>();
beforeEach(() => {
  reset();
  clearFaults();
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
  vi.unstubAllGlobals();
  reset();
});

test('the vault filter is reachable when it is hiding the only vault there is', async () => {
  // One vault left, and it is the hidden one — the state the owner was stranded in.
  await import('./lib/mock').then((m) => m.handle('forget_vault', { name: 'lab' }));
  store.set('fm-hidden-vaults', JSON.stringify(['personal']));

  render(App);

  // The control that undoes it must exist, or the filter is unrecoverable from inside the product.
  expect(await screen.findByRole('button', { name: /which vaults to show/i })).toBeTruthy();
});

test('and it says so, rather than reading as an empty notebook', async () => {
  await import('./lib/mock').then((m) => m.handle('forget_vault', { name: 'lab' }));
  store.set('fm-hidden-vaults', JSON.stringify(['personal']));

  render(App);

  // "0 of 1 vaults" is the whole point of the label: it turns "my notes are gone" into "I am
  // looking at none of my vaults".
  const chip = await screen.findByRole('button', { name: /which vaults to show/i });
  expect(chip.textContent).toMatch(/0 of 1/);
});

test('a name left over from a removed vault does not keep the filter on screen', async () => {
  // The inert case: `lab` is gone, so hiding it hides nothing. The control should be absent again,
  // or every user who ever removed a vault carries a filter chip forever.
  await import('./lib/mock').then((m) => m.handle('forget_vault', { name: 'lab' }));
  store.set('fm-hidden-vaults', JSON.stringify(['lab']));

  render(App);
  // Anchored on something that must render, so this cannot pass before the app has loaded.
  await screen.findByRole('button', { name: /back up notes/i });
  expect(screen.queryByRole('button', { name: /which vaults to show/i })).toBeNull();
});
