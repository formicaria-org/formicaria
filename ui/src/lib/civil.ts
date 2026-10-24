// **Days as a calendar has them** (`decisions.md#data`, 2026-10-24, *one app clock*): `YYYY-MM-DD`,
// with no zone at all. Every computation here is in UTC on purpose, so the same day gives the same
// answer on any device, in any zone, on the day a clock changes. What "today" is, and which day a
// moment falls on, are the app clock's questions (`appTime.ts`); this only counts days.

const DAY_MS = 86_400_000;

function utcOf(day: string): number {
  const [y, m, d] = day.slice(0, 10).split('-').map(Number);
  return Date.UTC(y, m - 1, d);
}

function dayOfUtc(ms: number): string {
  const t = new Date(ms);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${t.getUTCFullYear()}-${p(t.getUTCMonth() + 1)}-${p(t.getUTCDate())}`;
}

/** `YYYY-MM-DD` from a year, a 0-based month and a day — normalised across month and year ends. */
export function civilDay(year: number, month0: number, day: number): string {
  return dayOfUtc(Date.UTC(year, month0, day));
}

/** `day` moved by `n` days (n may be negative). */
export function addDays(day: string, n: number): string {
  return dayOfUtc(utcOf(day) + n * DAY_MS);
}

/** 0 = Sunday … 6 = Saturday, as `Date#getDay` counts. */
export function weekday(day: string): number {
  return new Date(utcOf(day)).getUTCDay();
}

/** The Monday of `day`'s week. */
export function mondayOf(day: string): string {
  return addDays(day, -((weekday(day) + 6) % 7));
}

/** Whole days from `a` to `b` (negative when `b` is earlier). */
export function daysFrom(a: string, b: string): number {
  return Math.round((utcOf(b) - utcOf(a)) / DAY_MS);
}

/** How many days `month0` (0-based) of `year` has. */
export function daysInMonth(year: number, month0: number): number {
  return new Date(Date.UTC(year, month0 + 1, 0)).getUTCDate();
}

/** `day` worded by the reader's locale (`{ weekday: 'short', day: 'numeric' }` …), never moved by a
 *  zone: it is formatted at noon UTC with the UTC zone. */
export function formatCivil(day: string, opts: Intl.DateTimeFormatOptions): string {
  return new Intl.DateTimeFormat(undefined, { ...opts, timeZone: 'UTC' }).format(
    new Date(utcOf(day) + DAY_MS / 2),
  );
}
