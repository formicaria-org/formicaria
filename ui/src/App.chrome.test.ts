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

test('the panel offers every view you can open, and opening one adds a window', async () => {
  render(App);
  const rail = (await screen.findByRole('navigation', { name: 'views' })) as HTMLElement;
  const names = Array.from(rail.querySelectorAll('button')).map((b) =>
    (b.textContent ?? '').trim(),
  );
  // The list is fixed so it can be learned.
  for (const v of ['Board', 'Agenda', 'Timeline', 'Collaboration', 'Discussions']) {
    expect(names, `${v} missing from the rail`).toContain(v);
  }
  // **And two absences that are decisions, not omissions.** `Search` is offered nowhere: an empty
  // search pane does nothing, and the search box makes one when you type. `Activity` is off the
  // rail — a column costs attention per entry — and stays in the palette, which is filterable and
  // can afford it.
  expect(names).not.toContain('Search');
  expect(names).not.toContain('Activity');
  // And the saved views, which is the half a fixed list of built-ins would miss.
  expect(names).toContain('Active');

  // Clicking opens it: a second window appears alongside the one that was already there.
  const before = document.querySelectorAll('.pane').length;
  await fireEvent.click(screen.getByRole('button', { name: 'open Agenda' }));
  await waitFor(() => expect(document.querySelectorAll('.pane').length).toBe(before + 1));
});

/// **The collapsed rail hides words; it must never crop them.**
///
/// The first version set the rail's width with `overflow: hidden` and left every label in place, so
/// it showed a sliver of "Back up" and a sliver of the vault name. jsdom applies no CSS, so this
/// cannot check that a label is *invisible* — and asserting that would be a lie. What it can check
/// is the thing that makes hiding possible at all: that no label is a bare text node the stylesheet
/// has no handle on.
test('every label in the chrome is wrapped in something CSS can hide', async () => {
  render(App);
  const header = (await waitFor(() => {
    const h = document.querySelector('header.topbar');
    if (!h) throw new Error('no header');
    return h;
  })) as HTMLElement;

  for (const chip of Array.from(header.querySelectorAll('.vault-chip'))) {
    expect(chip.querySelector('.lbl'), `a filter chip's name is not wrapped: ${chip.textContent}`)
      .toBeTruthy();
  }
  for (const item of Array.from(header.querySelectorAll('.view-item'))) {
    expect(item.querySelector('.lbl'), `a rail label is not wrapped: ${item.textContent}`)
      .toBeTruthy();
  }
});

test('back up is an icon like the controls beside it, and says so only while it is working', async () => {
  render(App);
  const btn = await screen.findByRole('button', { name: 'back up notes' });
  // Help and Settings next to it carry no words; one labelling rule per state is what makes the
  // column read as a column. The meaning lives in the tooltip.
  expect((btn.textContent ?? '').trim()).toBe('');
  expect(btn.getAttribute('title')).toMatch(/commit and push/i);
});

test('a view can be opened when the chrome is a bar, where there is no rail', async () => {
  // Below 60rem the rail is hidden and the pane header no longer switches views, so without this
  // a narrow window has no way to open a view at all. jsdom applies no CSS, so this cannot check
  // *which* of the two is visible at a given width — only that the second way in exists and works.
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  const menu = await screen.findByRole('menu');
  const names = Array.from(menu.querySelectorAll('button')).map((b) => (b.textContent ?? '').trim());
  expect(names).toContain('Timeline');
  expect(names).toContain('Active'); // a saved view, not just the built-ins
  expect(names).not.toContain('Search'); // nowhere offers an empty search pane

  const before = document.querySelectorAll('.pane').length;
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Agenda' }));
  await waitFor(() => expect(document.querySelectorAll('.pane').length).toBe(before + 1));
});
