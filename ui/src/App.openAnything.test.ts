// **formicaria contains the browser** (`decisions.md` 2026-11-16, night, last): inside formicaria's
// browser the app says its item in its own address (`#at=`), so the browser's formicaria tab follows
// it; a new tab (＋) opens *Open…*, where a note found fills the tab and an address turns it into a page.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => {
  mock.reset();
  document.documentElement.setAttribute('data-formicaria-bridge', '');
});
afterEach(() => {
  document.documentElement.removeAttribute('data-formicaria-bridge');
  location.hash = '';
});

const at = () => {
  const m = /#at=([^&]*)&title=([^&]*)/.exec(location.hash);
  return m ? { item: decodeURIComponent(m[1]), title: decodeURIComponent(m[2]) } : null;
};

it('the app says the item in front, so its tab can follow it', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Field notes', body: 'Ants.', props: {} }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}&tab`;
  render(App);
  await waitFor(() => expect(at()).toEqual({ item: `note:${made.id}`, title: 'Field notes' }));
});

it('a new tab opens Open…: a note found fills it, an address makes it a page', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Leafcutter colonies', body: 'Fungus.', props: {} }],
  })) as { id: string }[];
  location.hash = '#open&tab';
  render(App);
  const field = await screen.findByRole('searchbox', { name: 'search your notes or the web' });
  await waitFor(() => expect(at()?.item).toBe('open:'));

  await fireEvent.input(field, { target: { value: 'example.org/ants' } });
  expect(await screen.findByRole('button', { name: 'Go to example.org/ants' })).toBeTruthy();
  expect(
    screen.getByRole('button', { name: 'Search the web for “example.org/ants”' }),
  ).toBeTruthy();

  await fireEvent.input(field, { target: { value: 'Leafcutter' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'Leafcutter colonies' }));
  await waitFor(() => expect(at()?.item).toBe(`note:${made.id}`));
  expect(screen.queryByRole('searchbox', { name: 'search your notes or the web' })).toBeNull();
});

it('an address chosen in Open… is said as a page for the browser to load', async () => {
  location.hash = '#open&tab';
  render(App);
  const field = await screen.findByRole('searchbox', { name: 'search your notes or the web' });
  await fireEvent.input(field, { target: { value: 'example.org' } });
  // The browser reads every change of the address; the page's is the one before the app moves on.
  const seen: string[] = [];
  const replace = history.replaceState.bind(history);
  history.replaceState = (s, t, u) => {
    seen.push(String(u));
    replace(s, t, u);
  };
  try {
    await fireEvent.click(await screen.findByRole('button', { name: 'Go to example.org' }));
  } finally {
    history.replaceState = replace;
  }
  expect(seen.some((u) => u.includes(`#at=${encodeURIComponent('web:https://example.org')}`))).toBe(
    true,
  );
});

it('Ctrl+click or a middle click on a note opens it in a tab of its own', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Tabbed away', body: 'Here.', props: {} }],
  })) as { id: string }[];
  const sent: Record<string, unknown>[] = [];
  const post = window.postMessage.bind(window);
  window.postMessage = ((m: { formicaria_up?: Record<string, unknown> }, o: string) => {
    if (m?.formicaria_up) sent.push(m.formicaria_up);
    post(m, o);
  }) as typeof window.postMessage;
  try {
    location.hash = '#open&tab';
    render(App);
    const field = await screen.findByRole('searchbox', { name: 'search your notes or the web' });
    await fireEvent.keyDown(field, { key: 'Escape' });
    // The search under the bar lists it; a Ctrl+click asks the browser for a tab.
    const search = screen.getAllByRole('searchbox', { name: 'search notes' })[0];
    await fireEvent.input(search, { target: { value: 'Tabbed' } });
    const hit = await screen.findByRole('button', { name: /Tabbed away/ });
    await fireEvent.click(hit, { ctrlKey: true });
    await waitFor(() =>
      expect(sent).toContainEqual({ up: 'open-tab', address: `note:${made.id}`, title: '' }),
    );
    // It opened in its own tab, not here.
    expect(screen.queryByText('Here.')).toBeNull();
  } finally {
    window.postMessage = post;
  }
});
