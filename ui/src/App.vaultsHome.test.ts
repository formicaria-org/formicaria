// **Back up is actions; a vault is set up in Settings; a new vault is made where vaults are
// listed** (`decisions.md#ui`, 2026-10-15). The owner: "Should be easier to find no? Also, backup
// options looks like something I would put into settings."
import { fireEvent, render, screen, within } from '@testing-library/svelte';
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

test('the Back up menu offers actions, and one way into the settings', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'back up' }));
  const menu = await screen.findByRole('menu');
  const items = within(menu)
    .getAllByRole('menuitem')
    .map((b) => (b.textContent ?? '').trim());
  expect(items.some((t) => t.startsWith('Backup settings…'))).toBe(true);
  // The two it replaces are gone — one place for a vault's settings, not two doors to two places.
  expect(items.some((t) => t.startsWith('Backup options'))).toBe(false);
  expect(items.some((t) => t.startsWith('Attachment settings'))).toBe(false);

  await fireEvent.click(within(menu).getByRole('menuitem', { name: /Backup settings/ }));
  // It lands on Vaults, in Settings — with New vault… right there.
  expect(await screen.findByRole('button', { name: 'New vault…' })).toBeTruthy();
});

test('the vault menu, where the vaults are listed, offers New vault…', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /which vaults to show/i }));
  const menu = await screen.findByRole('menu');
  await fireEvent.click(within(menu).getByRole('menuitem', { name: /New vault/ }));
  // The form that creates, joins, restores or imports a vault.
  expect(await screen.findByRole('button', { name: /import from another app/i })).toBeTruthy();
});
