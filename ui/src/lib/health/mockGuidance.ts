/// The Health cards, for the in-memory backend used by `pnpm dev` and the tests.
///
/// **A stand-in, not a second implementation**, like `mockPlan.ts`: the real cards and their
/// counting are `guidance.rs`, tested there. This carries three of them and counts what a screen
/// test needs: days since the diary began, days of this week or month at 1 or more, and cycle
/// lengths with a cycle marked as missed logging.
import { addDays } from '../units';
import type {
  HealthCard,
  HealthCounted,
  HealthFollowed,
  HealthGuidance,
  ObjectMeta,
} from '../types';

const CARDS: HealthCard[] = [
  {
    id: 'headache-diary',
    shelf: 'diary',
    name: 'Keep a headache diary',
    rests_on: 'guideline',
    says: [{ by: 'NICE CG150, 1.1.4', text: '…record the following for a minimum of 8 weeks…' }],
    sources: [{ by: 'NICE', title: 'CG150', url: 'https://www.nice.org.uk/guidance/cg150' }],
    trackers: ['headache_0to3'],
    checks: [
      {
        id: 'kept',
        label: 'Days since the diary was started',
        kind: 'kept',
        min: 56,
        basis: 'NICE CG150 1.1.4: "for a minimum of 8 weeks"',
      },
    ],
    not_said: ['A diary does not diagnose.'],
  },
  {
    id: 'painkiller-days',
    shelf: 'diary',
    name: 'Painkiller days',
    rests_on: 'guideline',
    says: [
      {
        by: 'NHS, Migraine',
        nhs: true,
        text: 'Try not to take painkillers for more than 2 days a week…',
      },
    ],
    sources: [{ by: 'NHS', title: 'Migraine', url: 'https://www.nhs.uk/conditions/migraine/' }],
    trackers: [],
    choose: [
      { id: 'simple', label: 'Paracetamol, aspirin or an NSAID (such as ibuprofen)' },
      { id: 'specific', label: 'A triptan, an opioid, an ergot, or a combination painkiller' },
    ],
    checks: [
      {
        id: 'week',
        label: 'Days this week with any painkiller',
        kind: 'week_days',
        keys_from: ['simple', 'specific'],
        max: 2,
        basis: 'NHS: "more than 2 days a week"',
      },
      {
        id: 'simple',
        label: 'Days this month with paracetamol, aspirin or an NSAID',
        kind: 'month_days',
        keys_from: ['simple'],
        at_or_over: 15,
        basis: 'NICE CG150 1.2.7: "on 15 days per month or more"',
      },
    ],
    not_said: ['It does not say you have medication overuse headache.'],
  },
  {
    id: 'dust-mites',
    shelf: 'nose',
    name: 'Fewer dust mites in the bedroom',
    rests_on: 'review',
    says: [{ by: 'AAAAI', text: 'Bedding should be washed weekly in hot water (130° F)…' }],
    disagree: 'A hospital leaflet and Cochrane disagree about covers; each is named.',
    sources: [{ by: 'AAAAI', title: 'Indoor allergens', url: 'https://www.aaaai.org/' }],
    trackers: ['bedding_washed_yes'],
    checks: [
      {
        id: 'weekly',
        label: 'Days this week the bedding was washed',
        kind: 'week_days',
        keys: ['bedding_washed_yes'],
        min: 1,
        basis: 'AAAAI: "washed weekly"',
      },
    ],
    not_said: ['It does not say these measures relieve symptoms.'],
  },
];

CARDS.push({
  id: 'your-cycle',
  shelf: 'cycle',
  name: 'Your cycle',
  rests_on: 'definition',
  says: [
    {
      by: 'FIGO System 1',
      text: 'The normal length of a menstrual cycle ranges from 24 to 38 days…',
    },
  ],
  sources: [
    { by: 'FIGO', title: 'System 1', url: 'https://pmc.ncbi.nlm.nih.gov/articles/PMC10952771/' },
  ],
  trackers: ['period_start_yes', 'period_flow_0to3'],
  checks: [
    {
      id: 'lengths_figo',
      label: 'Cycles of the last 6 months, in days',
      kind: 'cycle_lengths',
      min: 24,
      max: 38,
      basis: 'FIGO: "ranges from 24 to 38 days"',
    },
  ],
  not_said: ['Nothing is predicted.'],
});

