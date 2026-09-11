/// **On a phone there is no top row: the open windows and the view's switches live in the bottom bar.**
///
/// The owner, 2026-09-11: the row at the top naming the view was not useful on a phone, and the square
/// counting windows "can fit perfectly right on the right side of the view button". So below 40rem the
/// top row is hidden and what it carried moves — a counted button beside the view button lists the open
/// windows, and the view menu starts with the switches of the view in front of you.
///
/// jsdom applies no CSS, so these pin what the controls do, not which width shows them; that is
/// checked by screenshot at 390px.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
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

/// Open a view through the bottom bar's view menu, the way a phone does.
async function openView(name: string) {
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name }));
}

const panes = () => document.querySelectorAll('.pane').length;
const windowsButton = () => screen.getByRole('button', { name: /^open windows: \d+$/ });

test('the counted button carries the number of open windows, and lists them', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  await openView('Agenda');
  await openView('Timeline');

  const button = windowsButton();
  expect(button.getAttribute('aria-label')).toBe(`open windows: ${panes()}`);
  expect(button.textContent?.trim()).toBe(String(panes()));

  await fireEvent.click(button);
  const menu = await screen.findByRole('menu', { name: 'open windows' });
  const names = within(menu)
    .getAllByRole('menuitem')
    .map((b) => b.textContent?.trim())
    .filter(Boolean);
  expect(names).toEqual(expect.arrayContaining(['Agenda', 'Timeline']));
  // The window in front of you is marked: the one opened last.
  expect(
    within(menu).getByRole('menuitem', { name: 'Timeline' }).getAttribute('aria-current'),
  ).toBe('true');
});

test('a window in the list switches to it, and its × closes it', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  await openView('Agenda');
  const before = panes();

  await fireEvent.click(windowsButton());
  let menu = await screen.findByRole('menu', { name: 'open windows' });
  const first = within(menu)
    .getAllByRole('menuitem')
    .filter((b) => b.textContent?.trim())[0];
  const name = first.textContent?.trim();
  await fireEvent.click(first);
  // Choosing is the end of choosing, and the window chosen is the one now in front.
  expect(screen.queryByRole('menu', { name: 'open windows' })).toBeNull();
  const row = screen.getByRole('navigation', { name: 'open views' });
  expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe(name);

  await fireEvent.click(windowsButton());
  menu = await screen.findByRole('menu', { name: 'open windows' });
  await fireEvent.click(within(menu).getByRole('menuitem', { name: 'close Agenda' }));
  await waitFor(() => expect(panes()).toBe(before - 1));
});

test('the view menu starts with the switches of the view in front of you', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  await openView('Agenda');

  await fireEvent.click(screen.getByRole('button', { name: 'open a view' }));
  const menu = await screen.findByRole('menu');
  expect(within(menu).getByText('Show as')).toBeTruthy();

  // Pressing Week applies it and puts the menu away — a mode is chosen once, not browsed.
  await fireEvent.click(within(menu).getByRole('button', { name: 'W' }));
  expect(screen.queryByRole('menu')).toBeNull();
  const row = screen.getByRole('navigation', { name: 'open views' });
  expect(within(row).getByRole('button', { name: 'W' }).className).toContain('on');
});

test('a view with no switches of its own adds nothing to the view menu', async () => {
  // The app opens on a board, which has none.
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  const menu = await screen.findByRole('menu');
  expect(within(menu).queryByText('Show as')).toBeNull();
  expect(within(menu).queryByRole('separator')).toBeNull();
});
