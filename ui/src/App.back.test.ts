// **Back walks what you opened, and panels close first** — `decisions.md` 2026-10-09.
//
// The phone's Back key reaches the page as the browser's own Back (`popstate`): Tauri's `AppPlugin`
// calls `goBack()` while the WebView has history, and closes the app only when it has none. So these
// drive `history.back()` and check where the app lands.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset } from './lib/mock';
import * as nav from './lib/nav.svelte';

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
  nav.reset();
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
});

/** The view the bar marks as showing — a hidden window stays mounted, so this is what counts. */
/// A built-in view is reached inside its group: open the group, then pick the view's tab.
async function openTab(group: string, tab: string) {
  await fireEvent.click(await screen.findByRole('button', { name: `open ${group}` }));
  await fireEvent.click(
    within(await screen.findByRole('group', { name: group })).getByRole('button', { name: tab }),
  );
}

const showing = () =>
  screen.getByRole('navigation', { name: 'open views' }).querySelector('[aria-current="page"]')
    ?.textContent ?? '';
const noteOpen = () => (/GAE|lambda/i.test(showing()) ? true : null);

test('Back from a note returns to the board it was opened from', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await waitFor(() => expect(noteOpen()).toBeTruthy());

  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('Back walks several steps in order: agenda → note → board', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await waitFor(() => expect(noteOpen()).toBeTruthy());
  await openTab('Tasks', 'Agenda');
  await waitFor(() => expect(showing()).toMatch(/Agenda/));

  history.back();
  await waitFor(() => expect(noteOpen()).toBeTruthy());
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('Back closes an open panel first, and stays where it was', async () => {
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  await fireEvent.click(screen.getByLabelText('settings'));
  const dialog = await screen.findByRole('heading', { name: 'Settings' });
  expect(dialog).toBeTruthy();

  history.back();
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Settings' })).toBeNull());
  expect(screen.getByText(/GAE lambda interacts badly/)).toBeTruthy();
});

test('Back first ends editing, then leaves the note', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await fireEvent.click(await screen.findByTitle('Click to edit'));
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();

  history.back();
  await waitFor(() => expect(screen.queryByLabelText('note body (Markdown)')).toBeNull());
  expect(noteOpen()).toBeTruthy();
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

test('the first screen is stamped, so there is nowhere earlier inside the app', async () => {
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  expect((history.state as { fm?: number; nav?: { kind: string } }).nav?.kind).toBeTruthy();
});

test('a view picked from a menu stays picked, and Back returns from it', async () => {
  // The other way a menu leads somewhere. Closing the menu must not step back onto the place just
  // left (the v0.6.2 phone bug, there through the window list).
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  // The menu names the same places the panel does: the Library opens on its shelves.
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Library' }));
  await waitFor(() => expect(showing()).toMatch(/Shelves/));
  await new Promise((r) => setTimeout(r, 100));
  expect(showing()).toMatch(/Shelves/);
  history.back();
  await waitFor(() => expect(showing()).toMatch(/Board/));
});

/// **The owner's report (2026-10-20)**: on the phone, Settings → About & updates → Back closed
/// Settings. On a narrow screen a category is a second screen: Back returns to the list, and only
/// the next Back closes Settings.
test('Back from a Settings category returns to the list on a narrow screen', async () => {
  const { setNarrowScreen } = await import('./test-setup');
  setNarrowScreen(true);
  try {
    render(App);
    await fireEvent.click(await screen.findByRole('button', { name: 'settings' }));
    const cats = await screen.findByRole('navigation', { name: 'settings categories' });
    await fireEvent.click(await within(cats).findByRole('button', { name: /About & updates/ }));
    const panel = () => document.querySelector('.at-list, [role="dialog"][aria-label="settings"]');
    await waitFor(() => expect(document.querySelector('.at-list')).toBeNull());

    // The category's entry is settled a moment after the press; a person's Back always comes later.
    await waitFor(() => expect(history.state?.layer).toBe('settings-category'));
    history.back();
    await waitFor(() => expect(document.querySelector('.at-list')).not.toBeNull());
    expect(panel()).not.toBeNull();

    history.back();
    await waitFor(() =>
      expect(screen.queryByRole('navigation', { name: 'settings categories' })).toBeNull(),
    );
  } finally {
    setNarrowScreen(false);
  }
});

// ── Back is one step, everywhere (2026-10-20) ───────────────────────────────────────────────
// After the owner's report, every level was audited. A person's Back always comes after the app
// has recorded the step (`nav.idle`); these wait for that before pressing it.

test('closing a note is a step back: Back does not reopen it', async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await waitFor(() => expect(noteOpen()).toBeTruthy());
  await fireEvent.click(await screen.findByRole('button', { name: 'close note' }));
  await waitFor(() => expect(showing()).toMatch(/Board/));
  await nav.idle();
  // The step back landed on the board's own entry: nothing above it holds the note.
  expect((history.state as { nav?: { kind: string } }).nav?.kind).toBe('board');
  history.back();
  await new Promise((r) => setTimeout(r, 150));
  expect(noteOpen()).toBeNull();
});

test("Back closes a note's ＋ window first, and the note stays", async () => {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  await fireEvent.click(await screen.findByRole('button', { name: 'note options' }));
  await screen.findByRole('dialog', { name: 'note options' });
  await nav.idle();
  history.back();
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'note options' })).toBeNull());
  expect(noteOpen()).toBeTruthy();
});

test('New vault asked for from Settings opens over it, and Back returns to Settings', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'settings' }));
  await screen.findByRole('heading', { name: 'Settings' });
  const cats = await screen.findByRole('navigation', { name: 'settings categories' });
  await fireEvent.click(await within(cats).findByRole('button', { name: /Vaults & backup/ }));
  await fireEvent.click(await screen.findByRole('button', { name: /New vault/ }));
  await screen.findByRole('dialog', { name: 'New vault' });
  await nav.idle();
  history.back();
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'New vault' })).toBeNull());
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
  await nav.idle();
  history.back();
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Settings' })).toBeNull());
});

test('a menu that hands over to a panel leaves no Back that does nothing', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /which vaults to show/i }));
  const menu = await screen.findByRole('menu');
  await fireEvent.click(within(menu).getByRole('menuitem', { name: /New vault/ }));
  await screen.findByRole('dialog', { name: 'New vault' });
  await nav.idle();
  // One entry for the panel, on the place's own: the menu's is gone.
  expect((history.state as { layer?: string }).layer).toBe('new-vault');
  history.back();
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'New vault' })).toBeNull());
  await nav.idle();
  expect((history.state as { layer?: string }).layer).toBeUndefined();
});