const PHQ = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'].map((k) => `phq9_${k}_0to3`);
CARDS.push({
  id: 'mood-phq9',
  shelf: 'mind',
  name: 'Mood (PHQ-9)',
  rests_on: 'definition',
  urgent: [
    { by: 'SCDF, Singapore', text: '995: Fire / Rescue / Emergency Ambulance' },
    {
      by: 'National mindline 1771, Singapore',
      text: '1771: "Get 24-hour access to our helpline".',
    },
  ],
  says: [
    { by: 'PHQ-9 instructions (Pfizer)', text: 'Scores of 5, 10, 15, and 20 represent cutpoints…' },
  ],
  sources: [
    { by: 'Pfizer', title: 'PHQ screeners', url: 'https://www.phqscreeners.com/select-screener' },
  ],
  trackers: [],
  form: {
    prompt: 'Over the last 2 weeks, how often have you been bothered by any of the following:',
    answers: ['Not at all', 'Several days', 'More than half the days', 'Nearly every day'],
    items: PHQ.map((key, i) => ({ key, text: `Question ${i + 1}?` })),
  },
  checks: [
    {
      id: 'total',
      label: 'PHQ-9 total, latest',
      kind: 'questionnaire',
      keys: PHQ,
      basis: '0 to 3 each, added',
    },
  ],
  not_said: ['A score is not a diagnosis.'],
});

/** As `guidance::measure_questionnaire`: the latest day in 4 weeks with every item answered. */
function questionnaire(
  entries: ObjectMeta[],
  day: string,
  c: HealthCard['checks'][number],
): HealthCounted {
  const keys = c.keys ?? [];
  for (let d = day; d >= addDays(day, -27); d = addDays(d, -1)) {
    const here = entries.filter((e) => (e.start ?? '').slice(0, 10) === d);
    const values = keys.map((k) => here.map((e) => e.props[k]).find((v) => typeof v === 'number'));
    if (values.every((v) => typeof v === 'number'))
      return {
        id: c.id,
        label: c.label,
        basis: c.basis,
        value: (values as number[]).reduce((a, b) => a + b, 0),
        values: values as number[],
        of: 1,
        recorded: 1,
        from: d,
        to: day,
      };
  }
  return {
    id: c.id,
    label: c.label,
    basis: c.basis,
    value: 0,
    of: 0,
    recorded: 0,
    from: addDays(day, -27),
    to: day,
    not_compared: 'not filled in in the last 4 weeks',
  };
}

/** As `cycle::cycles`, without contexts: first day to first day, a marked gap left out. */
function cycleLengths(
  entries: ObjectMeta[],
  day: string,
  c: HealthCard['checks'][number],
): HealthCounted {
  const on = (d: string) => entries.filter((e) => (e.start ?? '').slice(0, 10) === d);
  const firsts = [
    ...new Set(
      entries
        .filter((e) => e.props.period_start_yes === 1 && (e.start ?? '').slice(0, 10) <= day)
        .map((e) => (e.start ?? '').slice(0, 10)),
    ),
  ].sort();
  const since = addDays(day, -183);
  const cycles = [];
  for (let i = 0; i + 1 < firsts.length; i++) {
    const [a, b] = [firsts[i], firsts[i + 1]];
    if (a < since) continue;
    let length = 0;
    let noted = 0;
    let gap = false;
    for (let d = a; d < b; d = addDays(d, 1)) {
      length += 1;
      const here = on(d);
      if (here.length) noted += 1;
      if (here.some((e) => e.props.cycle_gap_yes === 1)) gap = true;
    }
    cycles.push({
      start: a,
      length,
      noted,
      note: on(a)[0]?.id ?? '',
      ...(gap ? { left_out: 'gap' } : {}),
    });
  }
  const values = cycles.filter((x) => !x.left_out).map((x) => x.length);
  return {
    id: c.id,
    label: c.label,
    basis: c.basis,
    value: values.length,
    of: cycles.length,
    recorded: values.length,
    from: since,
    to: day,
    min: c.min,
    max: c.max,
    ...(values.length ? { within: values.every((v) => v >= c.min! && v <= c.max!) } : {}),
    values,
    cycles,
  };
}

