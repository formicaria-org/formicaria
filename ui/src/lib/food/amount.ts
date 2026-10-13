// How much of a food an entry is, and in what: grams, or millilitres for a food whose label is
// by volume (`decisions.md#data`, 2026-10-13). **A millilitre is never turned into a gram**: no
// density is assumed anywhere, so the two are shown, changed and added up apart.
//
// Everything that shows or adds an amount asks here which one an entry has. A place that reads
// `amount_g` alone shows a drink as having no amount at all.
import { showWith, whole } from '../units';

export type AmountUnit = 'g' | 'ml';

/** The property an amount in this unit is kept under. */
export const amountKey = (unit: AmountUnit) => (unit === 'ml' ? 'amount_ml' : 'amount_g');
/** Both, for whatever must leave the amount out of a sum of values. */
export const AMOUNT_KEYS = ['amount_g', 'amount_ml'];

/** The amount an entry carries, and its unit. Grams when it has none. */
export function entryAmount(props: Record<string, unknown>): {
  n: number | null;
  unit: AmountUnit;
} {
  const ml = whole(props.amount_ml);
  if (ml !== null) return { n: ml, unit: 'ml' };
  return { n: whole(props.amount_g), unit: 'g' };
}

/** `180 g`, `250 ml`, `1.5 l`. */
export const shownAs = (n: number, unit: AmountUnit) => showWith(amountKey(unit), n);

/** An entry's amount as a person reads it, or nothing when it has none. */
export function shownAmount(props: Record<string, unknown>): string {
  const a = entryAmount(props);
  return a.n === null ? '' : shownAs(a.n, a.unit);
}
