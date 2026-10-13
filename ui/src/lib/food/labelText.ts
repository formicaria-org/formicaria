// The figures of a nutrition label, read out of a transcription of it
// (`decisions.md#agent`, 2026-10-13, *a label is read from a photograph, and the person checks
// it*). The assistant turns the picture into text and decides no number; this finds the column
// that is per 100 g or 100 ml, the lines it knows, and each figure with its unit, and leaves
// empty whatever it cannot find. Nothing is worked out, converted between columns or guessed.
//
// The text is a Markdown table when the reader made one, and plain lines when it did not; both
// are read. Labels are read in English, Italian, French, German, Spanish and Portuguese.
import { KJ_PER_KCAL, type Basis, type LabelForm, type Unit } from './label';

export type LabelReading = {
  /** What was found, in the form's own terms; a key that was not found is absent. */
  form: Partial<Omit<LabelForm, 'values'>> & { values: Record<string, number> };
  /** How many figures were found. Zero means the text held no label this could read. */
  found: number;
  /** What does not add up, in words for the person: reasons to look at the packet again. */
  doubts: string[];
};

type Key =
  | 'energy'
  | 'fat_mg'
  | 'saturated_fat_mg'
  | 'carbs_mg'
  | 'sugars_mg'
  | 'fiber_mg'
  | 'protein_mg'
  | 'salt'
  | 'sodium';

/** Lower case, accents off, so one spelling of each word serves. */
const plain = (s: string) =>
  s.normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/ß/g, 'ss').toLowerCase();

// The order is the decision: "saturated fat" names fat, and "of which sugars" sits under
// carbohydrate, so the narrower line is tried first.
const NAMES: [Key, RegExp][] = [
  ['saturated_fat_mg', /\b(saturate[sd]?|saturi|satures|gesattigte|saturad[ao]s)\b/],
  ['sugars_mg', /\b(sugars?|zuccheri|sucres?|zucker|azucares|acucares)\b/],
  ['fiber_mg', /\b(fib(re|er|ra)s?|ballaststoffe)\b/],
  ['energy', /\b(energy|energia|energie|brennwert|energetico|energetique|calories?|calorie)\b/],
  ['fat_mg', /\b(fats?|grassi|grasses|lipides|lipidos|fett|grasas|gorduras)\b/],
  ['carbs_mg', /\b(carbohydrates?|carboidrati|glucides|kohlenhydrate|hidratos|carboidratos)\b/],
  ['protein_mg', /\b(proteins?|proteine|proteines|eiweiss|proteinas)\b/],
  ['sodium', /\b(sodium|sodio|natrium)\b/],
  ['salt', /\b(salt|sale|sel|salz|sal)\b/],
];
/** Lines that name a nutrient above but are a different figure: trans fat, added sugars. */
const NOT = /\b(trans|added|adicionados|aggiunti|ajoutes|anadidos|zugesetzte)\b/;

type Amount = { n: number; unit: string };
/** Every figure in `s`, each with the unit printed after it, or before it as some labels have
 *  it ("g 47", "kcal 492"). A percentage is not a figure. */
function amounts(s: string): Amount[] {
  const out: Amount[] = [];
  const re = /(?:\b(kj|kcal|mg|g)\s*)?(\d+(?:[.,]\d+)?)\s*(kj|kcal|mg|µg|mcg|ml|g\b|%)?/gi;
  for (const m of s.matchAll(re)) {
    const unit = (m[3] ?? m[1] ?? '').toLowerCase();
    if (unit === '%') continue;
    out.push({ n: Number(m[2].replace(',', '.')), unit });
  }
  return out;
}

type Size = { n: number; unit: Unit };
/** What a heading says its figures are for: per 100 of something, a serving of a size, or both
 *  when a reader put two headings in one cell. */
