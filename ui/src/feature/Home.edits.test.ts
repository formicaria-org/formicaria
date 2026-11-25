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
    ).toContain('span12;'),
  );
  await fireEvent.click(within(await menu('Continue')).getByRole('button', { name: 'Tall' }));
  await vi.waitFor(() =>
    expect(
      cardNamed(list, /Continue/)
        ?.getAttribute('style')
        ?.replace(/\s/g, ''),
    ).toMatch(/grid-row:\d+\/span8;?$/),
  );
  await fireEvent.click(
    within(await menu('Continue')).getByRole('button', { name: 'Remove from Home' }),
  );
  await vi.waitFor(() => expect(cardNamed(list, /Continue/)).toBeUndefined());
});

it('wide: a card shown as a glance becomes a small figure on the grid, where it was', async () => {
  show();
  const list = await grid();
  await within(list).findByRole('heading', { name: /Tasks board/ });
  await fireEvent.click(within(await menu('Tasks board')).getByRole('button', { name: 'Glance' }));
  await vi.waitFor(() => expect(within(list).getByText(/open in Tasks/)).toBeTruthy());
  const tile = cardNamed(list, /open in Tasks/)!;
  expect(tile.classList.contains('tile-card')).toBe(true);
  expect(tile.getAttribute('style')?.replace(/\s/g, '')).toContain('span3;');
  expect(screen.queryByRole('list', { name: 'at a glance' })).toBeNull();
});

it('wide: a card moved with its grip’s arrow keys keeps its new place across a reload', async () => {
  const first = show();
  const list = await grid();
  await within(list).findByRole('heading', { name: /Continue/ });
  const place = () =>
    cardNamed(list, /Continue/)
      ?.getAttribute('style')
      ?.replace(/\s/g, '')
      .match(/grid-column:(\d+)\/span\d+;grid-row:(\d+)\//)
      ?.slice(1)
      .map(Number);
  const [x0, y0] = place()!;
  const grip = within(cardNamed(list, /Continue/)!).getByRole('button', { name: 'move Continue' });
  await fireEvent.keyDown(grip, { key: x0 > 1 ? 'ArrowLeft' : 'ArrowRight' });
  await vi.waitFor(() => expect(place()![0]).not.toBe(x0));
  const moved = place()!;
  expect(moved[1]).toBeLessThanOrEqual(y0 + 1);
  // Written into the Home note before Home is drawn again.
  await vi.waitFor(async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home', limit: 1 })) as { id: string }[];
    const d = (await mock.handle('get', { id: h.id })) as { body: string };
    expect(d.body).toMatch(new RegExp(`card:continue\\?[^)]*&x=${moved[0] - 1}&`));
  });
  first.unmount();
  show();
  const again = await grid();
  // The saved Home, once read (the generated one is drawn until it comes).
  await vi.waitFor(() => {
    const kept = cardNamed(again, /Continue/)
      ?.getAttribute('style')
      ?.replace(/\s/g, '')
      .match(/grid-column:(\d+)\//)?.[1];
    expect(Number(kept)).toBe(moved[0]);
  });
});

it('wide: ⋯ → Move and Tidy up rearrange the grid, and no two cards overlap', async () => {
  show();
  const list = await grid();
  await within(list).findByRole('heading', { name: /Continue/ });
  await fireEvent.click(
    within(await menu('Continue')).getByRole('button', { name: 'move Continue down' }),
  );
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit Home' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Tidy up' }));
  await vi.waitFor(() => {
    const boxes = within(list)
      .getAllByRole('listitem')
      .filter((li) => li.parentElement === list)
      .map((li) => {
        const m = li
          .getAttribute('style')!
          .replace(/\s/g, '')
          .match(/grid-column:(\d+)\/span(\d+);grid-row:(\d+)\/span(\d+)/)!
          .slice(1)
          .map(Number);
        return { x: m[0], w: m[1], y: m[2], h: m[3] };
      });
    for (const a of boxes)
      for (const b of boxes)
        if (a !== b)
          expect(a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h).toBe(
            false,
          );
  });
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
