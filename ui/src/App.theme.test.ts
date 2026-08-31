/// **A theme you cannot take off is the one unrecoverable failure here.**
///
/// A theme is arbitrary CSS in the user's own vault, and there is no CSS-level guarantee against
/// one that hides every control — so the layer that actually rescues the app does not depend on CSS
/// at all. Before a stored theme is applied a flag is set; the first click, key, wheel or touch
/// clears it. If the app starts and the flag is still set from last time, nobody managed to reach
/// anything, and the theme is not put back.
///
/// This is the part of the escape hatch that is pure JS, and therefore the part that can actually
/// be tested. The escape *control's* survival against a hostile `!important` is not asserted
/// anywhere, because jsdom does not implement the cascade faithfully and a green test there would
/// be a claim we cannot support.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';

const { readTheme, ping } = vi.hoisted(() => ({ readTheme: vi.fn(), ping: vi.fn() }));
vi.mock('./lib/ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/ipc')>()),
  readTheme,
  ping,
}));

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
  readTheme.mockResolvedValue(':root { --bg: #f4f1ea; }');
  // The mock vault never moves (`changed: false`), so the heartbeat is stubbed per-test where a
  // test needs it to report one.
  ping.mockResolvedValue({
    changed: false, generation: 0, git: true, restic: false, skipped: [], unopened_vaults: [],
  });
  document.getElementById('fm-theme')?.remove();
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

function selectTheme() {
  store.set('fm-appearance', JSON.stringify({ vault: 'personal', name: 'writing-desk' }));
}

test('a chosen theme is applied at startup', async () => {
  selectTheme();
  render(App);
  await waitFor(() =>
    expect(document.getElementById('fm-theme')?.textContent).toBe(':root { --bg: #f4f1ea; }'),
  );
});

test('a theme nobody could interact with last time is not put back on', async () => {
  selectTheme();
  // What the previous run left behind: armed, never disarmed, because nothing was ever clicked.
  store.set('fm-appearance-armed', '1');
  render(App);

  // The banner has to say what happened and how to undo it — an app that silently ignores a
  // setting is indistinguishable from one that lost it.
  const banner = await screen.findByText(/was switched off/i);
  expect(banner.textContent).toMatch(/writing-desk/);
  expect(banner.textContent).toMatch(/Settings/);

  expect(readTheme).not.toHaveBeenCalled();
  expect(document.getElementById('fm-theme')).toBeNull();
  // And the selection is gone, so this does not repeat every launch.
  expect(store.get('fm-appearance')).toBeUndefined();
});

test('the way out is on screen until the app is shown to be reachable', async () => {
  selectTheme();
  render(App);
  const escape = await screen.findByRole('button', { name: /turn off/i });
  expect(escape.textContent).toMatch(/writing-desk/);

  // Any interaction anywhere is the proof: the control goes, and the flag with it.
  await fireEvent(document, new Event('pointerdown', { bubbles: true }));
  await waitFor(() => expect(screen.queryByRole('button', { name: /turn off/i })).toBeNull());
  expect(store.get('fm-appearance-armed')).toBeUndefined();
});

test('turning it off removes the styling and the choice together', async () => {
  selectTheme();
  render(App);
  const escape = await screen.findByRole('button', { name: /turn off/i });
  await fireEvent.click(escape);
  await waitFor(() => expect(document.getElementById('fm-theme')).toBeNull());
  expect(store.get('fm-appearance')).toBeUndefined();
});

test('a theme whose file has gone falls back instead of leaving a blank screen', async () => {
  selectTheme();
  readTheme.mockRejectedValue(new Error("there is no theme called 'writing-desk'"));
  render(App);
  await screen.findByText(/not in this vault any more/i);
  expect(document.getElementById('fm-theme')).toBeNull();
  expect(store.get('fm-appearance')).toBeUndefined();
});

test('no stored theme means nothing is fetched and nothing is injected', async () => {
  render(App);
  await waitFor(() => expect(screen.queryByRole('button', { name: /turn off/i })).toBeNull());
  expect(readTheme).not.toHaveBeenCalled();
  expect(document.getElementById('fm-theme')).toBeNull();
});

/// **Not tested here, and that is a statement rather than an omission.** The bug this file would
/// most like to pin — the escape control coming back after every edit — travels through the vault
/// heartbeat, and the repeating heartbeat is production-only (`App.svelte`: "a repeating interval
/// under Vitest is its own bug"). Under jsdom exactly one beat runs, at mount, so there is no way
/// to make the theme effect re-run for an already-proven theme. A test written against it passes
/// whether the fix is present or not, which is worse than no test. The guard is `armedFor` in
/// `App.svelte`; verify it by using the app, not by reading a green tick here.