type Heading = { hundred: Basis | null; serving: Size | null };
const SERVING = /\b(per|pro|par|por|je|serving|portion|porzione|porcao|porcion|racion)\b/;
function heading(cell: string, bare: boolean): Heading {
  const c = plain(cell).replace(/\s+/g, ' ');
  const after = c.match(/\b100 ?(g|ml)\b/);
  const before = after ? null : c.match(/\b(g|ml) ?100\b/);
  const hundred = after ?? before;
  const rest = hundred ? c.replace(hundred[0], ' ') : c;
  // A bare size is a heading only in a table's own heading row; in a loose line it could be
  // the weight of the whole packet.
  const size = bare || SERVING.test(rest) ? rest.match(/(\d+(?:[.,]\d+)?) ?(g|ml)\b/) : null;
  const n = size ? Number(size[1].replace(',', '.')) : 0;
  return {
    hundred: hundred ? (hundred[1] === 'ml' ? '100ml' : '100g') : null,
    serving: size && n > 0 ? { n, unit: size[2] as Unit } : null,
  };
}

/** A line as rows of cells: a table row split at its bars, or the whole line as one cell. A
 *  reader that put two printed lines in one row, parted by a line break, gets them back as two. */
function cells(line: string): string[][] {
  if (!line.includes('|')) return [[line]];
  const parts = line.split('|').map((c) => c.trim());
  if (parts[0] === '') parts.shift();
  if (parts.at(-1) === '') parts.pop();
  const stacked = parts.map((c) => c.split(/<br\s*\/?>/i).map((x) => x.trim()));
  const deep = Math.max(...stacked.map((c) => c.length));
  if (deep === 1 || stacked[0].length !== deep)
    return [parts.map((c) => c.replace(/<br\s*\/?>/gi, ' '))];
  return Array.from({ length: deep }, (_, i) => stacked.map((c) => c[i] ?? ''));
}

const grams = (a: Amount) => (a.unit === 'mg' ? a.n / 1000 : a.n);
const round = (n: number, places: number) => Math.round(n * 10 ** places) / 10 ** places;

