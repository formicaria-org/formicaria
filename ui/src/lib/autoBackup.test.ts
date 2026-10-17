import { describe, expect, it } from 'vitest';
import {
  GAP_MS,
  LEFT_NOTE_GAP_MS,
  QUIET_MS,
  RETRY_MS,
  STUCK_AFTER_MS,
  dueVaults,
  unsentLabel,
  unsentTitle,
  unsentVaults,
  type Standing,
} from './autoBackup';

const NOW = 1_800_000_000_000;
const secondsAgo = (s: number) => Math.floor(NOW / 1000) - s;
const row = (over: Partial<Standing> = {}): Standing => ({
  vault: 'v',
  last_commit: secondsAgo(600),
  last_sent: secondsAgo(3600),
  unsent: 3,
  ...over,
});

describe('when formicaria backs up by itself', () => {
  it('backs up a vault that holds changes never sent', () => {
    expect(dueVaults([row()], [], {}, NOW)).toEqual(['v']);
  });

  it('leaves alone a vault with nothing to send and nothing to get', () => {
    expect(dueVaults([row({ unsent: 0 })], [], {}, NOW)).toEqual([]);
  });

  it("gets the other device's changes even when there is nothing to send", () => {
    expect(dueVaults([row({ unsent: 0 })], ['v'], {}, NOW)).toEqual(['v']);
  });

  it("never makes a first backup: where notes go is somebody's decision", () => {
    expect(dueVaults([row({ last_sent: null, unsent: null })], [], {}, NOW)).toEqual([]);
    expect(dueVaults([row({ last_sent: null, unsent: 9 })], ['v'], {}, NOW)).toEqual([]);
  });

  it('waits until writing has been still, so a burst of typing is one backup', () => {
    const typing = row({ last_commit: secondsAgo(QUIET_MS / 1000 - 5) });
    expect(dueVaults([typing], [], {}, NOW)).toEqual([]);
    expect(dueVaults([row({ last_commit: secondsAgo(QUIET_MS / 1000) })], [], {}, NOW)).toEqual([
      'v',
    ]);
  });

  it('does not wait for stillness when the app is being put away', () => {
    const typing = row({ last_commit: secondsAgo(2) });
    expect(dueVaults([typing], [], {}, NOW, 'leaving')).toEqual(['v']);
  });

  it('keeps a gap after a backup, and a longer one after a failure', () => {
    const at = (ms: number, ok: boolean) => ({ v: { at: NOW - ms, ok } });
    expect(dueVaults([row()], [], at(GAP_MS - 1, true), NOW)).toEqual([]);
    expect(dueVaults([row()], [], at(GAP_MS, true), NOW)).toEqual(['v']);
    expect(dueVaults([row()], [], at(GAP_MS, false), NOW)).toEqual([]);
    expect(dueVaults([row()], [], at(RETRY_MS, false), NOW)).toEqual(['v']);
    // Leaving does not override the gaps: an offline phone must not retry on every screen-off.
    expect(dueVaults([row()], [], at(GAP_MS, false), NOW, 'leaving')).toEqual([]);
  });

  it('decides each vault on its own', () => {
    const rows = [row({ vault: 'a' }), row({ vault: 'b', unsent: 0 }), row({ vault: 'c' })];
    expect(dueVaults(rows, [], { c: { at: NOW - 1000, ok: true } }, NOW)).toEqual(['a']);
  });
});

