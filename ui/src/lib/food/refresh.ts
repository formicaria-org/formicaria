// **An entry is a snapshot; the food list improves.** (2026-10-29.) A food logged before the list
// had a value for it keeps logging without it: a person ate eggs after the update, and choline
// still read 0, because "＋ again" copied the old entry's numbers and the entries already written
// were never told. So an entry from the list can be brought up to date — on a press, never by
// itself — and "＋ again" reads the food from the list as it is now. Values the entry already has
// stay as they are: they are what was logged.
import type { FoodDetail } from '../types';
import { entryAmount } from './amount';
import { borrowedOf, borrowedProp, nutrientName, tableOf, TABLES, valueOf } from './provenance';

/** A key that holds a nutrient amount (`kcal`, `protein_mg`, `vitamin_d_ng` …), not the amount
 *  eaten or anything else an entry keeps. */
export const isNutrientKey = (k: string) =>
  k === 'kcal' || (/_(mg|ug|ng)$/.test(k) && !k.startsWith('amount_'));

/** Whether an entry's food is one of the food list's (`foods@<version>/<table>:<code>`). */
export const fromList = (props: Record<string, unknown>) =>
  typeof props.source === 'string' && /^foods@[^/]+\/[a-z]+:/.test(props.source);

/** The values the list has now for an entry's food that the entry lacks, at the amount it was
 *  logged at, and the `borrowed` property to go with them. `null` when there is nothing to add, or
 *  the entry is not in grams (the list is per 100 g, and no density is ever assumed). */
export function freshValues(
  props: Record<string, unknown>,
  food: FoodDetail,
): Record<string, string | number> | null {
  const { n, unit } = entryAmount(props);
  if (unit !== 'g' || !n) return null;
  const add: Record<string, number> = {};
  for (const [k, per100] of Object.entries(food.per100)) {
    if (!isNutrientKey(k) || valueOf(props, k) !== null) continue;
    add[k] = Math.round((per100 * n) / 100);
  }
  if (!Object.keys(add).length) return null;
  const lent = { ...borrowedOf(props), ...(food.borrowed ?? {}) };
  const keys = [...Object.keys(props).filter(isNutrientKey), ...Object.keys(add)];
  const borrowed = borrowedProp(lent, keys);
  return { ...add, ...(borrowed ? { borrowed } : {}) };
}

/** An entry logged again: its own properties without its old numbers, and the food's values from
 *  the list now, at the same amount. */
export function againFromList(
  props: Record<string, string | number>,
  food: FoodDetail,
): Record<string, string | number> {
  const { n } = entryAmount(props);
  const kept = Object.fromEntries(
    Object.entries(props).filter(([k]) => !isNutrientKey(k) && k !== 'borrowed'),
  );
  if (!n) return props;
  const values = Object.fromEntries(
    Object.entries(food.per100).map(([k, v]) => [k, Math.round((v * n) / 100)]),
  );
  const borrowed = borrowedProp(food.borrowed ?? {}, Object.keys(values));
  return { ...kept, ...values, ...(borrowed ? { borrowed } : {}) };
}

/** Why an entry has no value for `key`, in a few words — the food list's food `food` when the entry
 *  is from the list and it could be read, `undefined` while that is not known. */
export function whyNot(
  props: Record<string, unknown>,
  key: string,
  food: FoodDetail | null | undefined,
): string {
  const src = typeof props.source === 'string' ? props.source : '';
  const what = nutrientName(key);
  if (!src) return 'typed in by hand, without it';
  if (src.startsWith('product/')) return `its label does not state ${what}`;
  if (src.startsWith('recipe/')) return `its ingredients give no ${what}`;
  if (!fromList(props)) return `its source gives no ${what}`;
  if (food === undefined) return '';
  if (food && food.per100[key] !== undefined)
    return `logged before the food list had ${what} for it`;
  const t = tableOf(src);
  const table = t ? TABLES[t] : 'its table';
  return `${table} has no ${what} for this food, and no close enough food in another table has`;
}
