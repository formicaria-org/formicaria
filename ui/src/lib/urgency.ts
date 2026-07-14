// Urgency is DERIVED from the due date, never stored — there is no priority
// field. overdue > soon > this-week > later. Computed against today in the
// viewer's local time, so nudging `due` forward is the whole reprioritization
// gesture.
export type Urgency = 'overdue' | 'soon' | 'week' | 'later' | 'none';

const DAY = 86_400_000;

function daysUntil(due: string): number | null {
  const d = new Date(`${due}T00:00:00`);
  if (Number.isNaN(d.getTime())) return null;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  return Math.round((d.getTime() - today.getTime()) / DAY);
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

export function relativeDue(due: string | null): string {
  if (!due) return '';
  const days = daysUntil(due);
  if (days === null) return due;
  if (days === 0) return 'today';
  if (days === 1) return 'tomorrow';
  if (days === -1) return 'yesterday';
  return days < 0 ? `${-days} days ago` : `in ${days} days`;
}
