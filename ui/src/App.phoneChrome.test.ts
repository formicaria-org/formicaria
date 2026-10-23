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
import { clearFaults, reset, seed } from './lib/mock';

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
  reset();
});

/// Open a view through the bottom bar's view menu, the way a phone does.
async function openView(name: string) {
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name }));
}

const panes = () => document.querySelectorAll('.pane').length;
const windowsButton = () => screen.getByRole('button', { name: /^open notes: \d+$/ });
/// Open the GAE note from the board — a note being the only kind of window (2026-10-16).
async function openGaeNote() {
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await screen.findByRole('button', { name: 'note options' });
}

test('the counted button counts open notes, and lists them with the view', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  // No note open: nothing to count, so no button. And a view opened is not a window.
  await openView('Library');
  expect(screen.queryByRole('button', { name: /^open notes:/ })).toBeNull();
  await openView('Tasks');
  await openGaeNote();

  const button = windowsButton();
  expect(button.getAttribute('aria-label')).toBe('open notes: 1');
  expect(button.textContent?.trim()).toBe('1');

  await fireEvent.click(button);
  const menu = await screen.findByRole('menu', { name: 'open windows' });
  const names = within(menu)
    .getAllByRole('menuitem')
    .map((b) => b.getAttribute('aria-label') ?? b.textContent?.trim())
    .filter(Boolean);
  // The view, which is not closed — only re-pointed — and the note, which is.
  expect(names.filter((n) => n?.startsWith('close '))).toHaveLength(1);
  expect(within(menu).queryByRole('menuitem', { name: 'close Board · status' })).toBeNull();
  expect(panes()).toBe(2);
});

test('a window in the list switches to it, and its × closes it', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  await openGaeNote();
  const note = screen
    .getByRole('navigation', { name: 'open views' })
    .querySelector('[aria-current="page"]')
    ?.textContent?.trim();

  await fireEvent.click(windowsButton());
  let menu = await screen.findByRole('menu', { name: 'open windows' });
  await fireEvent.click(within(menu).getByRole('menuitem', { name: 'Board · status' }));
  // Choosing is the end of choosing, and the window chosen is the one now in front.
  expect(screen.queryByRole('menu', { name: 'open windows' })).toBeNull();
  const row = screen.getByRole('navigation', { name: 'open views' });
  expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe('Board · status');
  // **And it stays there** (v0.6.2, a user's phone): closing the list stepped history back a
  // moment later, onto the window just left, so the switch undid itself.
  await new Promise((r) => setTimeout(r, 100));
  expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe('Board · status');
  // Switching is a move: Back returns to the note, where we were.
  history.back();
  await waitFor(() =>
    expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe(note),
  );

  await fireEvent.click(windowsButton());
  menu = await screen.findByRole('menu', { name: 'open windows' });
  await fireEvent.click(within(menu).getByRole('menuitem', { name: `close ${note}` }));
  await waitFor(() => expect(panes()).toBe(1));
  // The last note closed: the button goes, and the view is in front.
  expect(screen.queryByRole('button', { name: /^open notes:/ })).toBeNull();
  expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe('Board · status');
});

test('the view menu starts with the switches of the view in front of you', async () => {
  render(App);
  await screen.findByRole('button', { name: 'open a view' });
  // The agenda is a tab of Tasks, which is where the app opens.
  await fireEvent.click(
    within(await screen.findByRole('group', { name: 'Tasks' })).getByRole('button', {
      name: 'Agenda',
    }),
  );

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

/// **A note's window is named by its title.** The owner, on the list behind the counted button: instead of
/// a generic "Note", the first letters of the note's title. The list truncates; the name is the title.
test('a note window is named by its title, not "Note"', async () => {
  reset();
  seed({ notes: 3 });
  render(App);
  await fireEvent.click(await screen.findByText(/Seeded 0/));
  await screen.findByLabelText('note options');

  await fireEvent.click(windowsButton());
  const menu = await screen.findByRole('menu', { name: 'open windows' });
  // The title arrives when the note has loaded, so the row may say "Note" for a moment first.
  await waitFor(() =>
    expect(within(menu).getByRole('menuitem', { name: 'Seeded 0' })).toBeTruthy(),
  );
  expect(within(menu).queryByRole('menuitem', { name: 'Note' })).toBeNull();
  // The same name on the wide screen's tab for that window, from the one function both use.
  const row = screen.getByRole('navigation', { name: 'open views' });
  expect(row.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe('Seeded 0');
});
