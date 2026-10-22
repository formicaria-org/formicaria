// What the Health diary starts from, and what its scales are called (`decisions.md#ui` `#data`,
// 2026-10-22, *a fifth group, Health*; every row's source is in `health-plan-2026-10-22.md` §3).
//
// **A starter is a set of keys written into the person's own Health template** when they ask for
// it: a template is a note, and none is held in the app (`decisions.md#ui`, 2026-10-12). The person
// then adds, and removes, what they track. Nothing here is a claim about the person.
import type { TrackerKind } from '../types';

/** The note a person's trackers are kept on. */
export const TEMPLATE = {
  title: 'Health diary',
  tags: ['template'],
  group: 'Health',
  body:
    'What you track in the Health diary. Each property is one thing you track, and the end of its ' +
    'name says what kind it is: _0to3 none to severe, _0to10 a number from 0 to 10, _0to100 how ' +
    'much it bothered you, _yes yes or no, _count or _doses how many times, _min _ml _mg _g _mmhg ' +
    '_pct _c an amount in that unit, _score a questionnaire total.',
};

/** Said on every Mind screen and note (owner, 2026-10-22: Mind ships with Singapore's contacts). */
export const CONTACTS_LINE =
  'In an emergency: 995. National mindline 1771, 24 hours (WhatsApp +65 6669 1771).';
/** The one caution the research asks for: in one trial, daily mood rating did no better (MONARCA I). */
export const MIND_LINE =
  'A private notebook, not a test: nothing you write is scored. If keeping it makes you feel worse, stop or take a break.';

export interface Starter {
  name: string;
  /** What the starter is for, in a line. */
  about: string;
  keys: string[];
  /** Where its fields and its scale come from. */
  source: string;
  /** Said before it is added, when there is something the person should know first. */
  before?: string;
}

export const STARTERS: Starter[] = [
  {
    name: 'Headache',
    about: 'How bad, whether it lasted 4 hours or more, and what came with it.',
    keys: [
      'headache_0to3',
      'headache_4h_or_longer_yes',
      'nausea_yes',
      'light_hurts_yes',
      'noise_hurts_yes',
    ],
    source:
      'Fields: IHS guidelines for headache diaries (Tassorelli 2018) and NICE CG150 1.1.4. ' +
      'Scale: IHS, 0 none to 3 severe (Diener 2019).',
  },
  {
    name: 'Nose and eyes',
    about: 'A runny, blocked or itchy nose, sneezing, itchy or watery eyes.',
    keys: [
      'runny_nose_0to3',
      'blocked_nose_0to3',
      'itchy_nose_0to3',
      'sneezing_0to3',
      'itchy_eyes_0to3',
      'watery_eyes_0to3',
    ],
    source:
      'Nose: the four symptoms of the Total Nasal Symptom Score, each 0 to 3 (US FDA, Allergic ' +
      'Rhinitis guidance, 2018). Eyes on the same scale.',
  },
  {
    name: 'Cycle',
    about: 'The first day of each period, how much bleeding, spotting, and pain.',
    keys: ['period_start_yes', 'period_flow_0to3', 'spotting_yes', 'period_pain_0to3'],
    source:
      'A cycle runs from one first day to the next (FIGO System 1). Flow in the words Apple Health ' +
      'and Android Health Connect use: none, light, medium, heavy. Nothing is predicted.',
    before:
      'These notes go wherever this notebook is backed up: its git remote and any snapshot are ' +
      'copies, and a deleted note stays in its history. The manual says how to keep them to yourself.',
  },
  {
    name: 'Blood pressure',
    about: 'Two readings in the morning and two in the evening, for 7 days.',
    keys: ['am1', 'am2', 'pm1', 'pm2'].flatMap((t) => [`bp_${t}_sys_mmhg`, `bp_${t}_dia_mmhg`]),
    source:
      'NICE NG136 1.2.7, in our words: two readings a minute apart, sitting, morning and evening, ' +
      'for 4 to 7 days, the first day left out. The layout follows the BIHS home diary.',
  },
  {
    name: 'Bladder',
    about: 'How often, how much, leaks, pads and drinks, for at least 3 days.',
    keys: [
      'voids_count',
      'night_voids_count',
      'urine_ml',
      'leaks_count',
      'pads_count',
      'drinks_ml',
    ],
    source:
      'NICE NG123 1.3, in our words: a bladder diary over at least 3 days, working and leisure days both.',
  },
  {
    name: 'Mood and activity',
    about:
      'Your mood, and what you planned and did, with how much pleasure and achievement it brought.',
    keys: [
      'mood_0to10',
      'activities_planned_count',
      'activities_done_count',
      'pleasure_0to10',
      'achievement_0to10',
    ],
    source:
      'Behavioural activation: plan something, do it, note how it felt (Cuijpers 2026, 105 trials; ' +
      'the COBRA trial, 2016). In our own words; the mood scale is your own 0 to 10.',
    before: `${MIND_LINE} ${CONTACTS_LINE}`,
  },
  {
    name: 'Sleep',
    about: 'How long you slept, how long it took, times awake, and how well.',
    keys: ['sleep_min', 'fall_asleep_min', 'awakenings_count', 'awake_min', 'sleep_quality_0to10'],
    source: 'In our own words, after the morning sleep diary of Carney et al. 2012.',
  },
];