// **Occasions besides the tick** (2026-10-17, `decisions.md#sync`): coming back, starting to edit,
// leaving a note and the network returning each relax a different wait, and only that one.
describe('the occasions that back up between ticks', () => {
  const at = (ms: number, ok: boolean) => ({ v: { at: NOW - ms, ok } });
  const typing = row({ last_commit: secondsAgo(2) });

  it('coming back backs up at once, past the quiet wait and the gap', () => {
    expect(dueVaults([typing], [], at(1000, true), NOW, 'returning')).toEqual(['v']);
    expect(dueVaults([row({ unsent: 0 })], ['v'], at(1000, true), NOW, 'returning')).toEqual(['v']);
  });

  it('coming back still waits after a failure, so an offline phone does not retry on every unlock', () => {
    expect(dueVaults([row()], [], at(1000, false), NOW, 'returning')).toEqual([]);
  });

  it('starting to edit backs up only when another device has sent changes', () => {
    expect(dueVaults([typing], ['v'], at(1000, true), NOW, 'editing')).toEqual(['v']);
    expect(dueVaults([typing], [], {}, NOW, 'editing')).toEqual([]);
    expect(dueVaults([row()], ['v'], at(1000, false), NOW, 'editing')).toEqual([]);
  });

  it('leaving a note skips the quiet wait and keeps a short gap instead of the long one', () => {
    expect(dueVaults([typing], [], {}, NOW, 'left-note')).toEqual(['v']);
    expect(dueVaults([typing], [], at(LEFT_NOTE_GAP_MS - 1, true), NOW, 'left-note')).toEqual([]);
    expect(dueVaults([typing], [], at(LEFT_NOTE_GAP_MS, true), NOW, 'left-note')).toEqual(['v']);
    expect(LEFT_NOTE_GAP_MS).toBeLessThan(GAP_MS);
  });

  it('the network coming back skips the wait after a failure', () => {
    expect(dueVaults([row()], [], at(1000, false), NOW, 'online')).toEqual(['v']);
  });

  it('none of them makes a first backup', () => {
    for (const o of ['returning', 'editing', 'left-note', 'online'] as const) {
      expect(dueVaults([row({ last_sent: null })], ['v'], {}, NOW, o)).toEqual([]);
    }
  });
});

// **A send that has not happened is shown** — and one of the two signals must not depend on the
// loop working, because the five days the phone held its notes were a loop that never ran.
describe('saying that changes have not left this device', () => {
  it('says so when the last automatic try failed, with its reason', () => {
    const u = unsentVaults(
      [row()],
      { v: { at: NOW - 1000, ok: false, phase: 'failed', error: 'could not resolve host' } },
      NOW,
      true,
    );
    expect(u).toEqual([expect.objectContaining({ vault: 'v', kind: 'failed', unsent: 3 })]);
    expect(unsentLabel(u)).toBe('v: 3 not sent');
    expect(
      unsentTitle(
        u,
        (v) => v,
        () => 'There is no connection.',
      ),
    ).toContain('There is no connection.');
  });

  it('says so when changes have waited too long, with no failure to point at', () => {
    const old = row({ last_commit: secondsAgo(STUCK_AFTER_MS / 1000) });
    expect(unsentVaults([old], {}, NOW, true)).toEqual([
      expect.objectContaining({ kind: 'stuck', minutes: STUCK_AFTER_MS / 60_000 }),
    ]);
    const fresh = row({ last_commit: secondsAgo(STUCK_AFTER_MS / 1000 - 60) });
    expect(unsentVaults([fresh], {}, NOW, true)).toEqual([]);
  });

  it('names a conflict as the reason, not as a failure', () => {
    const u = unsentVaults([row()], { v: { at: NOW, ok: false, phase: 'conflicts' } }, NOW, true);
    expect(unsentTitle(u)).toContain('conflicting edits');
  });

  it('is silent when all is sent, when nothing was ever sent, and when the switch is off', () => {
    const old = { last_commit: secondsAgo(STUCK_AFTER_MS) };
    expect(unsentVaults([row({ ...old, unsent: 0 })], {}, NOW, true)).toEqual([]);
    expect(unsentVaults([row({ ...old, last_sent: null })], {}, NOW, true)).toEqual([]);
    expect(unsentVaults([row(old)], {}, NOW, false)).toEqual([]);
  });

  it('uses no git words', () => {
    const u = unsentVaults([row({ last_commit: secondsAgo(STUCK_AFTER_MS) })], {}, NOW, true);
    const words = `${unsentLabel(u)} ${unsentTitle(u)}`.toLowerCase();
    for (const w of ['push', 'pull', 'commit', 'remote', 'merge']) expect(words).not.toContain(w);
  });
});
