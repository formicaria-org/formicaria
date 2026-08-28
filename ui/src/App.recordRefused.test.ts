/// **The button that reported success and did nothing.**
///
/// `record_unrecorded` answers `{committed, notes, reason}`. `committed: false` has two meanings —
/// there was nothing to record, and git *refused* to record what was found — and the UI collapsed both
/// into “Nothing left to record”. So on the owner's phone, with 147 notes outstanding and a note
/// mid-merge, the one action that rescues unrecorded notes reassured them, indefinitely (2026-07-31).
///
/// A silent refusal on a data-safety action is worse than an error, because it stops the user looking.

import { render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { setRecordRefused, setUnrecorded } from './lib/mock';

beforeEach(() => {
  setRecordRefused(false);
  setUnrecorded([]);
});

async function openPanel() {
  const chip = await screen.findByRole('button', { name: /not in history/i });
  chip.click();
  return await screen.findByRole('button', { name: /Record all/i });
}

test('a refused recording says why, and does not claim there was nothing to record', async () => {
  setUnrecorded([{ vault: 'v', count: 147, new: 142, modified: 4, deleted: 1, notes: [] }]);
  setRecordRefused(true);
  render(App);

  (await openPanel()).click();

  // The reason, verbatim from the backend — including where to go to clear it.
  await waitFor(() => expect(screen.getByText(/mid-merge/i)).toBeTruthy());
  expect(screen.getByText(/Could not record the 147 notes/i)).toBeTruthy();
  // And emphatically not the reassurance.
  expect(screen.queryByText(/Nothing left to record/i)).toBeNull();
});

test('an empty vault still reports plainly, without inventing a failure', async () => {
  setUnrecorded([{ vault: 'v', count: 3, new: 3, modified: 0, deleted: 0, notes: [] }]);
  render(App);
  (await openPanel()).click();
  await waitFor(() => expect(screen.getByText(/Recorded 3 notes/i)).toBeTruthy());
  expect(screen.queryByText(/Could not record/i)).toBeNull();
});
