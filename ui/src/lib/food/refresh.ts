// Why an entry has no value for a nutrient, in a few words (2026-10-29: under choline the sheet
// said only "No entry of this day carries it", and nothing about the eggs). An entry of the food list
// is read with the list's values as they are now (`packs::resolve` on the server), so when it lacks
// one, the list has none for that food.
import { nutrientName, tableOf, TABLES } from './provenance';

/** Whether an entry's food is one of the food list's (`foods@<version>/<table>:<code>`). */
export const fromList = (props: Record<string, unknown>) =>
  typeof props.source === 'string' && /^foods@[^/]+\/[a-z]+:/.test(props.source);

/** Why an entry has no value for `key`, by what it was logged from. */
export function whyNot(props: Record<string, unknown>, key: string): string {
  const src = typeof props.source === 'string' ? props.source : '';
  const what = nutrientName(key);
  if (!src) return 'typed in by hand, without it';
  if (src.startsWith('product/')) return `its label does not state ${what}`;
  if (src.startsWith('recipe/')) return `its ingredients give no ${what}`;
  if (!fromList(props)) return `its source gives no ${what}`;
  const t = tableOf(src);
  const table = t ? TABLES[t] : 'its table';
  return `${table} has no ${what} for this food, and no close enough food in another table has`;
}