/** **Plain names for the starters' keys**, where the key's own words read badly
 *  (`bp_am1_sys_mmhg` → *Morning reading 1, systolic*). Shown, never stored. */
export const LABELS: Record<string, string> = {
  ...Object.fromEntries(
    (
      [
        ['am1', 'Morning reading 1'],
        ['am2', 'Morning reading 2'],
        ['pm1', 'Evening reading 1'],
        ['pm2', 'Evening reading 2'],
      ] as const
    ).flatMap(([t, name]) => [
      [`bp_${t}_sys_mmhg`, `${name}, systolic`],
      [`bp_${t}_dia_mmhg`, `${name}, diastolic`],
    ]),
  ),
  mood_0to10: 'Mood',
  activities_planned_count: 'Things planned',
  activities_done_count: 'Things done of those planned',
  pleasure_0to10: 'Pleasure',
  achievement_0to10: 'Achievement',
  sleep_min: 'Sleep',
  fall_asleep_min: 'Time to fall asleep',
  awakenings_count: 'Times awake in the night',
  awake_min: 'Time awake in the night',
  sleep_quality_0to10: 'How well you slept',
  voids_count: 'Times passing urine',
  night_voids_count: 'Times at night',
  urine_ml: 'Urine measured',
  leaks_count: 'Leaks',
  pads_count: 'Pads changed',
  drinks_ml: 'Drinks',
};
/** A tracker with its plain name, when it has one. */
export const named = <T extends { key: string; label: string }>(t: T): T =>
  LABELS[t.key] ? { ...t, label: LABELS[t.key] } : t;

/** The words of a `_0to3` tracker that is not a severity: a period's flow, in the vocabulary of
 *  Apple Health (`HKCategoryValueVaginalBleeding`: none, light, medium, heavy) and Android Health
 *  Connect (`MenstruationFlowRecord`: light, medium, heavy). */
export const WORDS: Record<string, string[]> = {
  period_flow_0to3: ['None', 'Light', 'Medium', 'Heavy'],
};

/** The four words of a `_0to3` tracker: IHS's *"0 = no headache; 1 = mild headache; 2 = moderate
 *  headache; 3 = severe headache"* (Diener 2019), used for any severity. */
export const SEVERITY = ['None', 'Mild', 'Moderate', 'Severe'];

/** The ends of a `_0to100` tracker, adapted from Sousa-Pinto et al. 2021 (CC BY 4.0). */
export const BOTHER = { low: 'Not at all bothersome', high: 'Extremely bothersome' };
export const BOTHER_CREDIT =
  'Bother scale adapted from Sousa-Pinto et al., Clin Transl Allergy 2021, CC BY 4.0.';

