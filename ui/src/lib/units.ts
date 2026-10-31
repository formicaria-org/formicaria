// A tracked number is a whole number whose unit is the end of its key (`protein_mg`, `iron_ug`,
// `load_g`, `duration_s`; `kcal` is its own unit). This is the one place that reads that suffix,
// so no view definition carries a unit or a scale (`decisions.md#ui`, 2026-10-11).
import { appToday, clockText } from './appTime';
import { addDays as civilAddDays, formatCivil } from './civil';

export type Shown = { text: string; unit: string };

const group = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

/** One decimal below ten, none from ten up — `8.5`, `82`, `1,240`. */
function tidy(n: number): string {
  const a = Math.abs(n);
  if (a >= 10 || Number.isInteger(n)) return group(Math.round(n));
  return (Math.round(n * 10) / 10).toFixed(1).replace(/\.0$/, '');
}

type Step = { unit: string; per: number };
/** The units a suffix may be shown in, smallest first. `per` is how many stored units make one. */
const LADDERS: Record<string, Step[]> = {
  // Vitamin B12 and D are kept in nanograms (2026-10-29: in whole micrograms, milk's B12 was 0) and
  // read in micrograms, with the decimal that whole micrograms could not keep: 0.2 µg, 4 µg.
  ng: [{ unit: 'µg', per: 1000 }],
  ug: [
    { unit: 'µg', per: 1 },
    // …and no further: a value kept in micrograms is a vitamin or a mineral, and those are read
    // in milligrams however many there are (calcium is 1,000 mg, never 1 g).
    { unit: 'mg', per: 1000 },
  ],
  mg: [
    { unit: 'mg', per: 1 },
    { unit: 'g', per: 1000 },
  ],
  g: [
    { unit: 'g', per: 1 },
    { unit: 'kg', per: 1000 },
  ],
  // A food whose label is by volume (`amount_ml`): never turned into grams.
  ml: [
    { unit: 'ml', per: 1 },
    { unit: 'l', per: 1000 },
  ],
};

/** **Water is read by volume** (2026-10-31): kept like every value in milligrams, read as a
 *  person drinks it — a gram of water is a millilitre, so `1.5 l`, never `1,500 g`. */
const WATER: Step[] = [
  { unit: 'ml', per: 1000 },
  { unit: 'l', per: 1_000_000 },
];

function suffix(key: string): string {
  if (key === 'kcal' || key.endsWith('_kcal')) return 'kcal';
  const at = key.lastIndexOf('_');
  return at === -1 ? '' : key.slice(at + 1);
}

/** The step a value is shown in. `ref` (a target, a day's total) picks it, so `82 / 140 g` never
 *  mixes units; without one the value picks for itself. */
function stepFor(key: string, ref: number): Step | null {
  const ladder = key === 'water_mg' ? WATER : LADDERS[suffix(key)];
  if (!ladder) return null;
  let pick = ladder[0];
  for (const s of ladder) if (Math.abs(ref) >= s.per) pick = s;
  return pick;
}

/** A stored value as a person reads it. */
export function show(key: string, value: number, ref: number = value): Shown {
  const sfx = suffix(key);
  if (sfx === 'kcal') return { text: tidy(value), unit: 'kcal' };
  if (sfx === 's') {
    const m = Math.floor(value / 60);
    return { text: `${m}:${String(Math.round(value % 60)).padStart(2, '0')}`, unit: '' };
  }
  const step = stepFor(key, ref);
  if (!step) return { text: tidy(value), unit: '' };
  // Kilograms keep one decimal: a body weight is read to the tenth, and so is a plate.
  if (step.unit === 'kg') {
    const kg = Math.round((value / step.per) * 10) / 10;
    return { text: Number.isInteger(kg) ? group(kg) : kg.toFixed(1), unit: 'kg' };
  }
  return { text: tidy(value / step.per), unit: step.unit };
}

/** `82 g`, `1,240 kcal`. */
export function showWith(key: string, value: number, ref: number = value): string {
  const s = show(key, value, ref);
  return s.unit ? `${s.text} ${s.unit}` : s.text;
}

/** The unit a person types a value in, and how many stored units one of it is: grams for a `_mg`
 *  key, because nobody types protein in milligrams. */
export function typed(key: string): Step {
  const sfx = suffix(key);
  if (sfx === 'kcal') return { unit: 'kcal', per: 1 };
  if (sfx === 'mg') return { unit: 'g', per: 1000 };
  if (sfx === 'ug') return { unit: 'mg', per: 1000 };
  // Nanograms are typed in micrograms, as a label prints vitamin B12 and D.
  if (sfx === 'ng') return { unit: 'µg', per: 1000 };
  // Grams are typed as kilograms: what is kept in grams and typed by hand is a weight.
  if (sfx === 'g') return { unit: 'kg', per: 1000 };
  return { unit: '', per: 1 };
}

/** A number read from a property, or `null` when it is absent or not a whole number. */
export function whole(v: unknown): number | null {
  const n = typeof v === 'number' ? v : typeof v === 'string' && v.trim() !== '' ? Number(v) : NaN;
  return Number.isInteger(n) ? n : null;
}

// ── days ─────────────────────────────────────────────────────────────────────────────────────────
// An entry is dated by day, never by time, and the day is the person's own: the app clock's
// (`appTime.ts`, 2026-10-24), never the browser's or UTC's.

/** Today, on the app's clock. */
export const today = () => appToday();
/** The time it is, as an entry's moment carries it: `HH:MM`, on the app's clock. */
export function nowClock(): string {
  return clockText();
}
/** The time an entry's moment carries (`2026-10-12T13:05` → `13:05`), or nothing. */
export const clockOf = (start: string | null | undefined): string =>
  start && start.length > 10 ? start.slice(11, 16) : '';
/** The day of an entry's moment. */
export const dayOf = (start: string | null | undefined): string => (start ?? '').slice(0, 10);
export const addDays = civilAddDays;
/** Every day from `from` to `to`, both included. */
export function daysBetween(from: string, to: string): string[] {
  const out: string[] = [];
  for (let d = from; d <= to && out.length < 400; d = addDays(d, 1)) out.push(d);
  return out;
}
/** `Sun 11 Oct`. */
export function dayLabel(day: string): string {
  return formatCivil(day, { weekday: 'short', day: 'numeric', month: 'short' });
}
/** One letter for a strip of days. */
export function dayLetter(day: string): string {
  return formatCivil(day, { weekday: 'narrow' });
}
export const dayNumber = (day: string) => Number(day.slice(8, 10));
