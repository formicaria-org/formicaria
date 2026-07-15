import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { relativeDue, urgency } from './urgency';

// Urgency is derived from `due` against the local midnight of `new Date()`. Pin
// that reference so the classification is deterministic regardless of when the
// suite runs. 2026-07-14 (noon, local) is the app's working date; every case
// below is written relative to it.
beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-07-14T12:00:00'));
});
afterEach(() => {
  vi.useRealTimers();
});

describe('urgency', () => {
  it('is none for a missing or unparseable due date', () => {
    expect(urgency(null)).toBe('none');
    expect(urgency('not-a-date')).toBe('none');
  });

  const cases: Array<[string, string]> = [
    ['2026-07-10', 'overdue'], // 4 days ago
    ['2026-07-14', 'soon'], //    today (0 days)
    ['2026-07-16', 'soon'], //    +2 days (boundary)
    ['2026-07-17', 'week'], //    +3 days
    ['2026-07-21', 'week'], //    +7 days (boundary)
    ['2026-07-25', 'later'], //   +11 days
  ];
  it.each(cases)('classifies %s as %s', (due, expected) => {
    expect(urgency(due)).toBe(expected);
  });

  // Regression: `daysUntil` used to build `${due}T00:00:00` by concatenation, so
  // a timed due produced `2026-07-16T14:30T00:00:00` -> NaN -> 'none'. The note
  // then dropped out of every agenda band silently. Bands are day-granular, so a
  // timed stamp must classify exactly like its bare day.
  const timed: Array<[string, string]> = [
    ['2026-07-10T09:00', 'overdue'],
    ['2026-07-14T23:59', 'soon'],
    ['2026-07-16T14:30', 'soon'],
    ['2026-07-21T08:00', 'week'],
    ['2026-07-25T18:00', 'later'],
  ];
  it.each(timed)('classifies the timed %s as %s, same as its bare day', (due, expected) => {
    expect(urgency(due)).toBe(expected);
    expect(urgency(due)).toBe(urgency(due.slice(0, 10)));
  });

  it('never lets a time move an item across a band boundary', () => {
    // A 00:00 and a 23:59 item on the same day belong to the same band — the
    // clock is presentation, urgency is about which DAY needs attention.
    expect(urgency('2026-07-17T00:00')).toBe(urgency('2026-07-17T23:59'));
  });
});

describe('relativeDue', () => {
  const cases: Array<[string | null, string]> = [
    [null, ''],
    ['2026-07-14', 'today'],
    ['2026-07-15', 'tomorrow'],
    ['2026-07-13', 'yesterday'],
    ['2026-07-24', 'in 10 days'],
    ['2026-07-04', '10 days ago'],
    ['not-a-date', 'not-a-date'], // unparseable → echo the raw value back
  ];
  it.each(cases)('renders %s as "%s"', (due, expected) => {
    expect(relativeDue(due)).toBe(expected);
  });
});
