/// **The other half of "a view can be made from the app".**
///
/// `delete_view` shipped end to end — the dispatch arm, `ipc.ts`, the mock — and **nothing ever
/// called it**. A view could be created from the UI and then removed only with a file manager, in
/// an app whose owner works only through the UI. A capability with no way in is the failure
/// `outstanding.md` §2.6b is about, and it is the same one that made "Save view" necessary.
///
/// The second thing this pins is the *vault*: `list_views` spans every vault, so deleting by name
/// alone resolves against the default one and reports success having deleted nothing.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';

const { deleteView } = vi.hoisted(() => ({ deleteView: vi.fn() }));
vi.mock('./lib/ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/ipc')>()),
  deleteView,
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
  deleteView.mockResolvedValue([]);
  // A confirm() would let this pass with no on-screen affordance at all, which is the point.
  vi.stubGlobal('confirm', () => {
    throw new Error('window.confirm must not be how a view is deleted');
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

function openPane(pane: Record<string, unknown>) {
  store.set(
    'fm-workspace',
    JSON.stringify({
      cols: 1,
      layout: 'single',
      active: 0,
      panes: [{ id: 1, colSpan: 1, rowSpan: 1, ...pane }],
    }),
  );
  render(App);
}

test('a saved view can be deleted from the app', async () => {
  openPane({ kind: 'view', viewName: 'Active' });
  const btn = await screen.findByRole('button', { name: 'delete this view' });

  // One click only arms it — a file in someone's vault must not go on a stray click.
  await fireEvent.click(btn);
  expect(deleteView).not.toHaveBeenCalled();
  await screen.findByRole('button', { name: 'confirm deleting this view' });

  await fireEvent.click(screen.getByRole('button', { name: 'confirm deleting this view' }));
  // The mock's 'Active' view lives in the mock's vault; whatever `list_views` reported is what
  // must be sent back, never a guess at the default.
  await waitFor(() => expect(deleteView).toHaveBeenCalledWith('Active', expect.anything()));
});

test('a board is not offered a delete button — there is no file to delete', async () => {
  openPane({ kind: 'board', groupBy: 'status' });
  await screen.findByRole('button', { name: /save this view/i });
  expect(screen.queryByRole('button', { name: 'delete this view' })).toBeNull();
});