/** Where a questionnaire a score was typed from is kept by its holder. The items are not carried:
 *  their terms do not allow it (`health-plan-2026-10-22.md` §3.4). */
export const SCORE_HOLDERS: Record<string, { name: string; url: string }> = {
  snot22: {
    name: 'SNOT-22, Washington University in St. Louis',
    url: 'https://sinonasaltest.wustl.edu/',
  },
  hit6: { name: 'HIT-6, IQVIA', url: 'https://coas.iqvia.com/COAs/headache-impact-test-standard' },
  epworth: {
    name: 'Epworth Sleepiness Scale, M. W. Johns',
    url: 'https://epworthsleepinessscale.com/',
  },
};

/** What a person picks when they track something new, and the key's ending each makes. */
export const KINDS: { id: string; label: string; kind: TrackerKind['kind']; suffix: string }[] = [
  { id: 'severity', label: 'None to severe', kind: 'severity', suffix: '_0to3' },
  { id: 'rating', label: 'A number from 0 to 10', kind: 'rating', suffix: '_0to10' },
  { id: 'bother', label: 'How much it bothered you, 0 to 100', kind: 'bother', suffix: '_0to100' },
  { id: 'yesno', label: 'Yes or no', kind: 'yesno', suffix: '_yes' },
  { id: 'count', label: 'How many times', kind: 'count', suffix: '_count' },
  // A count too, ending `_doses`: it is what puts a medicine under Medication.
  { id: 'doses', label: 'Doses of a medicine', kind: 'count', suffix: '_doses' },
  { id: 'amount', label: 'An amount', kind: 'amount', suffix: '' },
  { id: 'score', label: 'A questionnaire score', kind: 'score', suffix: '_score' },
];

// ── sections ────────────────────────────────────────────────────────────────────────────────
/** **The Diary's sections**, in the order they are shown. A section is read off the key, never
 *  stored: the starters' keys are theirs, a card's trackers sit where its shelf is, a key ending
 *  `_doses` is a medicine, and anything else the person named is theirs. */
export const SECTIONS = [
  'Headache',
  'Nose and eyes',
  'Cycle',
  'Blood pressure',
  'Bladder',
  'Mood and activity',
  'Sleep',
  'Medication',
  'Every day',
  'Yours',
];
/** The trackers the cards bring, by the section their shelf belongs to. */
const CARD_SECTIONS: Record<string, string> = {
  nose_rinse_count: 'Nose and eyes',
  decongestant_spray_yes: 'Nose and eyes',
  bedding_washed_yes: 'Nose and eyes',
  bedding_wash_c: 'Nose and eyes',
  clots_large_yes: 'Cycle',
  sleep_min: 'Every day',
  caffeine_mg: 'Every day',
};
export function sectionOf(key: string): string {
  const starter = STARTERS.find((s) => s.keys.includes(key));
  if (starter) return starter.name;
  if (CARD_SECTIONS[key]) return CARD_SECTIONS[key];
  if (key.endsWith('_doses')) return 'Medication';
  return 'Yours';
}
/** Things tracked, in their sections, in order; empty sections left out. */
export function sectioned<T extends { key: string }>(list: T[]): { name: string; items: T[] }[] {
  return SECTIONS.map((name) => ({
    name,
    items: list.filter((t) => sectionOf(t.key) === name),
  })).filter((s) => s.items.length > 0);
}

/** The units an amount may be in, as `health.rs` accepts them, with what each is called. */
export const UNITS: { unit: string; label: string; per?: number }[] = [
  { unit: 'min', label: 'minutes' },
  { unit: 'ml', label: 'millilitres' },
  { unit: 'mg', label: 'milligrams' },
  { unit: 'g', label: 'grams' },
  { unit: 'mmhg', label: 'mmHg' },
  { unit: 'pct', label: 'percent' },
  { unit: 'c', label: '°C' },
  // Tenths, so a temperature or a glucose stays a whole number: 38.1 °C is kept as 381.
  { unit: 'c10', label: '°C, to a tenth', per: 10 },
  { unit: 'mmoll10', label: 'mmol/L, to a tenth', per: 10 },
];
/** How many of a unit make one shown: 10 for tenths, else 1. */
export const perOf = (unit: string) => UNITS.find((u) => u.unit === unit)?.per ?? 1;

