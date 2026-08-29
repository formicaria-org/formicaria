/// **Making a filtered view no longer needs a text editor.**
///
/// `save_view` could write a renderer and a group-by and nothing else, so the only way to have a
/// view that filters anything was to author YAML by hand — for a headline feature, in an app whose
/// owner works only through the UI (`outstanding.md` §2.6b). The naming step was also a
/// `window.prompt()`, which is genuinely in-app and asks exactly one question.
///
/// The nine-predicate grammar is still not exposed — a UI for it is a query builder, and that
/// ruling stands. One tag is not that.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';

const { saveView } = vi.hoisted(() => ({ saveView: vi.fn() }));
vi.mock('./lib/ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/ipc')>()),
  saveView,
}));

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
  saveView.mockResolvedValue([]);
  // A prompt would make this test pass without a dialog existing, which is the whole point.
  vi.stubGlobal('prompt', () => {
    throw new Error('window.prompt must not be how a view is named');
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

function openBoard() {
  store.set(
    'fm-workspace',
    JSON.stringify({
      cols: 1,
      layout: 'single',
      active: 0,
      panes: [{ id: 1, kind: 'board', groupBy: 'status', colSpan: 1, rowSpan: 1 }],
    }),
  );
  render(App);
}

async function openSaveDialog() {
  openBoard();
  const save = await screen.findByRole('button', { name: /save.*view|save this view/i });
  await fireEvent.click(save);
  return await screen.findByRole('dialog', { name: /save this view/i });
}

test('a view is named in a dialog, not a browser prompt', async () => {
  await openSaveDialog();
  // Both questions are on screen at once — the thing one prompt could never ask.
  expect(screen.getByLabelText('view name')).toBeTruthy();
  expect(screen.getByLabelText('only notes tagged')).toBeTruthy();
});

test('a tag typed here becomes the view filter', async () => {
  await openSaveDialog();
  await fireEvent.input(screen.getByLabelText('view name'), { target: { value: 'Papers' } });
  await fireEvent.input(screen.getByLabelText('only notes tagged'), { target: { value: 'paper' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    // The board's own group-by rides along: the arrangement is the name, the grouping and the tag.
    expect(saveView).toHaveBeenCalledWith('Papers', 'board', 'status', '', 'paper'),
  );
});

test('leaving the tag empty saves an unfiltered view, as before', async () => {
  await openSaveDialog();
  await fireEvent.input(screen.getByLabelText('view name'), { target: { value: 'Everything' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

  await waitFor(() =>
    expect(saveView).toHaveBeenCalledWith('Everything', 'board', 'status', '', ''),
  );
});

test('a nameless view cannot be saved', async () => {
  await openSaveDialog();
  expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled).toBe(true);
});
