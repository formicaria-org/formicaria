// The Coach's arithmetic on days and drafts, kept out of the screens so it can be tested
// (`decisions.md` 2026-11-20, *Fitness gets a Coach tab; the app still chooses no loads*).
//
// **Nothing here chooses a number.** A row starts from the plan's template: a published plan's own
// figure where it has one (`numbers_from: 'source'`), and otherwise blank for the person to fill.
import { addDays, mondayOf } from '../civil';
import type { CoachSession, CoachWeek, NoteDraft, ObjectMeta } from '../types';

/** The figures a done entry keeps as `planned_<key>` — the same list as `coach.rs`'s. */
export const PLANNED_KEYS = ['minutes', 'distance_m', 'reps', 'load_g', 'effort'] as const;

/** **What was planned, kept** (Spec K.3): for an entry linked to a plan, its planned figures as
 *  `planned_<key>`, to be written before the done ones. Empty for an entry with no `plan` link, so
 *  ticking an ordinary planned set or meal is unchanged. A figure already kept is not overwritten. */
export function plannedCopies(row: ObjectMeta): Record<string, number> {
  if (!row.props.plan) return {};
  const out: Record<string, number> = {};
  for (const k of PLANNED_KEYS) {
    const v = row.props[k];
    if (typeof v === 'number' && row.props[`planned_${k}`] === undefined) out[`planned_${k}`] = v;
  }
  return out;
}

/** fm's sports, as an import writes them (`shipped/fit-sports.json`): the activity name a planned
 *  entry carries, and what it counts as. */
export const SPORTS: { value: string; label: string; activity: string; counts_as: string }[] = [
  { value: 'run', label: 'Run', activity: 'Running', counts_as: 'aerobic' },
  { value: 'ride', label: 'Ride', activity: 'Cycling', counts_as: 'aerobic' },
  { value: 'swim', label: 'Swim', activity: 'Swimming', counts_as: 'aerobic' },
  { value: 'walk', label: 'Walk', activity: 'Walking', counts_as: 'aerobic' },
  { value: 'row', label: 'Row', activity: 'Rowing', counts_as: 'aerobic' },
  { value: 'strength', label: 'Strength', activity: 'Strength training', counts_as: 'strength' },
];
export const sportLabel = (s: unknown) =>
  SPORTS.find((x) => x.value === s)?.label ?? (typeof s === 'string' && s ? s : 'Other');

/** A session's type, in words. */
const TYPES: Record<string, string> = {
  easy: 'Easy',
  long: 'Long',
  intervals: 'Intervals',
  race: 'Event',
  'walk-run': 'Walk and run',
  strength: 'Strength',
  'strength-then-endurance': 'Strength, then endurance',
};
export const typeLabel = (t: unknown) =>
  TYPES[String(t ?? '')] ?? (typeof t === 'string' && t ? t : 'Session');

/** The words a step's part is shown with. */
export const PARTS: Record<string, string> = {
  warmup: 'Warm-up',
  work: 'Work',
  recover: 'Recovery',
  cooldown: 'Cool-down',
};

const HINTS = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'];
/** The day of `monday`'s week a template is meant for. */
export const dayFor = (monday: string, hint: string) =>
  addDays(monday, Math.max(0, HINTS.indexOf(hint)));
/** The seven days of the week holding `day`, Monday first. */
export const weekOf = (day: string) => {
  const monday = mondayOf(day);
  return Array.from({ length: 7 }, (_, i) => addDays(monday, i));
};

/** One session of a week, as the person edits it before it goes on the calendar. Text, so an empty
 *  field stays empty. */
export interface WeekRow {
  session: string;
  day: string;
  sport: string;
  type: string;
  said: string;
  minutes: string;
  km: string;
  target: string;
  steps: string;
  /** Whose the figures it starts with are. */
  numbers_from: CoachSession['numbers_from'];
}

const stepLines = (s: CoachSession) =>
  s.steps.map((t) => `${PARTS[t.part] ?? t.part}: ${t.said}`).join('\n');

/** A week's rows from its template: **pre-filled only where the source gives a figure**. */
export function rowsFor(week: CoachWeek, monday: string): WeekRow[] {
  return week.sessions.map((s) => {
    const theirs = s.numbers_from === 'source';
    return {
      session: s.id,
      day: dayFor(monday, s.day_hint),
      sport: s.sport,
      type: s.type,
      said: s.said,
      minutes: theirs && s.minutes !== undefined ? String(s.minutes) : '',
      km: theirs && s.metres !== undefined ? String(s.metres / 1000) : '',
      target: '',
      steps: stepLines(s),
      numbers_from: s.numbers_from,
    };
  });
}

/** A number from what was typed, or nothing. A number field hands back a number (or `null` when
 *  emptied), a text field a string: both are read the same. */
const num = (v: string | number | null | undefined) => {
  const s = String(v ?? '').trim();
  const n = Number(s.replace(',', '.'));
  return s && Number.isFinite(n) && n > 0 ? n : null;
};

/** The planned entries "Put this week on my calendar" writes: the training log's own shape
 *  (`log: workout`, `planned: 1`, `start`) and the three links back to the plan. */
export function draftsFor(rows: WeekRow[], program: string, week: number): NoteDraft[] {
  return rows.map((r) => {
    const sport = SPORTS.find((x) => x.value === r.sport);
    const minutes = num(r.minutes);
    const km = num(r.km);
    const props: Record<string, string | number> = {
      log: 'workout',
      planned: 1,
      activity: sport?.activity ?? (r.sport || 'Other'),
      sport: r.sport,
      counts_as: sport?.counts_as ?? 'aerobic',
      type: r.type,
      plan: program,
      plan_week: week,
      session: r.session,
    };
    if (minutes !== null) props.minutes = Math.round(minutes);
    if (km !== null) props.distance_m = Math.round(km * 1000);
    const target = String(r.target ?? '').trim();
    if (target) props.target = target;
    const title =
      r.sport === 'strength' ||
      typeLabel(r.type).toLowerCase().includes(sportLabel(r.sport).toLowerCase())
        ? typeLabel(r.type)
        : `${typeLabel(r.type)} · ${sportLabel(r.sport)}`;
    const body = String(r.steps ?? '')
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => `- ${l}`)
      .join('\n');
    return { title, start: r.day, body: body ? `${body}\n` : '', props };
  });
}

/** Where an entry stands: still meant, skipped, or done. */
export function stateOf(e: ObjectMeta): 'planned' | 'skipped' | 'done' {
  if (!e.props.planned) return 'done';
  return e.props.skipped ? 'skipped' : 'planned';
}

/** An entry's figures in a line: minutes and distance, done or meant. */
export function figures(e: ObjectMeta): string {
  const out: string[] = [];
  const m = e.props.minutes;
  const d = e.props.distance_m;
  if (typeof m === 'number') out.push(`${m} min`);
  if (typeof d === 'number') out.push(`${Math.round(d / 100) / 10} km`);
  const pm = e.props.planned_minutes;
  if (!e.props.planned && typeof pm === 'number' && pm !== m) out.push(`planned ${pm} min`);
  if (typeof e.props.target === 'string' && e.props.target) out.push(e.props.target);
  return out.join(' · ');
}

/** A moved entry keeps its time of day, if it had one. */
export const movedStart = (start: string | null, day: string) =>
  start && start.length > 10 ? `${day}${start.slice(10)}` : day;
