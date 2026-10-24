// Urgency is DERIVED from the due date, never stored — there is no priority
// field. overdue > soon > this-week > later. Computed against today in the
// viewer's local time, so nudging `due` forward is the whole reprioritization
// gesture.
//
// Bands are DAY-granular even though `due` may now carry a time: "due soon"
// answers which days need attention, and a 14:00 meeting shouldn't jump bands at
// 14:01. The time is presentation (see `formatStamp`), not urgency.
import { appToday } from './appTime';
import { daysFrom } from './civil';
import { dayOf } from './stamp';

export type Urgency = 'overdue' | 'soon' | 'week' | 'later' | 'none';

function daysUntil(due: string): number | null {
  // Take the day off the stamp before building a Date. The old code appended
  // `T00:00:00` to the raw value, which for a timed `due` produced the nonsense
  // `2026-07-20T14:30T00:00:00` -> NaN -> urgency 'none' -> the note vanished
  // from every agenda band with no error anywhere.
  const day = dayOf(due);
  if (!day) return null;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) return null;
  // Both are days, counted on the calendar: today is the app clock's (2026-10-24).
  return daysFrom(appToday(), day);
}

export function urgency(due: string | null): Urgency {
  if (!due) return 'none';
  const days = daysUntil(due);
  if (days === null) return 'none';
  if (days < 0) return 'overdue';
  if (days <= 2) return 'soon';
  if (days <= 7) return 'week';
  return 'later';
}

// Ordered urgency buckets and their section labels. Kept here — not inline in
// the agenda renderer — so the renderer stays free of any human-readable
// scheduling literal, mirroring the status-literal rule.
export const URGENCY_ORDER: Urgency[] = ['overdue', 'soon', 'week', 'later', 'none'];

const URGENCY_LABELS: Record<Urgency, string> = {
  overdue: 'Overdue',
  soon: 'Due soon',
  week: 'This week',
  later: 'Later',
  none: 'Someday',
};

export function urgencyLabel(u: Urgency): string {
  return URGENCY_LABELS[u];
}

export function relativeDue(due: string | null): string {
  if (!due) return '';
  const days = daysUntil(due);
  if (days === null) return due;
  if (days === 0) return 'today';
  if (days === 1) return 'tomorrow';
  if (days === -1) return 'yesterday';
  return days < 0 ? `${-days} days ago` : `in ${days} days`;
}
