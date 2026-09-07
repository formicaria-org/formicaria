import { describe, expect, it } from 'vitest';
import {
  assignLanes,
  clampRangeToWeek,
  dayHeading,
  isoDate,
  weekOf,
  ymd,
  type Day,
} from './calendar';

// The week of 2026-07-13 (a Monday) → 13..19 July, Monday-first.
const week: Day[] = weekOf('2026-07-15');

describe('isoDate', () => {
  it('keeps a bare calendar date untouched (no UTC round-trip)', () => {
    expect(isoDate('2026-07-14')).toBe('2026-07-14');
  });

  it('reduces an RFC3339 datetime to its local calendar day', () => {
    // Local time is midday, so every sane timezone lands on the 14th.
    expect(isoDate('2026-07-14T12:00:00Z')).toBe('2026-07-14');
  });
});

describe('clampRangeToWeek', () => {
  it('places a single-day range in one column, rounded both ends', () => {
    const span = clampRangeToWeek('2026-07-15', '2026-07-15', week);
    expect(span).toEqual({ startCol: 2, endCol: 2, continuesLeft: false, continuesRight: false });
  });

  it('spans a multi-day range within the week', () => {
    // Tue 14th → Fri 17th = columns 1..4 (Monday = col 0).
    const span = clampRangeToWeek('2026-07-14', '2026-07-17', week);
    expect(span).toEqual({ startCol: 1, endCol: 4, continuesLeft: false, continuesRight: false });
  });

  it('clamps a range that starts before the week and marks continuesLeft', () => {
    const span = clampRangeToWeek('2026-07-10', '2026-07-15', week);
    expect(span).toEqual({ startCol: 0, endCol: 2, continuesLeft: true, continuesRight: false });
  });

  it('clamps a range that ends after the week and marks continuesRight', () => {
    const span = clampRangeToWeek('2026-07-17', '2026-07-25', week);
    expect(span).toEqual({ startCol: 4, endCol: 6, continuesLeft: false, continuesRight: true });
  });

  it('spans the whole row and continues on both sides', () => {
    const span = clampRangeToWeek('2026-07-01', '2026-07-31', week);
    expect(span).toEqual({ startCol: 0, endCol: 6, continuesLeft: true, continuesRight: true });
  });

  it('returns null for a range entirely before or after the week', () => {
    expect(clampRangeToWeek('2026-07-01', '2026-07-05', week)).toBeNull();
    expect(clampRangeToWeek('2026-07-20', '2026-07-25', week)).toBeNull();
  });
});

describe('assignLanes', () => {
  it('keeps a single bar in lane 0', () => {
    expect(assignLanes([{ startCol: 0, endCol: 2 }])).toEqual([
      { startCol: 0, endCol: 2, lane: 0 },
    ]);
  });

  it('stacks overlapping bars into distinct lanes', () => {
    const lanes = assignLanes([
      { startCol: 0, endCol: 3 },
      { startCol: 2, endCol: 5 },
    ]);
    expect(lanes.map((l) => l.lane)).toEqual([0, 1]);
  });

  it('reuses a lane once the earlier bar has ended', () => {
    // [0..1] and [3..4] don't overlap → both fit lane 0; the middle [1..3] overlaps
    // both, so it takes lane 1.
    const lanes = assignLanes([
      { startCol: 0, endCol: 1 },
      { startCol: 3, endCol: 4 },
      { startCol: 1, endCol: 3 },
    ]);
    // Returned sorted by start column: [0..1], [1..3], [3..4].
    expect(lanes).toEqual([
      { startCol: 0, endCol: 1, lane: 0 },
      { startCol: 1, endCol: 3, lane: 1 },
      { startCol: 3, endCol: 4, lane: 0 },
    ]);
  });

  it('carries the segment payload through', () => {
    const lanes = assignLanes([{ startCol: 0, endCol: 0, id: 'x' }]);
    expect(lanes[0].id).toBe('x');
  });
});

// ── A note written just after midnight belongs to today ──
describe('day grouping across the UTC boundary', () => {
  /** **The bug, in one assertion.** `created` is an ISO instant, so its first ten characters are
   *  the day *in UTC*, while `dayHeading` compares against the *local* day. East of Greenwich
   *  those disagree from midnight until the offset elapses — at UTC+8 a note written at 00:30
   *  was filed under "Yesterday" every night until 08:00. Found because a test happened to run
   *  past midnight, not because anyone was looking. */
  it('files an instant under its local day, not its UTC day', () => {
    // 00:30 local, whatever this machine's offset is.
    const justAfterMidnight = new Date();
    justAfterMidnight.setHours(0, 30, 0, 0);

    // What the timeline now does…
    const localKey = ymd(justAfterMidnight);
    // …versus what it used to do: slice the ISO string.
    const utcKey = justAfterMidnight.toISOString().slice(0, 10);

    expect(dayHeading(localKey)).toBe('Today');
    // The old key only matched when the machine is at or west of UTC. Asserting the *heading*
    // rather than the offset keeps this true in every timezone CI might run in.
    if (utcKey !== localKey) {
      expect(dayHeading(utcKey)).not.toBe('Today');
    }
  });
});
