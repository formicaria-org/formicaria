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
} from './types';

const who: PlanOption = {
  id: 'who-healthy-diet',
  group: 'food',
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
  guides: [],
};
const energy: PlanOption = {
  id: 'energy-for-me',
  group: 'food',
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

export function planLibrary(args: { group: string }): PlanLibrary {
  return {
    plans: PLANS.filter((p) => p.group === args.group),
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

export function planProgress(
  notes: ObjectMeta[],
  args: { group: string; day: string; planned?: boolean },
): PlanProgress {
  const program = args.group === 'fitness' ? 'training' : 'food';
  const log = args.group === 'fitness' ? 'workout' : 'food';
  const d = new Date(`${args.day}T12:00:00`);
  const from = addDays(args.day, -((d.getDay() + 6) % 7));
  const to = addDays(from, 6);
  const day = (n: ObjectMeta) => (n.start ?? '').slice(0, 10);
  const note = notes
    .filter((n) => n.props.program === program && n.props.plan)
    .filter((n) => !n.start || day(n) <= args.day)
    .filter((n) => !n.due || n.due.slice(0, 10) >= args.day)
    .sort((a, b) => day(a).localeCompare(day(b)))
    .at(-1);
  const base = { from, to, day: args.day, checks: [] as PlanCheckProgress[] };
  if (!note) return base;
  const plan = PLANS.find((p) => p.id === note.props.plan);
  if (!plan) return { ...base, note, unknown: String(note.props.plan) };

  const week = Array.from({ length: 7 }, (_, i) => addDays(from, i));
  const kept = notes.filter((n) => n.props.log === 'kept' && n.props.plan === plan.id);
  const checks = plan.checks.flatMap((c): PlanCheckProgress[] => {
    const num = (k: string) =>
      typeof note.props[k] === 'number' ? (note.props[k] as number) : undefined;
    const min = num(`p_${c.id}_min`);
    const max = num(`p_${c.id}_max`);
    if (min === undefined && max === undefined) return [];
    let days: (number | null)[] = week.map(() => null);
    let per: 'day' | 'week' = 'week';
    let unit = 'days';
    if (c.kind === 'kept') days = week.map((w) => (kept.some((k) => day(k) === w) ? 1 : null));
    else if (c.kind === 'sum' && c.key) {
      per = 'day';
      unit = c.key;
      days = week.map((w) => {
        const rows = notes.filter(
          (n) => n.props.log === log && day(n) === w && (args.planned || !n.props.planned),
        );
        return rows.length
          ? rows.reduce(
              (t, n) => t + (typeof n.props[c.key!] === 'number' ? (n.props[c.key!] as number) : 0),
              0,
            )
          : null;
      });
    } else if (c.kind === 'share') {
      per = 'day';
      unit = 'percent';
    } else if (c.kind === 'minutes') unit = 'minutes';
    const at = week.indexOf(args.day);
    const value =
      per === 'day'
        ? (days[at] ?? undefined)
        : c.kind === 'kept'
          ? days.filter(Boolean).length
          : undefined;
    const state =
      value === undefined
        ? 'none'
        : max !== undefined && value > max
          ? 'over'
          : min !== undefined && value < min
            ? 'under'
            : 'met';
    return [
      {
        id: c.id,
        label: c.label,
        kind: c.kind,
        unit,
        per,
        min,
        max,
        value,
        state,
        days,
        basis: c.basis,
      },
    ];
  });
  return {
    ...base,
    plan,
    note,
    checks,
    kept_today: kept.find((k) => day(k) === args.day)?.id,
  };
}
