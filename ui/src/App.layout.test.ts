/// **One view at a time is the default, and a record written before that is moved exactly once.**
///
/// The layout arrangements are pure CSS (`decisions.md`, 2026-08-31), so jsdom can never say which
/// pane is *visible*. What it can say — and what actually broke things — is what ends up in
/// storage and which cell carries `.active`. Both of those are structure, not geometry.
///
/// `migrateWorkspace` itself is unit-tested in `lib/panes.test.ts`; this file pins that `App`
/// actually routes through it, which is the half a pure test cannot reach.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults } from './lib/mock';
import { newPane, SCHEMA } from './lib/panes';

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
});

const saved = () => JSON.parse(store.get('fm-workspace') ?? 'null');

test('a fresh install opens one view at a time, in one column', async () => {
  render(App);
  await screen.findByRole('navigation', { name: 'views' });
  await waitFor(() => expect(saved()).not.toBeNull());
  const w = saved();
  expect(w.layout).toBe('single');
  // Not two. `cols` was only recomputed on add/close, so a fresh wide session used to lay the
  // single board into a two-column grid and leave the second column blank.
  expect(w.cols).toBe(1);
  expect(w.v).toBe(SCHEMA);
});

test('a workspace saved before the flip is moved once, and a chosen `tiled` is left alone', async () => {
  store.set(
    'fm-workspace',
    JSON.stringify({ cols: 2, layout: 'auto', panes: [newPane('board'), newPane('agenda')] }),
  );
  render(App);
  await screen.findByRole('navigation', { name: 'views' });
  // `layout` was written by the very first persist, so a stored `auto` is the old default rather
  // than a decision — and `auto` no longer exists to honour.
  await waitFor(() => expect(saved().layout).toBe('single'));
  expect(saved().v).toBe(SCHEMA);
});

test('a deliberate `tiled` survives, because it was never a default', async () => {
  store.set(
    'fm-workspace',
    JSON.stringify({ cols: 2, layout: 'tiled', panes: [newPane('board'), newPane('agenda')] }),
  );
  render(App);
  await screen.findByRole('navigation', { name: 'views' });
  await waitFor(() => expect(saved()).not.toBeNull());
  expect(saved().layout).toBe('tiled');
});

/// **The blank-screen guard**, and a sibling of `App.boot.test.ts` for the same reason: a stored
/// number should never be able to render nothing. `closePane` clamped `active`; load did not. The
/// old tiled default hid it, because every cell was on screen whatever `active` said — under one
/// view at a time there is simply no `.cell.active`, and every other cell is hidden.
test('an active pane past the end of the list still renders a view', async () => {
  store.set(
    'fm-workspace',
    JSON.stringify({ cols: 1, layout: 'single', panes: [newPane('board')], active: 4 }),
  );
  render(App);
  await screen.findByRole('navigation', { name: 'views' });
  await waitFor(() => expect(document.querySelectorAll('.cell.active').length).toBe(1));
  expect(saved().active).toBe(0);
});

/// **One row of chrome, and it belongs to the view you are looking at.**
///
/// The pane header was the second row: it named the window and carried the window's controls,
/// directly above a bar that named the window. With one view filling the screen there is nothing
/// for it to disambiguate, so it goes and the bar carries both jobs. jsdom applies no CSS, so
/// what is asserted is which elements exist — which is the real change here anyway, because the
/// header is not merely hidden, it is not rendered.
test('one view at a time has no pane header — the bar names the view and carries its controls', async () => {
  render(App);
  const bar = await screen.findByRole('navigation', { name: 'open views' });

  expect(document.querySelector('.pane-head'), 'no second row of chrome').toBeNull();
  // The bar names every open view and marks the one you are on. (A board carries no controls of
  // its own any more — grouping left the UI on 2026-08-31 — so the marker is the tab itself.)
  expect(bar.querySelector('[aria-current="page"]')?.textContent).toMatch(/Board/);
});

test('tiled keeps its pane headers, because that is what tells four windows apart', async () => {
  store.set(
    'fm-workspace',
    JSON.stringify({
      v: SCHEMA,
      cols: 2,
      layout: 'tiled',
      active: 0,
      panes: [newPane('board'), newPane('agenda')],
    }),
  );
  render(App);
  await waitFor(() => expect(document.querySelectorAll('.pane-head').length).toBe(2));
  // And the bar is not mounted at all there: every window is already on screen, so a list of them
  // would be a second copy of what the headers already say.
  expect(screen.queryByRole('navigation', { name: 'open views' })).toBeNull();
});

/// A filter that hides something must say so on its face. The chip row only said it in colour,
/// which is how a vault hidden weeks ago comes to read as notes that have gone missing.
test('the vault filter is a menu that stays open, and says how much it is hiding', async () => {
  render(App);
  const trigger = await screen.findByRole('button', { name: 'which vaults to show' });
  expect(trigger.textContent).toMatch(/All vaults/);

  await fireEvent.click(trigger);
  const items = await screen.findAllByRole('menuitemcheckbox');
  expect(items.length).toBeGreaterThan(1);
  expect(items[0].getAttribute('aria-checked')).toBe('true');

  // Clicking a vault toggles it WITHOUT closing — a filter is many-of-many, unlike every other
  // menu in this app, and closing after each pick would make setting two of them a chore.
  await fireEvent.click(items[0]);
  expect(screen.queryAllByRole('menuitemcheckbox').length).toBe(items.length);
  await waitFor(() => expect(items[0].getAttribute('aria-checked')).toBe('false'));
  await waitFor(() => expect(trigger.textContent).toMatch(/of \d+ vaults/));

  // Escape closes it. None of the inherited single-shot menus handle Escape; one that stays open
  // has to, or a keyboard user has no way out.
  await fireEvent.keyDown(window, { key: 'Escape' });
  await waitFor(() => expect(screen.queryAllByRole('menuitemcheckbox').length).toBe(0));
});
