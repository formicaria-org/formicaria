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

it('+ suggests the features; one chosen opens in a new tab, and + on it again goes to that tab', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  const before = tabNames().length;
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  const sheet = await screen.findByRole('dialog', { name: 'new tab' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Food journal' }));
  await waitFor(() => expect(tabNames().length).toBe(before + 1));
  // No line over the tab: a feature's views are its own (the owner: it repeated them).
  expect(screen.queryByRole('button', { name: 'switch this tab to another feature' })).toBeNull();
  // + on a feature already open: that tab, not a second one.
  await fireEvent.click(tabs().getAllByRole('button')[0]);
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'new tab' })).getByRole('button', {
      name: 'Food journal',
    }),
  );
  await waitFor(() => expect(tabNames().length).toBe(before + 1));
  // × closes the tab in front.
  const front = screen
    .getByRole('list', { name: 'open tabs' })
    .querySelector('.tab.on .pick')!
    .textContent!.trim();
  await fireEvent.click(tabs().getByRole('button', { name: `close ${front}` }));
  await waitFor(() => expect(tabNames().length).toBe(before));
});

it('a new note left empty closes when it is left; one with words stays', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  const before = tabNames().length;
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'new tab' })).getByRole('button', {
      name: 'New note',
    }),
  );
  await waitFor(() => expect(tabNames().length).toBe(before + 1));
  // Leave it, empty: it goes.
  await fireEvent.click(tabs().getAllByRole('button')[0]);
  await waitFor(() => expect(tabNames().length).toBe(before));
});

it('a tab dragged into the screen shows with the one in front', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  await fireEvent.click(screen.getByRole('button', { name: 'new tab' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'new tab' })).getByRole('button', {
      name: 'Food journal',
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

it('≡ holds the options; the place of the bar is in Settings → Appearance', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: "formicaria's options" }));
  expect(screen.queryByRole('menuitem', { name: /This bar at the/ })).toBeNull();
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Settings' }));
  await fireEvent.click(
    await screen.findByRole('button', { name: 'Move it to the top' }, { timeout: 4000 }),
  );
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('bar-top')).toBe(true),
  );
  expect(store.get('fm-bar')).toBe('top');
});

it('a web page in front can be saved whole, from ≡', async () => {
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: () => {},
    hide: () => {},
    close: () => {},
    back: () => {},
    bring: () => {},
    html: (id: string) =>
      window.dispatchEvent(
        new CustomEvent('fm-page-html', {
          detail: {
            id,
            url: 'https://example.org/news/ants',
            title: 'Ants march on',
            html:
              '<html><head><title>Ants march on</title></head><body><article><h1>Ants march on</h1>' +
              '<p>Leafcutter ants carry leaves home along long trails every night, and grow fungus on them.</p>' +
              '<p>Scientists counted the trails for a whole year and found them busier in spring.</p></article></body></html>',
          },
        }),
      ),
  };
  try {
    render(App);
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://example.org/news/ants' } }),
    );
    await waitFor(() => expect(tabNames().some((n) => /example\.org/.test(n ?? ''))).toBe(true));
    await fireEvent.click(screen.getByRole('button', { name: "formicaria's options" }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save this page' }));
    expect(await screen.findByText(/Saved “Ants march on” in your notes/)).toBeTruthy();
  } finally {
    delete g.__fmPage;
  }
});
