import { describe, expect, it } from 'vitest';
import { GAP_MS, QUIET_MS, RETRY_MS, dueVaults, type Standing } from './autoBackup';

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
    expect(dueVaults([typing], [], {}, NOW, true)).toEqual(['v']);
  });

  it('keeps a gap after a backup, and a longer one after a failure', () => {
    const at = (ms: number, ok: boolean) => ({ v: { at: NOW - ms, ok } });
    expect(dueVaults([row()], [], at(GAP_MS - 1, true), NOW)).toEqual([]);
    expect(dueVaults([row()], [], at(GAP_MS, true), NOW)).toEqual(['v']);
    expect(dueVaults([row()], [], at(GAP_MS, false), NOW)).toEqual([]);
    expect(dueVaults([row()], [], at(RETRY_MS, false), NOW)).toEqual(['v']);
    // Leaving does not override the gaps: an offline phone must not retry on every screen-off.
    expect(dueVaults([row()], [], at(GAP_MS, false), NOW, true)).toEqual([]);
  });

  it('decides each vault on its own', () => {
    const rows = [row({ vault: 'a' }), row({ vault: 'b', unsent: 0 }), row({ vault: 'c' })];
    expect(dueVaults(rows, [], { c: { at: NOW - 1000, ok: true } }, NOW)).toEqual(['a']);
  });
});
