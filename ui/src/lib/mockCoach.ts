/// The Coach's commands for the in-memory backend used by `pnpm dev` and the tests.
///
/// **A stand-in, not a second implementation**: the real reading, counting and matching are
/// `coach.rs`, tested there (`crates/fm-app/tests/coach.rs`). This answers in the same shapes, on
/// the same words, enough for a screen to be built and tested against.
import { coachPlan } from './mockPlan';
import { addDays, mondayOf } from './civil';
import type {
  CoachAnswer,
  CoachFolded,
  CoachMatches,
  CoachWeekProgress,
  ObjectMeta,
} from './types';

const dayOf = (n: ObjectMeta) => (n.start ?? '').slice(0, 10);
const planned = (n: ObjectMeta) => !!n.props.planned;
const isLog = (n: ObjectMeta, kind: string) => n.props.log === kind;
const copy = <T>(x: T): T => JSON.parse(JSON.stringify(x)) as T;
const PLANNED_KEYS = ['minutes', 'distance_m', 'reps', 'load_g', 'effort'];

export function coach(
  notes: ObjectMeta[],
  a: { day: string; from: string; to: string },
): CoachAnswer {
  const followed = notes
    .filter(
      (n) =>
        n.props.program === 'coach' &&
        !n.props.log &&
        (!n.start || dayOf(n) <= a.day) &&
        (!n.due || n.due.slice(0, 10) >= a.day),
    )
    .sort((x, y) => (y.start ?? '').localeCompare(x.start ?? ''))
    .map((n) => ({
      note: copy(n),
      plan: n.props.plan === coachPlan.id ? coachPlan : undefined,
      week: Math.max(1, Number(n.props.week ?? 1) || 1),
      goal: String(n.props.goal ?? ''),
      ...(n.props.event_date ? { event_date: String(n.props.event_date) } : {}),
    }));
  const days = [];
  for (let d = a.from; d <= a.to; d = addDays(d, 1)) {
    const on = notes.filter((n) => dayOf(n) === d);
    days.push({
      day: d,
      training: on.filter((n) => isLog(n, 'workout')).map(copy),
      meals: on.filter((n) => isLog(n, 'food') && planned(n)).map(copy),
      meetings: on.filter((n) => !n.props.log && n.tags.includes('meeting')).map(copy),
    });
  }
  return { followed, days };
}

export function coachProgress(
  notes: ObjectMeta[],
  a: { day: string; weeks?: number },
): CoachWeekProgress[] {
  const weeks = a.weeks ?? 8;
  const first = addDays(mondayOf(a.day), -7 * (weeks - 1));
  return Array.from({ length: weeks }, (_, w) => {
    const from = addDays(first, 7 * w);
    const to = addDays(from, 6);
    const by = new Map<string, CoachWeekProgress['sports'][number]>();
    for (const n of notes) {
      const d = dayOf(n);
      if (!isLog(n, 'workout') || !n.props.activity || d < from || d > to) continue;
      const sport = String(n.props.sport ?? n.props.activity);
      const t = by.get(sport) ?? {
        sport,
        planned: 0,
        done: 0,
        skipped: 0,
        planned_minutes: 0,
        done_minutes: 0,
        distance_m: 0,
      };
      const now = planned(n);
      if (now || n.props.plan) {
        t.planned += 1;
        t.planned_minutes += Number((now ? n.props.minutes : n.props.planned_minutes) ?? 0);
      }
      if (now && n.props.skipped) t.skipped += 1;
      if (!now) {
        t.done += 1;
        t.done_minutes += Number(n.props.minutes ?? 0);
        t.distance_m += Number(n.props.distance_m ?? 0);
      }
      by.set(sport, t);
    }
    return { from, to, sports: [...by.values()] };
  });
}

const imported = (n: ObjectMeta) =>
  String(n.props.source_key ?? '').startsWith('fit:') &&
  !String(n.props.source_key).includes(':set:');

export function matchPlanned(notes: ObjectMeta[], a: { day?: string }): CoachMatches {
  const out: CoachMatches = { proposals: [], unmatched: [] };
  const open = notes.filter(
    (n) => isLog(n, 'workout') && planned(n) && n.props.activity && !n.props.skipped,
  );
  for (const n of notes.filter((x) => imported(x) && !x.props.plan && !planned(x))) {
    const d = String(n.props.day ?? dayOf(n)).slice(0, 10);
    if (a.day && d !== a.day) continue;
    const fits = open.filter(
      (p) =>
        dayOf(p) === d && (p.props.counts_as ?? 'aerobic') === (n.props.counts_as ?? 'aerobic'),
    );
    if (fits.length === 1) out.proposals.push({ imported: copy(n), planned: copy(fits[0]) });
    else if (fits.length > 1 || a.day)
      out.unmatched.push({ imported: copy(n), candidates: fits.map(copy) });
  }
  return out;
}

export function acceptMatch(
  notes: ObjectMeta[],
  a: { planned: string; imported: string },
  bodies: Map<string, string>,
): CoachFolded {
  const p = notes.find((n) => n.id === a.planned);
  const i = notes.findIndex((n) => n.id === a.imported);
  if (!p || !planned(p) || i < 0) throw new Error('that entry is not a planned session any more');
  const imp = notes[i];
  const previous: Record<string, string | null> = {};
  const was = (k: string) =>
    k in previous ? null : (previous[k] = p.props[k] == null ? null : String(p.props[k]));
  for (const k of PLANNED_KEYS) {
    if (typeof p.props[k] === 'number' && p.props[`planned_${k}`] == null) {
      was(`planned_${k}`);
      p.props[`planned_${k}`] = p.props[k];
    }
  }
  for (const [k, v] of Object.entries(imp.props)) {
    if (['planned', 'plan', 'plan_week', 'session'].includes(k) || k.startsWith('planned_'))
      continue;
    was(k);
    p.props[k] = v;
  }
  was('planned');
  delete p.props.planned;
  const bodyBefore = bodies.get(p.id) ?? '';
  const theirs = bodies.get(imp.id) ?? '';
  if (theirs.trim())
    bodies.set(p.id, bodyBefore.trim() ? `${bodyBefore.trimEnd()}\n\n${theirs}` : theirs);
  notes.splice(i, 1);
  return {
    planned: p.id,
    vault: p.vault,
    previous,
    body_before: bodyBefore,
    removed: { id: imp.id, vault: imp.vault, file: JSON.stringify(imp) },
    recorded: false,
  };
}