// ── the health record ──────────────────────────────────────────────────────────────────────
/** **The record's templates**, written into the notebook when a person starts a record: ordinary
 *  notes on the Library's `health` shelf, known by `record` (`record.rs`; the International Patient
 *  Summary's sections, and FDA's *My Medicine Record* for a medicine's lines). They carry no
 *  `group`, so they are never taken for the diary's own template. */
export const RECORD_TEMPLATES: {
  title: string;
  record: string;
  status?: string;
  about?: string;
  body: string;
}[] = [
  {
    title: 'Medicine',
    record: 'medicine',
    status: 'current',
    body:
      '## How much and when\n\n## Why\n\n## Who told me to take it\n\n## What it looks like\n\n' +
      'Set the first day as its start, and its last day when you stop. Keep it: a stopped medicine ' +
      'is history worth having. Its status is current, stopped or on hold.',
  },
  {
    title: 'Allergy or intolerance',
    record: 'allergy',
    status: 'active',
    body:
      '## What happens\n\n## How bad\n\n## Since\n\n' +
      'Medicines, foods, skin products and medical tape too. Its status is active, inactive or resolved.',
  },
  {
    title: 'Condition',
    record: 'condition',
    status: 'active',
    body: '## Since\n\n## Notes\n\nIts status is active, in remission or resolved.',
  },
  {
    title: 'Result',
    record: 'result',
    body:
      "## Value, as printed\n\n## The lab's range and flag, as printed\n\n## Lab\n\n" +
      'Attach the report. Ranges belong to the lab that printed them: copy them, never look one up.',
  },
  {
    title: 'Visit',
    record: 'visit',
    body: '## Questions to ask\n\n## What was said\n\n## What happens next\n',
  },
  { title: 'Immunization', record: 'immunization', body: '## Vaccine and dose\n\n## Where\n' },
  { title: 'Procedure', record: 'procedure', body: '## What and where\n\n## Notes\n' },
  { title: 'Device', record: 'device', status: 'current', body: '## What it is\n\n## Since\n' },
  { title: 'Contact', record: 'contact', body: '## Role\n\n## Phone\n\n## Address\n' },
  {
    title: 'No known allergies',
    record: 'none_known',
    about: 'allergy',
    body: 'Said, so that no allergies is told apart from allergies not written down.',
  },
];

/** A key's stem from what a person typed: lower case, words joined by `_`, letters and digits
 *  only. Empty when nothing usable is left (a name in another script, for now). */
export function stemOf(name: string): string {
  return name
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .replace(/^[0-9_]+/, '')
    .slice(0, 32)
    .replace(/_+$/, '');
}

/** **Mind notes**: templates for a thought record, worry time and a piece of writing, kept on the
 *  Library's `mind` shelf. Our own wording: the templates people know are not free to carry
 *  (`health-plan-2026-10-22.md` §10.2). Each opens with the contacts. */
export const MIND_TEMPLATES: { title: string; record: string; body: string }[] = [
  {
    title: 'Thought record',
    record: 'thought',
    body:
      `${CONTACTS_LINE}\n\n## What happened\n\n## The thought\n\n## How it felt, and how strong (0 to 100)\n\n` +
      '## What else could be true\n\n## How it feels now (0 to 100)\n',
  },
  {
    title: 'Worry time',
    record: 'worry',
    body: `${CONTACTS_LINE}\n\n## The worries\n\n## Which of them I can do something about\n\n## What I will do, and when\n`,
  },
  {
    title: 'Writing',
    record: 'writing',
    body:
      `${CONTACTS_LINE}\n\nPeople often feel worse straight after writing about something hard; ` +
      'any good tends to come later, and with a few days between sessions. Stop when you need to.\n\n',
  },
];
