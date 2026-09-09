/// **"When I back up I still see 1 not in history — isn't that what backup is for?"** (2026-08-24)
///
/// It was. `backUpNotes` runs `commit → push` over every vault, and the commit is exactly the step
/// that moves a note *into* history — the backend half of this is pinned in
/// `crates/fm-app/tests/backup_records_everything.rs`. What was missing is that the chip reporting
/// the count was loaded **once per `vaults` change**, and after the panel's own Record button, and
/// nowhere else. So the notes were recorded and the toolbar went on displaying the number it had
/// read at load.
///
/// From the only place the owner works, that is indistinguishable from a backup that does not
/// record. A durability affordance that appears not to have worked is one people press again, or
/// stop trusting — so the count, which is a fact about git, is re-read at the moments git changes:
/// the Backup button, and the debounced auto-commit underneath every write.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { backUpFromToolbar } from './lib/harness';
import { clearFaults, setUnrecorded } from './lib/mock';

beforeEach(() => clearFaults());
afterEach(() => {
  vi.useRealTimers();
  clearFaults();
});

test('backing up records the outstanding notes, and the chip stops claiming otherwise', async () => {
  setUnrecorded([{ vault: 'personal', count: 1, new: 1, modified: 0, deleted: 0, notes: [] }]);
  render(App);

  // The complaint's starting state: one note on disk that git does not have.
  await screen.findByRole('button', { name: /1 not in history/i });

  await backUpFromToolbar(screen, fireEvent);

  // Backup committed it, so nothing is outstanding — and the toolbar has to say so without a
  // reload. `queryByRole` and not a text match: the chip is a button, and its absence is the claim.
  await vi.waitFor(() =>
    expect(screen.queryByRole('button', { name: /not in history/i })).toBeNull(),
  );
});

test('the auto-commit clears it too, so the chip is never left behind by an ordinary write', async () => {
  // The same fact by the path nobody presses. Refreshing only in the Backup handler would leave a
  // vault that had already auto-committed showing a count for notes git has had for minutes —
  // which is the same false alarm, just slower to notice.
  vi.useFakeTimers();
  setUnrecorded([{ vault: 'personal', count: 4, new: 4, modified: 0, deleted: 0, notes: [] }]);
  render(App);
  await vi.waitFor(() =>
    expect(screen.getByRole('button', { name: /4 not in history/i })).toBeTruthy(),
  );

  // One real write through the app's own path — it is `scheduleCommit` that follows it that matters.
  await fireEvent.click(screen.getByRole('button', { name: /make something new/i }));
  await fireEvent.click(await screen.findByText('New note'));

  // Past the 5 s quiet period, the commit has fired and the count is stale.
  await vi.advanceTimersByTimeAsync(6000);
  await vi.waitFor(() =>
    expect(screen.queryByRole('button', { name: /not in history/i })).toBeNull(),
  );
});
