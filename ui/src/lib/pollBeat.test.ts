// The discussion poll's backoff policy, tested as arithmetic rather than by sleeping.
//
// What it protects: an open discussion used to poll at a fixed 1.5 s forever. On Android each poll
// is a blocking IPC round trip that parks the JS thread, so a note left open with its thread
// showing was a permanent tax on a battery device for information nobody was waiting on.
import { describe, expect, it } from 'vitest';
import { beatMs, POLL_BUSY_MS, POLL_IDLE_MS } from './pollBeat';

describe('the discussion poll beat', () => {
  it('stays fast while a turn is in flight', () => {
    expect(beatMs(true)).toBe(POLL_BUSY_MS);
  });

  it('backs off when nothing is happening', () => {
    expect(beatMs(false)).toBe(POLL_IDLE_MS);
    expect(POLL_IDLE_MS).toBeGreaterThan(POLL_BUSY_MS);
  });

  it('never stops polling entirely', () => {
    // A beat of 0 or Infinity is not a fix — the first is the bug again, the second leaves an
    // agent's reply invisible until the pane is reopened. Both directions are bounded on purpose.
    for (const busy of [true, false]) {
      expect(beatMs(busy)).toBeGreaterThan(0);
      expect(beatMs(busy)).toBeLessThanOrEqual(10_000);
    }
  });
});