export function readLabel(text: string): LabelReading {
  const rows = text
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l && !/^\|?[\s:|-]+\|?$/.test(l))
    .flatMap(cells);

  // Which column to read: the one per 100 g or 100 ml when there is one, since that is what a
  // food is kept as; otherwise a serving whose size is printed.
  let at = -1;
  let basis: Basis | null = null;
  let serving: Size | null = null;
  for (const row of rows) {
    // A line of figures is not a heading: "Fat | 16.0g" would otherwise read as a 16 g serving.
    if (NAMES.some(([, re]) => re.test(plain(row[0])))) continue;
    const heads = row.map((c) => heading(c, row.length > 1));
    const hundred = heads.findIndex((h) => h.hundred);
    const size = heads.findIndex((h) => h.serving);
    if (hundred >= 0) {
      at = row.length > 1 ? hundred : -1;
      basis = heads[hundred].hundred;
      serving = size >= 0 ? heads[size].serving : null;
      break;
    }
    if (size >= 0 && !serving) {
      at = row.length > 1 ? size : -1;
      serving = heads[size].serving;
    }
  }

  const got: Partial<Record<Key, Amount[]>> = {};
  let last: Key | null = null;
  for (const row of rows) {
    // A table row's name is its first cell; a loose line's is what stands before its figures.
    const digit = row[0].search(/\d/);
    const loose = row.length === 1;
    const label = plain(loose && digit >= 0 ? row[0].slice(0, digit) : row[0]);
    let key = NOT.test(label) ? null : (NAMES.find(([, re]) => re.test(label))?.[0] ?? null);
    // Energy printed on two lines, kJ then kcal, the second with no name of its own.
    if (!key && last === 'energy' && !/[a-z]{3,}/.test(label)) key = 'energy';
    last = key;
    if (!key) continue;
    // A unit given once in the line's name, "Protein (g)", stands for its bare figures.
    // "Calories", on an American label, is kilocalories and is printed with no unit.
    const named = label.match(/\((kcal|kj|g|mg)\)/)?.[1] ?? (/\bcalorie/.test(label) ? 'kcal' : '');
    const where = loose ? (digit >= 0 ? row[0].slice(digit) : '') : (row[at >= 0 ? at : 1] ?? '');
    let found = amounts(where).map((a) => ({ ...a, unit: a.unit || named }));
    // A loose line prints its columns one after another; the first is the one the heading named.
    if (loose && key !== 'energy') found = found.slice(0, 1);
    if (!found.length) continue;
    if (key === 'energy') got.energy = [...(got.energy ?? []), ...found];
    else got[key] ??= found;
  }

  const values: Record<string, number> = {};
  const form: LabelReading['form'] = { values };
  const doubts: string[] = [];

  const energy = got.energy ?? [];
  const kcal = energy.find((a) => a.unit === 'kcal');
  const kj = energy.find((a) => a.unit === 'kj');
  if (kcal) ((values.kcal = kcal.n), (form.energyIn = 'kcal'));
  else if (kj) ((values.kcal = kj.n), (form.energyIn = 'kJ'));
  if (kcal && kj && Math.abs(kj.n / KJ_PER_KCAL - kcal.n) > Math.max(3, kcal.n * 0.05))
    doubts.push('The kJ and kcal figures do not agree with each other.');

  for (const key of [
    'fat_mg',
    'saturated_fat_mg',
    'carbs_mg',
    'sugars_mg',
    'fiber_mg',
    'protein_mg',
  ] as const) {
    const a = got[key]?.[0];
    if (a && (a.unit === 'g' || a.unit === 'mg' || a.unit === '')) values[key] = round(grams(a), 2);
  }
  const salt = got.salt?.[0];
  const sodium = got.sodium?.[0];
  if (salt) ((values.sodium_ug = round(grams(salt), 3)), (form.sodiumAs = 'salt'));
  else if (sodium) {
    values.sodium_ug = round(sodium.unit === 'g' ? sodium.n * 1000 : sodium.n, 1);
    form.sodiumAs = 'sodium';
  }

  const found = Object.keys(values).length;
  if (found && basis) {
    form.basis = basis;
    // A serving is kept in whole grams or millilitres; one printed as 21.5 g is left out here,
    // where it is only a convenience, sooner than be written down as 22.
    if (serving && serving.unit === (basis === '100ml' ? 'ml' : 'g') && Number.isInteger(serving.n))
      form.serving = serving.n;
  } else if (found && serving) {
    form.basis = 'serving';
    form.serving = Math.round(serving.n);
    form.servingUnit = serving.unit;
    if (!Number.isInteger(serving.n))
      doubts.push(
        `A serving is printed as ${serving.n} ${serving.unit} and kept as a whole number.`,
      );
  } else if (found)
    doubts.push('It does not say what the figures are for: choose per 100 g, 100 ml or serving.');

  const v = values;
  if (v.saturated_fat_mg > v.fat_mg) doubts.push('Saturates are more than the fat.');
  if (v.sugars_mg > v.carbs_mg) doubts.push('Sugars are more than the carbohydrate.');
  if (form.energyIn === 'kcal' && [v.fat_mg, v.carbs_mg, v.protein_mg].every((n) => n >= 0)) {
    // Atwater's factors, as a label's own energy is worked out. Alcohol and polyols are not on
    // the lines read here, so a drink or a sugar-free sweet can fail this honestly.
    const from = 9 * v.fat_mg + 4 * v.carbs_mg + 4 * v.protein_mg + 2 * (v.fiber_mg ?? 0);
    if (Math.abs(from - v.kcal) > Math.max(25, v.kcal * 0.2))
      doubts.push('The energy does not match the fat, carbohydrate and protein.');
  }
  if (
    form.basis === '100g' &&
    (v.fat_mg ?? 0) + (v.carbs_mg ?? 0) + (v.protein_mg ?? 0) + (v.fiber_mg ?? 0) > 101
  )
    doubts.push('The figures add up to more than 100 g in 100 g.');

  return { form, found, doubts };
}
