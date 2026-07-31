// **Why 142 notes were never committed** — the dedicated test for the cause.
//
// The auto-commit is debounced: every write does `clearTimeout` and re-arms 5 s out. That is right for
// typing, and wrong for a *burst*: each write pushes the deadline further away, so a run of writes
// defers the commit indefinitely. It does not fire late — it never fires. On the owner's phone
// (2026-07-31) something wrote ~142 notes inside one minute, each resetting this timer; when the
// process ended, the pending commit died with the tab and the in-memory write-record died with the
// process, leaving those notes unstageable for good.
//
// The fix is a ceiling: still quiet-period debounced, but never deferred past `COMMIT_MAX_WAIT_MS`
// after the *first* pending write. This test is what pins it, and it fails against the uncapped
// version — verified by reverting the cap.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { commit, ping, capture } = vi.hoisted(() => ({
  commit: vi.fn(),
  ping: vi.fn(),
  capture: vi.fn(),
}));
vi.mock('./lib/ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/ipc')>()),
  commit,
  ping,
  capture,
}));

import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  mock.clearFaults();
  commit.mockResolvedValue({ committed: true, conflicts: [] });
  // `ping` must answer `git: true`, or `scheduleCommit` returns early and this test would pass for
  // the wrong reason.
  ping.mockResolvedValue({ changed: false, generation: 0, git: true, restic: true, skipped: [], unopened_vaults: [] });
  capture.mockImplementation(async () => ({
    id: `M0CK${String(Math.floor(Math.random() * 1e6)).padStart(22, '0')}`,
    type: 'note',
    title: null,
    preview: 'x',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-07-31T07:44:00Z',
    updated: '2026-07-31T07:44:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: 'personal',
  }));
});
afterEach(() => vi.useRealTimers());

/// One real write through the app's own path: open the create menu, pick "New note". That calls
/// `capture` and then `scheduleCommit`, which is what is under test — going through the UI rather than
/// poking internals means the test still means something if the write paths are rearranged.
async function writeOnce() {
  await fireEvent.click(screen.getByRole('button', { name: /make something new/i }));
  await fireEvent.click(await screen.findByText('New note'));
}

async function burst(times: number, everyMs: number) {
  for (let i = 0; i < times; i += 1) {
    await writeOnce();
    await vi.advanceTimersByTimeAsync(everyMs);
  }
}

describe('a burst of writes', () => {
  it('still commits, instead of deferring the commit for as long as the burst lasts', async () => {
    render(App);
    await vi.waitFor(() => expect(document.body.querySelector('header.topbar')).toBeTruthy());

    // 40 writes, one every second — a 40-second burst, slower than the real one and enough to prove
    // the point. Under the uncapped debounce this commits **zero** times.
    await burst(40, 1000);

    // The ceiling is 15 s, so a 40-second burst must have produced at least two commits along the way.
    expect(commit.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it('still waits out the quiet period for ordinary typing', async () => {
    render(App);
    await vi.waitFor(() => expect(document.body.querySelector('header.topbar')).toBeTruthy());

    await writeOnce();
    // Before the quiet period is up, nothing has been committed — the debounce still exists, and a
    // commit per keystroke is what it is there to prevent.
    await vi.advanceTimersByTimeAsync(3000);
    expect(commit).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(3000);
    expect(commit).toHaveBeenCalled();
  });
});
