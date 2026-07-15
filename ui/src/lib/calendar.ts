// Pure calendar helpers, unit-testable like urgency.ts. Days are treated as
// local calendar dates (a bare YYYY-MM-DD), matching how `due` is interpreted
// everywhere else — no timezone math beyond the viewer's local calendar.

export interface Day {
  date: string; // YYYY-MM-DD
  inMonth: boolean; // false for leading/trailing days from adjacent months
}

export const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

function iso(y: number, mZeroBased: number, d: number): string {
  const mm = String(mZeroBased + 1).padStart(2, '0');
  const dd = String(d).padStart(2, '0');
  return `${y}-${mm}-${dd}`;
}

/** Weeks (Monday-first) covering `month` (0-indexed), padded with adjacent-month
 *  days so every row has exactly 7 cells. */
export function monthGrid(year: number, month: number): Day[][] {
  const first = new Date(year, month, 1);
  const lead = (first.getDay() + 6) % 7; // JS getDay: 0=Sun; shift so Monday=0
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const cells = Math.ceil((lead + daysInMonth) / 7) * 7;
  const weeks: Day[][] = [];
  for (let i = 0; i < cells; i++) {
    // Date normalizes negative/overflow day-of-month across month and year ends.
    const d = new Date(year, month, 1 - lead + i);
    if (i % 7 === 0) weeks.push([]);
    weeks[weeks.length - 1].push({
      date: iso(d.getFullYear(), d.getMonth(), d.getDate()),
      inMonth: d.getMonth() === month && d.getFullYear() === year,
    });
  }
  return weeks;
}

/** The local calendar date of `d` as YYYY-MM-DD (today's cell key/highlight). */
export function ymd(d: Date): string {
  return iso(d.getFullYear(), d.getMonth(), d.getDate());
}

/** A human month heading, e.g. "July 2026". */
export function monthLabel(year: number, month: number): string {
  return new Date(year, month, 1).toLocaleDateString(undefined, {
    month: 'long',
    year: 'numeric',
  });
}

/** The day-of-month number from a YYYY-MM-DD string. */
export function dayOfMonth(dateIso: string): number {
  return Number(dateIso.slice(8, 10));
}

/** Shift a YYYY-MM-DD date by `n` days (n may be negative); normalizes across
 *  month and year ends. */
export function addDays(dateIso: string, n: number): string {
  const [y, m, d] = dateIso.split('-').map(Number);
  return ymd(new Date(y, m - 1, d + n));
}

/** The Monday–Sunday week (7 cells) containing `dateIso`. `inMonth` is always
 *  true here — the week view has no adjacent-month dimming. */
export function weekOf(dateIso: string): Day[] {
  const [y, m, d] = dateIso.split('-').map(Number);
  const lead = (new Date(y, m - 1, d).getDay() + 6) % 7; // Monday = 0
  const week: Day[] = [];
  for (let i = 0; i < 7; i++) {
    const c = new Date(y, m - 1, d - lead + i);
    week.push({ date: ymd(c), inMonth: true });
  }
  return week;
}

/** A compact week heading, e.g. "14 Jul – 20 Jul 2026". */
export function weekLabel(days: Day[]): string {
  if (days.length === 0) return '';
  const first = new Date(`${days[0].date}T00:00:00`);
  const last = new Date(`${days[days.length - 1].date}T00:00:00`);
  const a = first.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
  const b = last.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
  return `${a} – ${b}`;
}

/** A journal day heading relative to today: "Today" / "Yesterday" / "Mon 14 Jul
 *  2026". Used to group the timeline. */
export function dayHeading(dateIso: string): string {
  const today = ymd(new Date());
  if (dateIso === today) return 'Today';
  if (dateIso === addDays(today, -1)) return 'Yesterday';
  return new Date(`${dateIso}T00:00:00`).toLocaleDateString(undefined, {
    weekday: 'short',
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}
