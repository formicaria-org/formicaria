/// The plan commands, for the in-memory backend used by `pnpm dev` and the tests.
///
/// **A stand-in, not a second implementation.** The real library, the formulas and the measuring
/// are `plan.rs`, tested there. This carries three options a group, works out no formula, and
/// measures only what a screen test needs: a day's total, and the days a person said they kept.
import { addDays } from './units';
import type {
  ObjectMeta,
  PlanBound,
  PlanCheckProgress,
  PlanLibrary,
  PlanOption,
  PlanPreview,
  PlanProfile,
  PlanProgress,
  PlanCheck,
  PlanFlag,
  PlanFollowed,
  PlanWatch,
} from './types';

const FLAGS: PlanFlag[] = [
  'pregnant',
  'breastfeeding',
  'postmenopausal',
  'kidney',
  'diabetes_meds',
  'eating_disorder',
];
/// The order the real core shows followed cards in (`plan.rs` `QUESTIONS`).
const QUESTIONS = [
  'healthy',
  'nutrients',
  'amount',
  'timing',
  'moving',
  'steps',
  'stronger',
  'fitter',
  'falls',
  'flexible',
  'vegan',
  'pregnancy',
  'own',
];

const who: PlanOption = {
  id: 'who-healthy-diet',
  group: 'food',
  kind: 'targets',
  question: 'healthy',
  start_here: true,
  relation: 'Written for all adults in all countries.',
  name: 'WHO healthy diet',
  rests_on: 'guideline',
  summary: 'Limits on fat, saturated fat and sodium; at least 25 g of fibre a day.',
  sources: [{ by: 'World Health Organization', title: 'Healthy diet, fact sheet', url: '' }],
  needs: [],
  checks: [
    {
      id: 'fibre',
      label: 'Fibre',
      kind: 'sum',
      key: 'fiber_mg',
      min: 25_000,
      basis: 'WHO: at least 25 g a day',
    },
    {
      id: 'fat',
      label: 'Fat, share of energy',
      kind: 'share',
      key: 'fat_mg',
      max: 30,
      basis: 'WHO: 30% or less',
    },
  ],
  unmeasured: ['Free sugars: the food tables give total sugars.'],
  examples: [
    {
      kind: 'selected',
      by: 'formicaria, by arithmetic from the recipes it carries',
      title: 'NHLBI Healthy Eating Recipes',
      url: 'https://www.nhlbi.nih.gov/',
      note: 'Chosen by arithmetic from the carried recipes; only an indication to start from.',
      recipes: [{ meal: 'Lunch', recipe: 'nhlbi-vegetable-stew', servings: 1 }],
      totals: { kcal: 119, fiber_g: 4 },
      against: 'Fibre 4 g: short by 21 g of 25 g. About a twentieth of a day’s energy.',
    },
  ],
  guides: [],
};
const energy: PlanOption = {
  id: 'energy-for-me',
  group: 'food',
  kind: 'targets',
  question: 'amount',
  relation: 'An estimate of what you need.',
  name: 'Energy and protein for me',
  rests_on: 'guideline',
  summary: 'A daily energy estimate from your measurements.',
  sources: [{ by: 'Henry CJK', title: 'Basal metabolic rate studies in humans', url: '' }],
  needs: ['sex', 'born', 'height_cm', 'weight_g', 'pal_x10'],
  checks: [{ id: 'energy', label: 'Energy', kind: 'sum', key: 'kcal', basis: 'An estimate' }],
  unmeasured: [],
  guides: [],
};
const tre: PlanOption = {
  id: 'time-restricted-eating',
  group: 'food',
  kind: 'timing',
  question: 'timing',
  not_for: ['pregnant', 'breastfeeding', 'eating_disorder'],
  relation: 'One of three ways of eating less.',
  name: 'Time-restricted eating',
  rests_on: 'definition',
  summary: 'All eating inside a part of the day.',
  sources: [
    { by: 'Koppold et al.', title: 'International consensus on fasting terminology', url: '' },
  ],
  needs: [],
  checks: [
    {
      id: 'kept',
      label: 'Days you kept to it this week',
      kind: 'kept',
      min: 7,
      basis: 'You say so each day',
    },
  ],
  unmeasured: ['The hours themselves.'],
  caution: 'This is a definition, not advice.',
  guides: [],
};
const ownFood: PlanOption = {
  id: 'own-food',
  group: 'food',
  kind: 'own',
  question: 'own',
  name: 'My own',
  rests_on: 'own',
  summary: 'Set each number yourself.',
  sources: [],
  needs: [],
  checks: [
    { id: 'energy', label: 'Energy', kind: 'sum', key: 'kcal', basis: 'Yours' },
    { id: 'fibre', label: 'Fibre', kind: 'sum', key: 'fiber_mg', basis: 'Yours' },
  ],
  unmeasured: [],
  guides: [],
};
const whoActivity: PlanOption = {
  id: 'who-activity',
  group: 'fitness',
  kind: 'amounts',
  question: 'moving',
  start_here: true,
  relation: 'Written for adults 18–64.',
  name: 'WHO weekly activity',
  rests_on: 'guideline',
  summary: 'At least 150 minutes of moderate activity a week, and strengthening on 2 days.',
  sources: [
    { by: 'World Health Organization', title: 'Guidelines on physical activity (2020)', url: '' },
  ],
  needs: [],
  checks: [
    {
      id: 'minutes',
      label: 'Activity this week',
      kind: 'minutes',
      min: 150,
      basis: 'WHO: at least 150 min',
    },
    {
      id: 'strength',
      label: 'Strengthening days this week',
      kind: 'days',
      min: 2,
      basis: 'WHO: 2 or more days',
    },
  ],
  unmeasured: [],
  examples: [
    {
      kind: 'published',
      by: 'Centers for Disease Control and Prevention',
      title: 'Adding Physical Activity as an Adult',
      url: 'https://www.cdc.gov/',
      note: 'The CDC’s own first example week; only a simple indication to start from.',
      week: ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'].map((day) => ({
        day,
        items:
          day === 'wed'
            ? [{ type: 'activity' as const, name: 'Weight training', counts_as: 'strength' }]
            : day === 'mon'
              ? [
                  {
                    type: 'activity' as const,
                    name: 'Brisk walking',
                    minutes: 30,
                    effort: 5,
                    counts_as: 'aerobic',
                  },
                ]
              : [],
      })),
    },
  ],
  guides: [
    {
      label: 'Strength exercises to do at home',
      by: 'NHS',
      url: 'https://www.nhs.uk/',
      seen: '2026-10-12',
    },
  ],
};
const ownFitness: PlanOption = {
  id: 'own-fitness',
  group: 'fitness',
  kind: 'own',
  question: 'own',
  name: 'My own',
  rests_on: 'own',
  summary: 'Set each number yourself.',
  sources: [],
  needs: [],
  checks: [
    { id: 'minutes', label: 'Activity this week, minutes', kind: 'minutes', basis: 'Yours' },
  ],
  unmeasured: [],
  guides: [],
};
const PLANS = [who, energy, tre, ownFood, whoActivity, ownFitness];

