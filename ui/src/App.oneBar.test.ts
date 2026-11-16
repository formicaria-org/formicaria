// **The one bar** (`decisions.md` 2026-11-17, *one frame*; the phone app's frame): ≡ (options), the
// tabs, + (suggesting the features). A tab is one feature, note or page; its line's chooser moves *this*
// tab; a tab dragged into the screen tiles with the one in front; the bar can sit at the top.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import { useTabBar } from './lib/shell.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  mock.reset();
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  useTabBar(true);
});
afterEach(() => {
  useTabBar(null);
  vi.unstubAllGlobals();
});

const tabs = () => within(screen.getByRole('list', { name: 'open tabs' }));
const tabNames = () =>
  [...screen.getByRole('list', { name: 'open tabs' }).querySelectorAll('.pick')].map((b) =>
    b.textContent?.trim(),
  );

it('one bar: options, the tabs, and +; none of the old bar', async () => {
  render(App);
  expect(await screen.findByRole('button', { name: "formicaria's options" })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'new tab' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'open a view' })).toBeNull();
  expect(document.querySelector('.topbar')).toBeNull();
  expect(tabNames().length).toBeGreaterThan(0);
});

it('+ suggests the features; one chosen opens in a new tab, which a chooser moves', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  const before = tabNames().length;
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  const sheet = await screen.findByRole('dialog', { name: 'new tab' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Tasks' }));
  await waitFor(() => expect(tabNames().length).toBe(before + 1));
  // The tab's line: the feature as a chooser that moves this tab.
  await fireEvent.click(
    await screen.findByRole('button', { name: 'switch this tab to another feature' }),
  );
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Agenda' }));
  await waitFor(() => expect(tabNames()).toContain('Agenda'));
  expect(tabNames().length).toBe(before + 1);
  // × closes the tab in front.
  await fireEvent.click(tabs().getByRole('button', { name: 'close Agenda' }));
  await waitFor(() => expect(tabNames().length).toBe(before));
});

it('a tab dragged into the screen shows with the one in front', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'new tab' })).getByRole('button', {
      name: 'Tasks',
    }),
  );
  await waitFor(() => expect(tabNames().length).toBeGreaterThan(1));
  const other = screen
    .getByRole('list', { name: 'open tabs' })
    .querySelector('.tab:not(.on)') as HTMLElement;
  await fireEvent.pointerDown(other, { clientY: 600 });
  await fireEvent.pointerMove(other, { clientY: 500 });
  await fireEvent.pointerUp(other, { clientY: 500 });
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('paired')).toBe(true),
  );
});

it('≡ holds the options, and moves the bar to the top', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: "formicaria's options" }));
  await fireEvent.click(screen.getByRole('menuitem', { name: 'This bar at the top' }));
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('bar-top')).toBe(true),
  );
  expect(store.get('fm-bar')).toBe('top');
  await fireEvent.click(screen.getByRole('button', { name: "formicaria's options" }));
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Settings' }));
  expect(await screen.findByRole('heading', { name: 'Settings' }, { timeout: 4000 })).toBeTruthy();
});
