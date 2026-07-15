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
