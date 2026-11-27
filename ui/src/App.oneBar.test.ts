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

/** **The Food journal has drawn**, so its lazily loaded screen (and the charts it imports) is not
 *  still loading when the test ends: Vitest fails the run when a module loads after its environment
 *  is torn down (`known-issues.md`, *Vitest closing a worker*; seen here on 2026-11-25). */
const trackerDrawn = () =>
  waitFor(() => expect(document.querySelector('.tracker')).toBeTruthy(), { timeout: 10000 });

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
  await trackerDrawn();
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
  await trackerDrawn();
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

it('≡ → Privacy opens the switches for the page in front', async () => {
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = { show: () => {}, hide: () => {}, close: () => {}, privacy: () => {} };
  try {
    render(App);
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://news.example/story' } }),
    );
    await waitFor(() => expect(tabNames().some((n) => /news\.example/.test(n ?? ''))).toBe(true));
    await fireEvent.click(screen.getByRole('button', { name: "formicaria's options" }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Privacy' }));
    const sheet = await screen.findByRole('dialog', { name: 'privacy' });
    expect(within(sheet).getByText('news.example')).toBeTruthy();
    expect(within(sheet).getByRole('switch', { name: 'Block trackers' })).toBeTruthy();
  } finally {
    delete g.__fmPage;
  }
});

it('≡ → Save media lists the page’s embedded video and what it fetched', async () => {
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: () => {},
    hide: () => {},
    close: () => {},
    html: (id: string) =>
      window.dispatchEvent(
        new CustomEvent('fm-page-html', {
          detail: {
            id,
            url: 'https://news.example/ants',
            title: 'Ants march on',
            html: '<html><head><title>Ants march on</title></head><body><iframe title="Leafcutters" src="https://www.youtube.com/embed/AbCdEf12345"></iframe></body></html>',
            seen: ['https://cdn.example/pod/ep1.mp3'],
          },
        }),
      ),
  };
  try {
    render(App);
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://news.example/ants' } }),
    );
    await waitFor(() => expect(tabNames().some((n) => /news\.example/.test(n ?? ''))).toBe(true));
    await fireEvent.click(screen.getByRole('button', { name: "formicaria's options" }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save media' }));
    const sheet = await screen.findByRole('dialog', { name: 'save media' });
    expect(within(sheet).getByRole('radio', { name: 'YouTube video: Leafcutters' })).toBeTruthy();
    expect(within(sheet).getByRole('radio', { name: 'Audio: ep1.mp3' })).toBeTruthy();
  } finally {
    delete g.__fmPage;
  }
});

it('a page has its address row: a new address or a search goes in this page', async () => {
  const went: [string, string][] = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: () => {},
    hide: () => {},
    close: () => {},
    go: (id: string, url: string) => went.push([id, url]),
    reload: () => {},
  };
  try {
    render(App);
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://news.example/ants' } }),
    );
    await fireEvent.click(await screen.findByRole('button', { name: "this page's address" }));
    const field = screen.getByRole('searchbox', { name: 'a web address or a search' });
    expect((field as HTMLInputElement).value).toBe('https://news.example/ants');
    await fireEvent.input(field, { target: { value: 'leafcutter ants' } });
    await fireEvent.submit(field.closest('form')!);
    expect(went[0][1]).toBe('https://duckduckgo.com/?q=leafcutter%20ants');
  } finally {
    delete g.__fmPage;
  }
});

it('Back with nowhere further in a page opened from a link closes it, back to its page', async () => {
  const closed: string[] = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = { show: () => {}, hide: () => {}, close: (id: string) => closed.push(id) };
  try {
    render(App);
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://news.example/ants' } }),
    );
    await waitFor(() => expect(tabNames().some((n) => /news\.example/.test(n ?? ''))).toBe(true));
    window.dispatchEvent(
      new CustomEvent('fm-page-open', { detail: { url: 'https://other.example/linked' } }),
    );
    await waitFor(() => expect(tabNames().some((n) => /other\.example/.test(n ?? ''))).toBe(true));
    const front = () =>
      screen.getByRole('list', { name: 'open tabs' }).querySelector('.tab.on .pick')!.textContent!;
    expect(front()).toMatch(/other\.example/);
    // The linked page, as the workspace keeps it: Back from it, with nowhere further in it.
    await waitFor(() => expect(store.get('fm-workspace')).toContain('other.example'));
    const ws = JSON.parse(store.get('fm-workspace')!) as { panes: { id: string; url?: string }[] };
    const linked = ws.panes.find((p) => p.url === 'https://other.example/linked')!.id;
    window.dispatchEvent(new CustomEvent('fm-page-back', { detail: { id: linked } }));
    await waitFor(() => expect(front()).toMatch(/news\.example/));
    expect(tabNames().some((n) => /other\.example/.test(n ?? ''))).toBe(false);
    expect(closed).toEqual([linked]);
  } finally {
    delete g.__fmPage;
  }
});

it('the only tab can be closed too', async () => {
  render(App);
  await screen.findByRole('button', { name: 'new tab' });
  while (tabNames().length > 1) {
    const front = screen
      .getByRole('list', { name: 'open tabs' })
      .querySelector('.tab.on .pick')!
      .textContent!.trim();
    await fireEvent.click(tabs().getByRole('button', { name: `close ${front}` }));
  }
  const last = tabNames()[0];
  expect(tabs().getByRole('button', { name: `close ${last}` })).toBeTruthy();
});

it('a saved page’s copy opens in a page tab on the phone', async () => {
  const shown: string[] = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: (_id: string, url: string) => shown.push(url),
    hide: () => {},
    close: () => {},
  };
  try {
    render(App);
    await screen.findByRole('button', { name: 'new tab' });
    const a = document.createElement('a');
    a.className = 'asset-link';
    a.dataset.ref = `asset:sha256-${'c'.repeat(64)}`;
    a.href = `data:text/html,${encodeURIComponent('<!doctype html><html><title>Ants</title></html>')}`;
    a.textContent = 'A copy of the page as it was saved';
    document.body.append(a);
    await fireEvent.click(a);
    await waitFor(() =>
      expect(tabNames().some((n) => /A copy of the page as it was saved/.test(n ?? ''))).toBe(true),
    );
    a.remove();
  } finally {
    delete g.__fmPage;
  }
});
