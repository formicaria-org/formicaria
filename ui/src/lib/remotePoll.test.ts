// The remote-awareness poll's schedule, per device.
//
// The defect this pins: the effect in `App.svelte` gated only on `import.meta.env.PROD`, and a
// Tauri build *is* `PROD`. So the shipped Android app ran a `git ls-remote` **per vault** every
// 45 seconds, plus one on every window `focus` — on mobile data, on a battery, for a nudge nobody
// was waiting on. It is the third-largest contributor to "it lags", and the only one that costs
// the user money.
//
// `ipc.ts` already carries two comments about this exact trap ("PROD is not a statement about
// which backend is present"). This is the third instance, so the rule is now a tested function
// rather than a comment asking the next reader to remember.
import { describe, expect, it } from 'vitest';
import {
  pollIntervalMs,
  foregroundCheckDue,
  DESKTOP_POLL_MS,
  PHONE_FOREGROUND_GAP_MS,
} from './remotePoll';

describe('the remote-awareness poll', () => {
  it('keeps its timer on the desktop', () => {
    expect(pollIntervalMs(false)).toBe(DESKTOP_POLL_MS);
  });

  it('runs no timer at all on a phone', () => {
    // Not "a longer interval" — none. The phone's schedule is the foreground, below.
    expect(pollIntervalMs(true)).toBeNull();
  });

  it('checks on every desktop focus, and rate-limits on a phone', () => {
    expect(foregroundCheckDue(false, 1_000, 999)).toBe(true);

    // Never checked → always due, whichever device.
    expect(foregroundCheckDue(true, 10_000, 0)).toBe(true);
    // Android fires focus when the screen wakes, when a notification shade closes, when the app
    // is picked from recents — none of which is "the user came back to work".
    expect(foregroundCheckDue(true, 60_000, 59_000)).toBe(false);
    expect(foregroundCheckDue(true, PHONE_FOREGROUND_GAP_MS + 1, 1)).toBe(true);
  });

  it('keeps the phone gap long enough to matter and short enough to be useful', () => {
    // A guard on the constant itself: shrinking it back toward the 45 s interval would quietly
    // undo the fix, and the failure mode (battery, data) is invisible in CI.
    expect(PHONE_FOREGROUND_GAP_MS).toBeGreaterThanOrEqual(60_000);
    expect(PHONE_FOREGROUND_GAP_MS).toBeLessThanOrEqual(30 * 60_000);
  });
});