/** As `guidance::own_card`: a card the person wrote, from its note. */
const owns: HealthCard[] = [];
function ownCard(n: ObjectMeta): HealthCard {
  const key = String(n.props.own_key ?? '');
  const card: HealthCard = {
    id: `own:${n.id}`,
    shelf: 'own',
    name: n.title || 'Your own card',
    rests_on: 'own',
    says: [],
    sources: [],
    trackers: key ? [key] : [],
    checks: key
      ? [
          {
            id: 'own',
            label: `Days this week with ${key}`,
            kind: 'week_days',
            keys: [key],
            ...(typeof n.props.own_min === 'number' ? { min: n.props.own_min } : {}),
            ...(typeof n.props.own_max === 'number' ? { max: n.props.own_max } : {}),
            basis: 'Your own number',
          },
        ]
      : [],
    not_said: ["This card is yours: its number is your own, not a body's."],
  };
  owns.push(card);
  return card;
}

const mondayOf = (d: string) => addDays(d, -((new Date(`${d}T12:00:00`).getDay() + 6) % 7));

export function healthGuidance(notes: ObjectMeta[], day: string): HealthGuidance {
  const entries = notes.filter((n) => n.props.log === 'health');
  const valueOn = (keys: string[], d: string): number | undefined => {
    let best: number | undefined;
    for (const e of entries)
      if ((e.start ?? '').slice(0, 10) === d)
        for (const k of keys)
          if (typeof e.props[k] === 'number') best = Math.max(best ?? 0, e.props[k] as number);
    return best;
  };
  const followed: HealthFollowed[] = [];
  for (const n of notes) {
    if (n.props.program !== 'health' || n.tags.includes('template')) continue;
    const card = n.props.plan === 'own' ? ownCard(n) : CARDS.find((c) => c.id === n.props.plan);
    const start = (n.start ?? '').slice(0, 10);
    if (!card || (start && start > day) || (n.due && n.due.slice(0, 10) < day)) continue;
    const chosen = Object.fromEntries(
      (card.choose ?? []).map((c) => [
        c.id,
        String(n.props[`p_${c.id}`] ?? '')
          .split(',')
          .map((k) => k.trim())
          .filter(Boolean),
      ]),
    );
    const checks: HealthCounted[] = card.checks.map((c) => {
      if (c.kind === 'cycle_lengths') return cycleLengths(entries, day, c);
      if (c.kind === 'questionnaire') return questionnaire(entries, day, c);
      const keys =
        c.kind === 'kept' && !c.keys?.length
          ? card.trackers
          : [...(c.keys ?? []), ...(c.keys_from ?? []).flatMap((f) => chosen[f] ?? [])];
      let from = c.kind === 'month_days' ? day.slice(0, 8) + '01' : mondayOf(day);
      if (c.kind === 'kept') {
        const first = entries
          .filter((e) => keys.some((k) => k in e.props))
          .map((e) => (e.start ?? '').slice(0, 10))
          .sort()[0];
        from = first && first <= day ? first : day;
      }
      let of = 0;
      let recorded = 0;
      let at = 0;
      for (let d = from; d <= day; d = addDays(d, 1)) {
        of += 1;
        const v = valueOn(keys, d);
        if (v === undefined) continue;
        recorded += 1;
        if (v >= 1) at += 1;
      }
      const value = c.kind === 'kept' ? (recorded ? of : 0) : at;
      const within = !keys.length
        ? undefined
        : c.min !== undefined
          ? value >= c.min
          : c.max !== undefined
            ? value <= c.max
            : c.at_or_over !== undefined
              ? value < c.at_or_over
              : undefined;
      return {
        id: c.id,
        label: c.label,
        basis: c.basis,
        value,
        of: c.kind === 'kept' && !recorded ? 0 : of,
        recorded,
        from,
        to: day,
        ...(c.min !== undefined ? { min: c.min } : {}),
        ...(c.max !== undefined ? { max: c.max } : {}),
        ...(c.at_or_over !== undefined ? { at_or_over: c.at_or_over } : {}),
        ...(within !== undefined ? { within } : {}),
        ...(!keys.length && c.keys_from?.length ? { unchosen: true } : {}),
      };
    });
    followed.push({ card: card.id, note: n.id, since: start, chosen, checks });
  }
  return {
    nhs_credit:
      'Information from the NHS website, licensed under the Open Government Licence v3.0, as at 22 October 2026.',
    cards: [...CARDS, ...owns.splice(0)],
    followed,
  };
}
