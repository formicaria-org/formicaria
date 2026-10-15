// **Back up is actions; a vault is set up in Settings → Vaults & backup and nowhere else; a new
// vault is made where vaults are listed** (`decisions.md#ui`, 2026-10-15). The owner: "Should be easier to find no? Also, backup
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

test('the Back up menu holds actions only, and no way into settings', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'back up' }));
  const menu = await screen.findByRole('menu');
  const items = within(menu)
    .getAllByRole('menuitem')
    .map((b) => (b.textContent ?? '').trim());
  expect(items.some((t) => t.startsWith('Back up notes'))).toBe(true);
  expect(items.some((t) => t.startsWith('Get their changes'))).toBe(true);
  // A vault's settings have one door, the gear — not a second one from an actions menu.
  for (const gone of ['Backup settings', 'Backup options', 'Attachment settings']) {
    expect(items.some((t) => t.startsWith(gone))).toBe(false);
  }
});

test('Take a snapshot is offered where a vault has a backup repo, and not otherwise', async () => {
  const m = await import('./lib/mock');
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'back up' }));
  let menu = await screen.findByRole('menu');
  expect(within(menu).queryByRole('menuitem', { name: /Take a snapshot/ })).toBeNull();
  await fireEvent.click(await screen.findByRole('button', { name: 'back up' }));

  await m.handle('set_restic_repo', { vault: 'personal', repo: '/backup/personal' });
  const { refreshVaults } = await import('./lib/vaults.svelte');
  await refreshVaults();
  await fireEvent.click(await screen.findByRole('button', { name: 'back up' }));
  menu = await screen.findByRole('menu');
  expect(within(menu).getByRole('menuitem', { name: /Take a snapshot/ })).toBeTruthy();
});

test('the vault menu, where the vaults are listed, offers New vault…', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /which vaults to show/i }));
  const menu = await screen.findByRole('menu');
  await fireEvent.click(within(menu).getByRole('menuitem', { name: /New vault/ }));
  // The form that creates, joins, restores or imports a vault.
  expect(await screen.findByRole('button', { name: /import from another app/i })).toBeTruthy();
});