export function planLibrary(notes: ObjectMeta[], args: { group: string }): PlanLibrary {
  const flags = planProfile(notes).flags ?? [];
  const group = PLANS.filter((p) => p.group === args.group);
  const left = (p: PlanOption) => (p.not_for ?? []).some((f) => flags.includes(f));
  return {
    plans: group.filter((p) => !left(p)),
    hidden: group.filter(left).map((p) => p.id),
    activities:
      args.group === 'fitness'
        ? [
            { name: 'Brisk walking', kind: 'aerobic', by: 'NHS' },
            { name: 'Yoga', kind: 'strength', by: 'NHS' },
          ]
        : [],
    weeks:
      args.group === 'fitness'
        ? [
            {
              by: 'Centers for Disease Control and Prevention',
              title: 'Adding Physical Activity as an Adult',
              url: 'https://www.cdc.gov/',
              seen: '2026-10-12',
              examples: [
                {
                  name: 'Example 1',
                  about: 'Moderate-intensity activity and muscle-strengthening activity',
                  days: ['30-minute brisk walk', '', '', 'Weight training', '', '', ''],
                  total: 'Total: 150 minutes',
                },
              ],
            },
          ]
        : [],
  };
}

export function planProfile(notes: ObjectMeta[]): PlanProfile {
  const n = notes.filter((x) => x.props.profile).at(-1);
  if (!n) return {};
  // The weight is the last one written down, as in `plan.rs`.
  const weighed = notes
    .filter((x) => x.props.log === 'body' && typeof x.props.weight_g === 'number' && x.start)
    .sort((a, b) => (a.start ?? '').localeCompare(b.start ?? ''))
    .at(-1);
  const num = (k: string) =>
    k === 'weight_g' && weighed
      ? (weighed.props.weight_g as number)
      : typeof n.props[k] === 'number'
        ? (n.props[k] as number)
        : undefined;
  return {
    id: n.id,
    sex: n.props.sex === 'female' || n.props.sex === 'male' ? n.props.sex : undefined,
    born: num('born'),
    height_cm: num('height_cm'),
    weight_g: num('weight_g'),
    pal_x10: num('pal_x10'),
    flags: FLAGS.filter((f) => n.props[f] === true || n.props[f] === 'true'),
  };
}

