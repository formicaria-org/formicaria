// **A view that failed to load is not a notebook that is empty.**
//
// `refresh()` swallowed every feed error with `.catch(() => ({}) as Feed)`, so `cards` became `[]`
// and `Timeline` drew its "No notes yet" empty state. There is no loading state either, so an
// unresolved feed looked identical to an empty vault. Any backend failure on a read therefore
// presented to the user as *"you have written nothing"* — the same family of lie as the four
// messages corrected on 2026-09-08, and the one that made a hidden-vault filter read as lost notes.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test } from 'vitest';
import App from './App.svelte';
import { clearFaults, faults, reset } from './lib/mock';

beforeEach(() => {
  reset();
  clearFaults();
});
afterEach(() => reset());

test('a view that could not load says so, instead of showing an empty notebook', async () => {
  // The timeline pane reads `recent`. Refuse it the way a broken vault would.
  // Whichever feed the default workspace shows — the point is that a read failed, not which.
  faults(
    ['recent', 'board', 'agenda', 'run_view', 'search'].map((cmd) => ({
      cmd,
      mode: 'reject' as const,
      message: 'vault is unreadable',
    })),
  );

  render(App);

  expect(await screen.findByText(/Could not load this view: vault is unreadable/i)).toBeTruthy();
});

test('and the message goes when the next refresh works', async () => {
  // Left transient on purpose: a read that failed once is not news worth outliving the poll that
  // fixes it. `times: 1` lets the retry succeed, exactly as a recovered backend would.
  faults(
    ['recent', 'board', 'agenda', 'run_view', 'search'].map((cmd) => ({
      cmd,
      mode: 'reject' as const,
      message: 'briefly unreachable',
      times: 1,
    })),
  );

  render(App);
  await screen.findByText(/briefly unreachable/i);

  // Anything that triggers a refresh; the vault filter is the cheapest control on screen.
  const { handle } = await import('./lib/mock');
  await handle('capture', { body: 'a note that forces a refresh' });

  // The empty state is the honest answer once the read succeeds and there is genuinely nothing.
  await screen.findByRole('button', { name: /^back up$/i });
});
