// **Home's edits do what they say, on screen** (2026-11-23; the owner: a board set to *summary* on the
// phone did not appear on the phone's Home). Every edit is checked by what Home **shows**, on a wide
// screen and at a phone's width — never only by what the Home note says.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import HomeView from './HomeView.svelte';
import * as mock from '../lib/mock';
import { resetShownVaults } from '../lib/shownVaults.svelte';

let width = 1200;
beforeEach(() => {
  mock.reset();
  resetShownVaults();
  width = 1200;
  // Home measures its own width: a phone is one narrower than 700 px.
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.classList?.contains('home') ? width : 0;
  });
  // A task, so the generated Home has its board and what is due.
  void mock.handle('create_notes', {
    notes: [{ title: 'Write the methods', body: '', props: { status: 'todo' } }],
  });
});
afterEach(() => vi.restoreAllMocks());

const show = () => render(HomeView, { go: () => {}, onopen: () => {} });
const grid = () => screen.findByRole('list', { name: 'home cards shown' });
const cardNamed = (list: HTMLElement, name: RegExp) =>
  within(list)
    .queryAllByRole('listitem')
    .find((li) => li.parentElement === list && name.test(li.textContent ?? ''));
async function menu(name: string) {
  await fireEvent.click(await screen.findByRole('button', { name: `more for ${name}` }));
  return screen.findByRole('group', { name: `${name} on Home` });
}

it('wide: removing, resizing and making a card full width are what Home then shows', async () => {
  show();
  const list = await grid();
  await within(list).findByRole('heading', { name: /Continue/ });
  await fireEvent.click(within(await menu('Continue')).getByRole('button', { name: 'Full width' }));
  await vi.waitFor(() =>
    expect(
      cardNamed(list, /Continue/)
        ?.getAttribute('style')
        ?.replace(/\s/g, ''),
    ).toContain('--span:12'),
  );
  await fireEvent.click(within(await menu('Continue')).getByRole('button', { name: 'Tall' }));
  await vi.waitFor(() =>
    expect(
      cardNamed(list, /Continue/)
        ?.getAttribute('style')
        ?.replace(/\s/g, ''),
    ).toContain('--h:26rem'),
  );
  await fireEvent.click(
    within(await menu('Continue')).getByRole('button', { name: 'Remove from Home' }),
  );
  await vi.waitFor(() => expect(cardNamed(list, /Continue/)).toBeUndefined());
});

it('wide: a card shown as a glance moves to the figures; as a summary it comes back', async () => {
  show();
  const list = await grid();
  await within(list).findByRole('heading', { name: /Tasks board/ });
  await fireEvent.click(within(await menu('Tasks board')).getByRole('button', { name: 'Glance' }));
  const figures = await screen.findByRole('list', { name: 'at a glance' });
  await vi.waitFor(() => expect(within(figures).getByText(/open in Tasks/)).toBeTruthy());
  expect(cardNamed(list, /Tasks board/)).toBeUndefined();
});

it('phone: a card changed on the phone is shown on the phone (the board set to summary)', async () => {
  width = 400;
  show();
  const list = await grid();
  // The board starts off the phone: not in the phone's Home.
  await within(list).findByRole('heading', { name: /Due and planned/ });
  expect(cardNamed(list, /Tasks board/)).toBeUndefined();
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit Home' }));
  const rows = screen.getByRole('list', { name: 'home cards' });
  const row = within(rows)
    .getAllByRole('listitem')
    .find((li) => /Tasks board/.test(li.textContent ?? ''))!;
  expect(row.textContent).toContain('not shown on the phone');
  await fireEvent.change(within(row).getByRole('combobox', { name: 'size of Tasks board' }), {
    target: { value: 'summary' },
  });
  await vi.waitFor(() => expect(cardNamed(list, /Tasks board/)).toBeTruthy());
});

it('phone: a card added on the phone is shown on the phone', async () => {
  width = 400;
  show();
  await grid();
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit Home' }));
  await fireEvent.change(screen.getByRole('combobox', { name: 'card to add' }), {
    target: { value: 'heat:notes' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add to Home' }));
  expect(await screen.findByRole('img', { name: /Notes, day by day/ })).toBeTruthy();
});

it('the journal: no hint, and on the phone it takes the screen while you write, then gives it back', async () => {
  width = 400;
  const { container } = show();
  const list = await grid();
  const entry = await within(list).findByRole('region', { name: /^journal, / });
  expect(entry.textContent).not.toMatch(/Write about today/);
  const home = container.querySelector('.home') as HTMLElement;
  const editor = await vi.waitFor(() => {
    const el = entry.querySelector('.cm-content') as HTMLElement | null;
    if (!el) throw new Error('no editor yet');
    return el;
  });
  await fireEvent.focusIn(editor);
  expect(home.classList.contains('writing')).toBe(true);
  await fireEvent.focusOut(editor, { relatedTarget: document.body });
  expect(home.classList.contains('writing')).toBe(false);
});
