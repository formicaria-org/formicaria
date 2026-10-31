// How much of a food an entry is, and in what: grams, millilitres for a food whose label is by
// volume (`decisions.md#data`, 2026-10-13), or a count of pieces — tablets, capsules, drops — for a
// supplement whose label is per piece (2026-10-31). **A millilitre is never turned into a gram**,
// nor a tablet into either: no density or weight is assumed anywhere, so they are shown, changed and
// added up apart.
//
// Everything that shows or adds an amount asks here which one an entry has. A place that reads
// `amount_g` alone shows a drink as having no amount at all.
import { showWith, whole } from '../units';

export type AmountUnit = 'g' | 'ml' | 'each';

/** What one piece of a supplement can be, as its label says it: singular, and `s` makes plural. */
export const PIECES = ['tablet', 'capsule', 'softgel', 'drop', 'sachet', 'scoop'] as const;

/** The property an amount in this unit is kept under. */
export const amountKey = (unit: AmountUnit) =>
  unit === 'ml' ? 'amount_ml' : unit === 'each' ? 'amount_each' : 'amount_g';
/** All of them, for whatever must leave the amount out of a sum of values. */
export const AMOUNT_KEYS = ['amount_g', 'amount_ml', 'amount_each'];

/** The amount an entry carries, and its unit. Grams when it has none. */
export function entryAmount(props: Record<string, unknown>): {
  n: number | null;
  unit: AmountUnit;
} {
  const ml = whole(props.amount_ml);
  if (ml !== null) return { n: ml, unit: 'ml' };
  const each = whole(props.amount_each);
  if (each !== null) return { n: each, unit: 'each' };
  return { n: whole(props.amount_g), unit: 'g' };
}

/** `2 tablets`, `1 capsule`; a piece with no name is `2 ×`. */
export const pieces = (n: number, piece = '') =>
  piece ? `${n} ${piece}${n === 1 ? '' : 's'}` : `${n} ×`;

/** `180 g`, `250 ml`, `1.5 l`, `2 tablets`. */
export const shownAs = (n: number, unit: AmountUnit, piece = '') =>
  unit === 'each' ? pieces(n, piece) : showWith(amountKey(unit), n);

/** An entry's amount as a person reads it, or nothing when it has none. */
export function shownAmount(props: Record<string, unknown>): string {
  const a = entryAmount(props);
  const piece = typeof props.piece === 'string' ? props.piece : '';
  return a.n === null ? '' : shownAs(a.n, a.unit, piece);
}
