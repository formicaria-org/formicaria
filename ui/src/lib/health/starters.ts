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
];

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
export const KINDS: { label: string; kind: TrackerKind['kind']; suffix: string }[] = [
  { label: 'None to severe', kind: 'severity', suffix: '_0to3' },
  { label: 'A number from 0 to 10', kind: 'rating', suffix: '_0to10' },
  { label: 'How much it bothered you, 0 to 100', kind: 'bother', suffix: '_0to100' },
  { label: 'Yes or no', kind: 'yesno', suffix: '_yes' },
  { label: 'How many times', kind: 'count', suffix: '_count' },
  { label: 'An amount', kind: 'amount', suffix: '' },
  { label: 'A questionnaire score', kind: 'score', suffix: '_score' },
];

/** The units an amount may be in, as `health.rs` accepts them, with what each is called. */
export const UNITS: { unit: string; label: string }[] = [
  { unit: 'min', label: 'minutes' },
  { unit: 'ml', label: 'millilitres' },
  { unit: 'mg', label: 'milligrams' },
  { unit: 'g', label: 'grams' },
  { unit: 'mmhg', label: 'mmHg' },
  { unit: 'pct', label: 'percent' },
  { unit: 'c', label: '°C' },
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
