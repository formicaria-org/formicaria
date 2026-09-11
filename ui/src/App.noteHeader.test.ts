/// **A note's header is its title and one ＋, on every device — and leaving a note ends editing.**
///
/// The owner, 2026-09-11, with a screenshot: the header was three rows (title; vault, last editor and a
/// ＋; ＋ Media, full screen and close), and ＋ Media's menu opened half off the left of the phone. Asked
/// for: "just a single title and a single plus to do things", no Done button, delete under the plus.
///
/// jsdom applies no CSS, so whether the window fits the screen is checked by screenshot. These pin what
/// the header holds, what the ＋ holds, and every way editing now ends.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults, reset } from './lib/mock';

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
  // The mock's notes outlive a test, and ending editing may write one; start each test from the seed.
  reset();
});

const editor = () => screen.queryByLabelText('note body (Markdown)');

/// Open the GAE note from the board, and return its ＋.
async function openNote() {
  render(App);
  await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
  return screen.findByRole('button', { name: 'note options' });
}

async function openOptions(plus: HTMLElement) {
  await fireEvent.click(plus);
  return screen.findByRole('dialog', { name: 'note options' });
}

test('the header holds the title and one ＋, and nothing else', async () => {
  const plus = await openNote();
  const header = plus.closest('header')!;
  expect(within(header).getByRole('heading', { level: 2 })).toBeTruthy();
  const buttons = within(header).getAllByRole('button');
  expect(buttons.map((b) => b.getAttribute('aria-label') ?? b.textContent?.trim())).toEqual([
    'note options',
  ]);
});

test('an untitled note is named by its first line, not "note"', async () => {
  const plus = await openNote();
  const heading = within(plus.closest('header')!).getByRole('heading', { level: 2 });
  expect(heading.textContent).toBe('GAE lambda interacts badly with inner-loop adaptation');
});

test('the ＋ holds what a note offers, and at its foot whose note it is', async () => {
  const win = await openOptions(await openNote());
  for (const name of ['Edit', 'Add media', 'Delete', 'Close']) {
    expect(within(win).getByRole('button', { name })).toBeTruthy();
  }
  // One at a time, a note already fills the window, so widening it is not offered.
  expect(within(win).queryByRole('button', { name: /full screen/i })).toBeNull();
  expect(win.querySelector('.options-info')?.textContent?.trim()).toBeTruthy();
});

test('there is no Done: while editing, the ＋ offers neither Done nor Edit', async () => {
  const plus = await openNote();
  await fireEvent.click(within(await openOptions(plus)).getByRole('button', { name: 'Edit' }));
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();

  const win = await openOptions(plus);
  expect(within(win).queryByRole('button', { name: /^(Done|Edit)$/ })).toBeNull();
});

test('Add media from reading opens the editor, because media goes in at the caret', async () => {
  const win = await openOptions(await openNote());
  expect(editor()).toBeNull();
  await fireEvent.click(within(win).getByRole('button', { name: 'Add media' }));
  const kinds = within(win).getByRole('group', { name: 'add media' });
  // The first kind after the recorder opens a file picker, which jsdom ignores.
  await fireEvent.click(within(kinds).getAllByRole('button')[1]);
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();
});

test('switching to another window ends editing', async () => {
  const plus = await openNote();
  await fireEvent.click(within(await openOptions(plus)).getByRole('button', { name: 'Edit' }));
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();

  await fireEvent.click(await screen.findByRole('button', { name: 'open Agenda' }));
  await waitFor(() => expect(editor()).toBeNull());
});

test("the phone's Back ends editing", async () => {
  const plus = await openNote();
  await fireEvent.click(within(await openOptions(plus)).getByRole('button', { name: 'Edit' }));
  expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();

  history.back();
  await waitFor(() => expect(editor()).toBeNull());
});
