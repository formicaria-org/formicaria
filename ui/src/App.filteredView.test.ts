/// **"My done column in the board is not showing up, even though I have done notes."** (2026-08-24)
///
/// The notes were there and the built-in Board had the column. What the owner was looking at was a
/// saved `.view` — `view: board`, with a filter that removes one status — and a view board draws
/// through the very same `Board.svelte` as the Board pane. Same pixels, one column fewer, and
/// nothing on screen to say why: a filtered view made its filter look like missing notes.
///
/// It is worse than an ordinary bit of hidden state, because a `.view` file cannot be written,
/// edited or deleted from the UI — and the UI is the only place the owner works. So the filter was
/// unreachable as well as unseen.
///
/// The rule this pins is the one the manual already applies to a *broken* view ("a broken view
/// tells you why"): **a working view must also say what it removes, and offer the way out.**
/// The words come from the server (`ViewInfo.filters`, built in `views.rs`), so this test asserts
/// on *a view naming its filter*, never on a status — the board is group-by-anything and the UI
/// carries no status literals.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, MOCK_VIEW_HIDDEN } from './lib/mock';

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

/// Open the app with one pane already on the saved *filtered board* view — seeded through the
/// app's own persistence format rather than by driving the UI to it — which is also the only way
/// now that a pane header names its window instead of switching it.
function openFilteredView() {
  store.set(
    'fm-workspace',
    JSON.stringify({
      cols: 1,
      layout: 'single',
      active: 0,
      panes: [{ id: 1, kind: 'view', viewName: 'Active', colSpan: 1, rowSpan: 1 }],
    }),
  );
  render(App);
}

test('a view that hides a column says so, and one click brings the column back', async () => {
  openFilteredView();

  // The header names the filter in the view's own words, and says how to leave it. The accessible
  // name is the whole sentence (the chip is one clipped line in a narrow pane), while the *visible*
  // text still carries the meaning — a phone cannot hover, so `title` alone would tell nobody.
  const chip = await screen.findByRole('button', { name: /shows only notes where/i });

  // The column the view removes is genuinely absent to begin with — otherwise this test would
  // pass against a board that never filtered anything.
  expect(screen.queryByText(MOCK_VIEW_HIDDEN.value)).toBeNull();

  expect(chip.textContent).toMatch(/filtered:/i);
  expect(chip.textContent).toMatch(new RegExp(MOCK_VIEW_HIDDEN.key, 'i'));
  expect(chip.getAttribute('title')).toMatch(/click to see everything/i);

  await fireEvent.click(chip);

  // The pane is now the built-in board it was shadowing — same grouping, nothing filtered — and
  // the column is back. That is the owner's whole journey.
  // Read the view's name off the bar that names it. There is no pane header when one view fills
  // the window — the bar is the chrome, and it is a label rather than a labelled control, so this
  // asks it for its text rather than inventing an `aria-label` to keep an old query working.
  await vi.waitFor(() =>
    expect(screen.getByRole('navigation', { name: 'open views' }).textContent).toMatch(/board/i),
  );
  await vi.waitFor(() =>
    expect(screen.getAllByText(MOCK_VIEW_HIDDEN.value).length).toBeGreaterThan(0),
  );
  // And the explanation goes with it: an unfiltered board has nothing to disclose.
  expect(screen.queryByRole('button', { name: /shows only notes where/i })).toBeNull();
});

test('an unfiltered view says nothing — the chip is about a filter, not about being a view', async () => {
  store.set(
    'fm-workspace',
    JSON.stringify({
      cols: 1,
      layout: 'single',
      active: 0,
      panes: [{ id: 1, kind: 'view', viewName: 'Recent notes', colSpan: 1, rowSpan: 1 }],
    }),
  );
  render(App);

  await vi.waitFor(() =>
    expect(screen.getByRole('navigation', { name: 'open views' }).textContent).toMatch(/Recent/i),
  );
  expect(screen.queryByRole('button', { name: /shows only notes where/i })).toBeNull();
});
