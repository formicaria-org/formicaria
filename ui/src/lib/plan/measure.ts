/// How a plan's numbers are shown and typed. Pure, so it has tests.
///
/// A bound is stored in the unit of what it measures: a value's own unit for a day's total
/// (`fiber_mg`), and otherwise percent, minutes, days or sets. A person types grams where the
/// store keeps milligrams, as on the targets form (`units.ts` says how).
import { showWith, typed } from '../units';
import type { PlanCheck, PlanCheckProgress } from '../types';

type Unit = { unit: string; per: number };

/** The unit a check's bound is typed in, and how many stored units one of it is. */
export function unitOf(check: Pick<PlanCheck, 'kind' | 'key'>): Unit {
  switch (check.kind) {
    case 'sum':
      return check.key ? typed(check.key) : { unit: '', per: 1 };
    case 'share':
      return { unit: '%', per: 1 };
    case 'minutes':
      return { unit: 'min', per: 1 };
    case 'sets':
      return { unit: 'sets', per: 1 };
    default:
      return { unit: 'days', per: 1 };
  }
}

/** A stored bound as the text in its field. */
export function shownBound(check: Pick<PlanCheck, 'kind' | 'key'>, v: number | undefined): string {
  if (v === undefined) return '';
  return String(Math.round((v / unitOf(check).per) * 10) / 10);
}

/** What was typed, as a stored bound: a whole number, or nothing when the field is empty or is
 *  not a number above zero. A bound of nothing is "do not measure this". */
export function storedBound(
  check: Pick<PlanCheck, 'kind' | 'key'>,
  text: string | number | null | undefined,
): number | undefined {
  if (text === '' || text === null || text === undefined) return undefined;
  const n = Number(text);
  if (!Number.isFinite(n) || n <= 0) return undefined;
  return Math.round(n * unitOf(check).per);
}

/** A measured value in words: `23 g`, `31%`, `90 min`, `2 days`. */
export function shownValue(c: Pick<PlanCheckProgress, 'unit'>, v: number): string {
  switch (c.unit) {
    case 'percent':
      return `${v}%`;
    case 'minutes':
      return `${v} min`;
    case 'days':
      return v === 1 ? '1 day' : `${v} days`;
    case 'sets':
      return v === 1 ? '1 set' : `${v} sets`;
    default:
      return showWith(c.unit, v);
  }
}

/** What a check asks for, in words: `at least 25 g`, `at most 30%`, `45–75%`. */
export function shownAsk(c: Pick<PlanCheckProgress, 'unit' | 'min' | 'max'>): string {
  if (c.min !== undefined && c.max !== undefined) {
    // One unit for the pair: `45–75%`, `20–35 g`.
    const hi = shownValue(c, c.max);
    const lo = shownValue(c, c.min).replace(/\s*[^\d.,]+$/, '');
    return `${lo}–${hi}`;
  }
  if (c.min !== undefined) return `at least ${shownValue(c, c.min)}`;
  if (c.max !== undefined) return `at most ${shownValue(c, c.max)}`;
  return '';
}

/** How far along a check is, from 0 to 1, for the bar under it. Towards the least it asks for
 *  when it asks for a least; towards the most it allows otherwise. */
export function fraction(c: Pick<PlanCheckProgress, 'min' | 'max' | 'value'>): number {
  if (c.value === undefined) return 0;
  const goal = c.min ?? c.max;
  if (!goal) return 0;
  return Math.max(0, Math.min(1, c.value / goal));
}

/** What a check's state means in a word a person would use. A week still under way is "so far",
 *  never "under": two of three days by Wednesday is not a failure. */
export function verdict(c: Pick<PlanCheckProgress, 'state' | 'per'>, weekOver: boolean): string {
  switch (c.state) {
    case 'met':
      return 'on track';
    case 'over':
      return 'over';
    case 'under':
      return c.per === 'week' && !weekOver ? 'so far' : 'under';
    default:
      return c.per === 'day' ? 'nothing logged' : 'nothing yet';
  }
}

/// The property keys a followed plan's note carries, from the bounds a person settled on. Each
/// bound under `p_<check>_min` / `p_<check>_max`; and, where a check is a day's total of a value
/// the day screen also shows, the same number under the value's own key, so the day screen
/// measures the day against it without knowing what a plan is.
export function boundProps(
  checks: PlanCheck[],
  bounds: Record<string, { min?: number; max?: number }>,
  dayKeys: string[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const c of checks) {
    const b = bounds[c.id];
    if (!b) continue;
    if (b.min !== undefined) out[`p_${c.id}_min`] = b.min;
    if (b.max !== undefined) out[`p_${c.id}_max`] = b.max;
    if (c.kind === 'sum' && c.key && dayKeys.includes(c.key)) {
      // A ceiling is the figure to stay under; with none, the floor is the figure to reach.
      const target = b.max ?? b.min;
      if (target !== undefined) out[c.key] = target;
    }
  }
  return out;
}

/// What following an option writes into its note's body: what it is, what it holds the person
/// to, and where every number came from — so the note reads without the app.
export function planBody(
  plan: { name: string; summary: string; sources: { by: string; title: string; url: string }[] },
  lines: { label: string; ask: string; basis: string }[],
): string {
  const out = [`# ${plan.name}`, '', plan.summary, ''];
  if (lines.length) {
    out.push('## What it asks', '');
    for (const l of lines) out.push(`- **${l.label}**: ${l.ask}. ${l.basis}`);
    out.push('');
  }
  if (plan.sources.length) {
    out.push('## Sources', '');
    for (const s of plan.sources)
      out.push(s.url ? `- ${s.by}: [${s.title}](<${s.url}>)` : `- ${s.by}: ${s.title}`);
    out.push('');
  }
  return out.join('\n');
}
