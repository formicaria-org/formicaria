// **The app's clock** (`decisions.md#data`, 2026-10-24): days and times are the zone the server
// reports, never the browser's. `pixi run test-ui` runs on `America/St_Johns` (−03:30), so a function
// that slipped back to the browser's zone answers differently here and fails.
import { afterEach, describe, expect, it } from 'vitest';
import { appZone, clockText, dayOfInstant, formatDay, setAppClock, setAppZone } from './appTime';
import { addDays, daysFrom, mondayOf, weekday } from './civil';

afterEach(() => setAppZone(null));

describe('the app clock', () => {
  it('puts a moment on its day and hour on a fixed offset that is not a whole hour (F21)', () => {
    setAppZone('+05:30');
    const late = Date.UTC(2026, 9, 23, 20, 0); // 01:30 on the 24th at +05:30
    expect(dayOfInstant(late)).toBe('2026-10-24');
    expect(clockText(late)).toBe('01:30');
    setAppZone('-03:30');
    expect(dayOfInstant(late)).toBe('2026-10-23');
    expect(clockText(late)).toBe('16:30');
  });

  it('reads a named zone through its own daylight saving', () => {
    setAppZone('Europe/Berlin');
    expect(clockText('2026-07-01T10:00:00Z')).toBe('12:00'); // summer, +02:00
    expect(clockText('2026-12-01T10:00:00Z')).toBe('11:00'); // winter, +01:00
  });

  it("takes the server's zone and says when it changed", () => {
    expect(setAppClock({ zone: 'Asia/Kathmandu' })).toBe(true);
    expect(setAppClock({ zone: 'Asia/Kathmandu' })).toBe(false);
    expect(appZone()).toBe('Asia/Kathmandu');
    expect(formatDay('2026-10-23T22:30:00Z')).toBe('24 Oct 2026'); // 04:15 there on the 24th
  });

  it('falls back to the browser only until the server answers', () => {
    setAppZone(null);
    expect(appZone()).toBe(Intl.DateTimeFormat().resolvedOptions().timeZone);
  });
});

describe('civil days', () => {
  it('counts days with no zone at all', () => {
    expect(addDays('2026-02-28', 1)).toBe('2026-03-01');
    expect(addDays('2026-01-01', -1)).toBe('2025-12-31');
    expect(daysFrom('2026-10-24', '2026-11-02')).toBe(9);
    expect(weekday('2026-10-24')).toBe(6); // a Saturday
    expect(mondayOf('2026-10-25')).toBe('2026-10-19');
  });
});