export function planPreview(notes: ObjectMeta[], args: { id: string }): PlanPreview {
  const plan = PLANS.find((p) => p.id === args.id);
  if (!plan) throw new Error(`no plan called '${args.id}' in this version`);
  const profile = planProfile(notes);
  const missing = plan.needs.filter((f) => profile[f as keyof PlanProfile] === undefined);
  // The stand-in's one "estimate": a fixed figure once the measurements are there.
  const estimate = plan.needs.length && !missing.length ? 2400 : undefined;
  const bounds: Record<string, PlanBound> = Object.fromEntries(
    plan.checks.map((c) => [
      c.id,
      c.id === 'energy' && estimate ? { max: estimate } : { min: c.min, max: c.max },
    ]),
  );
  return { plan, profile, missing, energy: estimate, bounds };
}

const day = (n: ObjectMeta) => (n.start ?? '').slice(0, 10);
const mondayOf = (d: string) => addDays(d, -((new Date(`${d}T12:00:00`).getDay() + 6) % 7));

/// One check measured on the week from `from`, up to `upto` (an index), with a four-week trend.
function measure(
  notes: ObjectMeta[],
  c: PlanCheck,
  bound: PlanBound,
  args: { group: string; day: string; planned?: boolean },
  plan?: string,
): PlanCheckProgress {
  const log = args.group === 'fitness' ? 'workout' : 'food';
  const weekDays = (from: string, planId?: string) => {
    const week = Array.from({ length: 7 }, (_, i) => addDays(from, i));
    const rows = (w: string) =>
      notes.filter(
        (n) => n.props.log === log && day(n) === w && (args.planned || !n.props.planned),
      );
    if (c.kind === 'kept')
      return week.map((w) =>
        notes.some((k) => k.props.log === 'kept' && k.props.plan === planId && day(k) === w)
          ? 1
          : null,
      );
    if (c.kind === 'sum' && c.key)
      return week.map((w) =>
        rows(w).length
          ? rows(w).reduce(
              (t, n) => t + (typeof n.props[c.key!] === 'number' ? (n.props[c.key!] as number) : 0),
              0,
            )
          : null,
      );
    if (c.kind === 'minutes')
      return week.map((w) => {
        const m = rows(w).reduce(
          (t, n) => t + (typeof n.props.minutes === 'number' ? (n.props.minutes as number) : 0),
          0,
        );
        return m > 0 ? m : null;
      });
    return week.map(() => null);
  };
  const per: 'day' | 'week' = c.kind === 'sum' || c.kind === 'share' ? 'day' : 'week';
  const unit =
    c.kind === 'sum'
      ? (c.key ?? '')
      : c.kind === 'share'
        ? 'percent'
        : c.kind === 'minutes'
          ? 'minutes'
          : 'days';
  const from = mondayOf(args.day);
  const at = Array.from({ length: 7 }, (_, i) => addDays(from, i)).indexOf(args.day);
  const days = weekDays(from, plan);
  const reading = (ds: (number | null)[], upto: number) => {
    const had = ds.slice(0, upto + 1).filter((x): x is number => x !== null);
    if (!had.length) return null;
    return per === 'day'
      ? Math.round(had.reduce((a, b) => a + b, 0) / had.length)
      : had.reduce((a, b) => a + b, 0);
  };
  const trend = [3, 2, 1].map((back) => reading(weekDays(addDays(from, -7 * back), plan), 6));
  trend.push(reading(days, at));
  const value =
    per === 'day'
      ? (days[at] ?? undefined)
      : days.slice(0, at + 1).reduce<number>((t, x) => t + (x ?? 0), 0);
  const state =
    value === undefined
      ? 'none'
      : bound.max !== undefined && value > bound.max
        ? 'over'
        : bound.min !== undefined && value < bound.min
          ? 'under'
          : 'met';
  return {
    id: c.id,
    label: c.label,
    kind: c.kind,
    unit,
    per,
    min: bound.min,
    max: bound.max,
    value,
    state,
    days,
    basis: c.basis,
    trend,
  };
}

