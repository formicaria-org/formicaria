import { describe, expect, it } from 'vitest';
import { dayOf, formatStamp, parseStamp, relativeTime, timeRange, toStamp } from './stamp';

describe('relativeTime', () => {
  const now = Date.parse('2026-07-18T12:00:00Z');
  it('reads recent edits in coarse, human units', () => {
    expect(relativeTime('2026-07-18T11:59:50Z', now)).toBe('just now');
    expect(relativeTime('2026-07-18T11:55:00Z', now)).toBe('5m ago');
    expect(relativeTime('2026-07-18T09:00:00Z', now)).toBe('3h ago');
    expect(relativeTime('2026-07-16T12:00:00Z', now)).toBe('2d ago');
    expect(relativeTime('2026-07-04T12:00:00Z', now)).toBe('2w ago');
  });
  it('falls back to a date past ~a year, and is empty for garbage', () => {
    expect(relativeTime('2025-01-01T12:00:00Z', now)).toMatch(/\d/); // a formatted date
    expect(relativeTime('not-a-date', now)).toBe('');
  });
});

// The UI mirror of Rust's `fm_model::Stamp`. The Rust side has its own
// round-trip suite; these pin the JS half, especially the boundary that keeps an
// offset-aware `created` instant out of the naive-stamp path.
describe('parseStamp', () => {
  it('splits an all-day stamp', () => {
    expect(parseStamp('2026-07-20')).toEqual({ day: '2026-07-20', time: null });
  });

  it('splits a timed stamp', () => {
    expect(parseStamp('2026-07-20T14:30')).toEqual({ day: '2026-07-20', time: '14:30' });
  });

  it.each([null, undefined, '', 'not-a-date', '20/07/2026', '2026-07'])(
    'returns null for %s',
    (raw) => {
      expect(parseStamp(raw)).toBeNull();
    },
  );

  it('refuses an offset-aware RFC3339 instant rather than matching its prefix', () => {
    // `created` is an instant whose LOCAL day may differ from the UTC day in the
    // string. If this matched, isoDate would hand back the UTC day and Timeline
    // would file notes under the wrong day for anyone east of UTC.
    expect(parseStamp('2026-07-14T09:00:00Z')).toBeNull();
    expect(parseStamp('2026-07-14T09:00:00+08:00')).toBeNull();
  });
});

describe('dayOf / toStamp', () => {
  it('takes the day off either form', () => {
    expect(dayOf('2026-07-20')).toBe('2026-07-20');
    expect(dayOf('2026-07-20T14:30')).toBe('2026-07-20');
    expect(dayOf(null)).toBe('');
  });

  it('recombines a date and an optional time', () => {
    expect(toStamp('2026-07-20', '14:30')).toBe('2026-07-20T14:30');
    expect(toStamp('2026-07-20', '')).toBe('2026-07-20');
    expect(toStamp('2026-07-20', null)).toBe('2026-07-20');
  });

  it('drops the seconds a native time input can emit', () => {
    // <input type="time"> yields HH:MM:SS when its step allows seconds; the wire
    // format is minute-granular, so trim rather than let the backend reject it.
    expect(toStamp('2026-07-20', '14:30:00')).toBe('2026-07-20T14:30');
  });

  it('clears the property when the date is gone, time or not', () => {
    // An empty string IS the clear gesture; a time with no day is on no calendar.
    expect(toStamp('', '14:30')).toBe('');
    expect(toStamp('', '')).toBe('');
  });

  it('round-trips every composed value back through parseStamp', () => {
    for (const [day, time] of [
      ['2026-07-20', ''],
      ['2026-07-20', '14:30'],
      ['2026-01-01', '00:00'],
      ['2026-12-31', '23:59'],
    ] as const) {
      const wire = toStamp(day, time);
      expect(parseStamp(wire)).toEqual({ day, time: time || null });
    }
  });
});

describe('formatStamp', () => {
  it('renders an all-day stamp without a phantom time', () => {
    expect(formatStamp('2026-07-20')).toBe('20 Jul');
  });

  it('renders a timed stamp with its clock', () => {
    expect(formatStamp('2026-07-20T14:30')).toBe('20 Jul 14:30');
  });

  it('echoes a value it cannot read instead of showing NaN', () => {
    expect(formatStamp('whenever')).toBe('whenever');
    expect(formatStamp(null)).toBe('');
  });
});

describe('timeRange', () => {
  it('reads a meeting as its window when both ends are timed', () => {
    expect(timeRange('2026-07-20T14:30', '2026-07-20T15:00')).toBe('14:30–15:00');
  });

  it('falls back to whichever end has a time', () => {
    expect(timeRange('2026-07-20', '2026-07-20T15:00')).toBe('15:00');
    expect(timeRange('2026-07-20T14:30', '2026-07-20')).toBe('14:30');
  });

  it('is empty for an all-day span', () => {
    expect(timeRange('2026-07-18', '2026-07-20')).toBe('');
    expect(timeRange(null, null)).toBe('');
  });
});
