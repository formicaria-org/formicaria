// Pure calendar helpers, unit-testable like urgency.ts. Days are treated as
// local calendar dates (a bare YYYY-MM-DD), matching how `due` is interpreted
// everywhere else — no timezone math beyond the viewer's local calendar. A
// `start`/`due` may carry a time; the grid is day-granular, so every value is
// narrowed with `dayOf` before it reaches the geometry below.
import { dayOf, parseStamp } from './stamp';

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

/** Normalize a timestamp to a local calendar date (YYYY-MM-DD). Three cases:
 *  a bare date and a naive `start`/`due` stamp (`2026-07-20T14:30`) already name
 *  their day, so their day is taken literally — never round-tripped through
 *  `Date`, which would risk a timezone shift on a value that has no timezone. An
 *  offset-aware RFC3339 instant (the `created` field, `...Z`) is genuinely a
 *  point in time, so it IS converted to the viewer's local day. */
export function isoDate(stamp: string): string {
  return dayOf(stamp) || ymd(new Date(stamp));
}

/** Where a bar sits within one Monday–Sunday week, in 0-based columns. `null`
 *  when the `[start, end]` day range (inclusive, `start <= end`) doesn't touch
 *  this week at all. `continuesLeft/Right` mark a bar that runs off the row into
 *  an adjacent week, so the renderer can flatten that end instead of rounding it.
 *  ISO date strings compare lexicographically, so no Date math is needed. */
export interface WeekSpan {
  startCol: number;
  endCol: number;
  continuesLeft: boolean;
  continuesRight: boolean;
}

export function clampRangeToWeek(start: string, end: string, week: Day[]): WeekSpan | null {
  if (week.length === 0) return null;
  const weekStart = week[0].date;
  const weekEnd = week[week.length - 1].date;
  if (end < weekStart || start > weekEnd) return null; // disjoint from this row
  const clampedStart = start < weekStart ? weekStart : start;
  const clampedEnd = end > weekEnd ? weekEnd : end;
  return {
    startCol: week.findIndex((d) => d.date === clampedStart),
    endCol: week.findIndex((d) => d.date === clampedEnd),
    continuesLeft: start < weekStart,
    continuesRight: end > weekEnd,
  };
}

/** Greedy lane packing: give each segment the lowest lane (row) in which it
 *  doesn't overlap an already-placed segment's columns, so overlapping bars in a
 *  week stack instead of colliding. Segments are returned sorted by start column
 *  (then end), each annotated with its `lane`. */
export function assignLanes<T extends { startCol: number; endCol: number }>(
  segments: T[],
): (T & { lane: number })[] {
  const laneEnds: number[] = []; // the last occupied endCol in each lane
  const sorted = [...segments].sort((a, b) => a.startCol - b.startCol || a.endCol - b.endCol);
  return sorted.map((seg) => {
    let lane = laneEnds.findIndex((end) => end < seg.startCol);
    if (lane === -1) {
      lane = laneEnds.length;
      laneEnds.push(seg.endCol);
    } else {
      laneEnds[lane] = seg.endCol;
    }
    return { ...seg, lane };
  });
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

// ---------------------------------------------------------------------------------------------
// The week's time grid: where a timed note sits inside its day
// ---------------------------------------------------------------------------------------------

/** How long a note with one clock time is drawn for. A deadline at 18:30 has no length of its own;
 *  half an hour is enough to hold a time and a title, and it is also the least any block is drawn,
 *  so a ten-minute meeting stays something a finger can hit. */
export const MARK_MINUTES = 30;
const DAY_MINUTES = 24 * 60;

/** Minutes since midnight for an `HH:MM` wall-clock time. */
export function minutesOf(time: string): number {
  const [h, m] = time.split(':').map(Number);
  return h * 60 + m;
}

interface Timed {
  start: string | null;
  due: string | null;
}

/** The day a note belongs to in the time grid, or null when it belongs in the all-day strip.
 *
 *  A note is placed by the clock when its `due` carries a time and it does not begin on an earlier
 *  day. One that spans days is a bar across them, which the strip already draws; one with a date
 *  and no time has no slot to sit in. */
export function timedDay(card: Timed): string | null {
  const due = parseStamp(card.due);
  if (!due?.time) return null;
  const start = parseStamp(card.start);
  if (start && start.day !== due.day) return null;
  return due.day;
}

/** A note placed in one day's column: `from`..`to` in minutes since midnight, and which of `cols`
 *  side-by-side columns it takes when others overlap it. */
export interface TimedBlock<T> {
  card: T;
  from: number;
  to: number;
  col: number;
  cols: number;
}

/** Every note placed by the clock on `day`, laid out so that overlapping ones share the width.
 *
 *  With a start and an end time the block runs between them. With only the due time it is a mark
 *  of [`MARK_MINUTES`] beginning at that time — a deadline is read downwards from its line, like
 *  every other entry. Notes that overlap, directly or through a chain of others, form a group; each
 *  takes the first free column, and the whole group is as wide as its busiest moment. */
export function timedBlocks<T extends Timed>(cards: T[], day: string): TimedBlock<T>[] {
  const placed = cards
    .filter((c) => timedDay(c) === day)
    .map((card) => {
      const end = minutesOf(parseStamp(card.due)!.time!);
      const startTime = parseStamp(card.start)?.time;
      const begin = startTime ? minutesOf(startTime) : null;
      let from = begin !== null && begin < end ? begin : end;
      let to = begin !== null && begin < end ? end : end + MARK_MINUTES;
      to = Math.max(to, from + MARK_MINUTES);
      // The last half hour of the day still has to fit on the day.
      if (to > DAY_MINUTES) {
        from = Math.max(0, from - (to - DAY_MINUTES));
        to = DAY_MINUTES;
      }
      return { card, from, to, col: 0, cols: 1 };
    })
    .sort((a, b) => a.from - b.from || b.to - a.to);

  let group: typeof placed = [];
  let groupEnd = -1;
  const close = () => {
    const cols = Math.max(0, ...group.map((b) => b.col)) + 1;
    for (const b of group) b.cols = cols;
    group = [];
  };
  for (const b of placed) {
    if (group.length && b.from >= groupEnd) close();
    const taken = new Set(group.filter((o) => o.to > b.from).map((o) => o.col));
    while (taken.has(b.col)) b.col++;
    group.push(b);
    groupEnd = Math.max(groupEnd, b.to);
  }
  if (group.length) close();
  return placed;
}