export function planProgress(
  notes: ObjectMeta[],
  args: { group: string; day: string; planned?: boolean },
): PlanProgress {
  const program = args.group === 'fitness' ? 'training' : 'food';
  const from = mondayOf(args.day);
  const to = addDays(from, 6);
  const flags = planProfile(notes).flags ?? [];
  // One card per question, the latest; notes from before questions were kept: only the latest.
  const inForce = notes
    .filter((n) => n.props.program === program && n.props.plan)
    .filter((n) => !n.start || day(n) <= args.day)
    .filter((n) => !n.due || n.due.slice(0, 10) >= args.day)
    .sort((a, b) => day(a).localeCompare(day(b)));
  const latestOld = inForce.filter((n) => !n.props.question).at(-1);
  const byQuestion = new Map<string, ObjectMeta>();
  for (const n of inForce) {
    if (!n.props.question && n !== latestOld) continue;
    const q = String(
      n.props.question ?? PLANS.find((p) => p.id === n.props.plan)?.question ?? n.props.plan,
    );
    byQuestion.set(q, n);
  }
  const order = (q: string) => {
    const i = QUESTIONS.indexOf(q);
    return i < 0 ? QUESTIONS.length : i;
  };
  const followed: PlanFollowed[] = [...byQuestion.entries()]
    .sort(([a], [b]) => order(a) - order(b))
    .map(([question, note]) => {
      const plan = PLANS.find((p) => p.id === note.props.plan);
      if (!plan) return { question, note, unknown: String(note.props.plan), checks: [] };
      const num = (k: string) =>
        typeof note.props[k] === 'number' ? (note.props[k] as number) : undefined;
      const checks = plan.checks
        .filter((c) => !(c.hide_if ?? []).some((f) => flags.includes(f as PlanFlag)))
        .flatMap((c) => {
          const bound = { min: num(`p_${c.id}_min`), max: num(`p_${c.id}_max`) };
          if (bound.min === undefined && bound.max === undefined) return [];
          return [measure(notes, c, bound, args, plan.id)];
        });
      const kept = notes.find(
        (n) => n.props.log === 'kept' && n.props.plan === plan.id && day(n) === args.day,
      );
      return { question, note, plan, checks, kept_today: kept?.id };
    });
  const first = followed[0];
  return {
    from,
    to,
    day: args.day,
    checks: first?.checks ?? [],
    plan: first?.plan,
    note: first?.note,
    unknown: first?.unknown,
    kept_today: first?.kept_today,
    followed,
  };
}

/// The "worth watching" set: a small stand-in of the real one.
const WATCH: (PlanCheck & { group: string })[] = [
  {
    group: 'food',
    id: 'fibre',
    label: 'Fibre',
    kind: 'sum',
    key: 'fiber_mg',
    min: 25_000,
    basis: 'WHO and EFSA: at least 25 g a day',
  },
  {
    group: 'food',
    id: 'sodium',
    label: 'Sodium',
    kind: 'sum',
    key: 'sodium_ug',
    max: 2_000_000,
    basis: 'WHO and EFSA: 2 g a day',
  },
  {
    group: 'fitness',
    id: 'minutes',
    label: 'Activity this week',
    kind: 'minutes',
    min: 150,
    basis: 'WHO: 150–300 minutes a week',
  },
];

export function planWatch(notes: ObjectMeta[], args: { group: string; day: string }): PlanWatch {
  const from = mondayOf(args.day);
  return {
    from,
    to: addDays(from, 6),
    checks: WATCH.filter((w) => w.group === args.group).map((w) =>
      measure(notes, w, { min: w.min, max: w.max }, args),
    ),
  };
}
