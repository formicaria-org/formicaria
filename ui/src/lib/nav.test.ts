// Back's copy of the stack, driven directly (`nav.svelte.ts`; `decisions.md#ui`, 2026-10-20).
import { afterEach, beforeEach, expect, test } from 'vitest';
import * as nav from './nav.svelte';

const board = { kind: 'board' };
const note = (id: string) => ({ kind: 'note', noteId: id });
const state = () => history.state as { nav: { kind: string; noteId?: string }; layer?: string };

// What App does with every Back: hand it to `nav`.
const onPop = (e: PopStateEvent) => void nav.popped(e.state);
beforeEach(() => {
  nav.reset();
  nav.start(board);
  window.addEventListener('popstate', onPop);
});
afterEach(async () => {
  await nav.idle();
  window.removeEventListener('popstate', onPop);
});

test('closing a note from an open window list steps back under the list, which stays', async () => {
  nav.visit(note('a'));
  let open = true;
  nav.openLayer('windows', () => (open = false));
  await nav.idle();
  expect(state().layer).toBe('windows');

  nav.leave('a', board);
  await nav.idle();
  // The note's entry is gone; the list's is back on top, over the board.
  expect(state().layer).toBe('windows');
  expect(state().nav.kind).toBe('board');
  expect(open).toBe(true);
});

test('a layer closed under one opened from it takes both entries back', async () => {
  let category = true;
  nav.openLayer('settings', () => undefined);
  await nav.idle();
  nav.openLayer('settings-category', () => (category = false));
  await nav.idle();
  nav.closeLayer('settings');
  expect(category).toBe(false);
  await nav.idle();
  expect(state().layer).toBeUndefined();
});

test('a menu handing over to a panel swaps its entry for the panel’s', async () => {
  nav.openLayer('menu', () => undefined);
  await nav.idle();
  const before = history.length;
  nav.closeLayer('menu');
  nav.openLayer('panel', () => undefined);
  await nav.idle();
  expect(state().layer).toBe('panel');
  expect(history.length).toBe(before);
});
