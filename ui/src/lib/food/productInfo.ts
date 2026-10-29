// A scanned product in English as well as in the words it was typed in (`product.rs`, 2026-10-29).
// Open Food Facts keeps a product's name in the language its first contributor used, so a tin bought
// in Italy can come back in German. The form's name says what it is in English first, and the lines
// under the look-up say where that English came from, where it is sold and where its barcode was
// issued — which is not where it was made.
import type { ProductFound } from '../types';

/** The name the form is filled with: the English first when it is the product's own English name or
 *  its category, then the name as typed. The food list's nearest food is only offered, never put in
 *  the name: it is a kind of food, not this product. */
export function nameFor(found: ProductFound): string {
  const en = found.english;
  return en && en.from !== 'list' ? `${en.name} — ${found.name}` : found.name;
}

/** The ingredients as typed, with the English under them when Open Food Facts has it. */
export function ingredientsFor(found: ProductFound): string {
  const en = found.ingredients_en?.trim();
  return en ? `${found.ingredients}\n\nIn English: ${en}` : found.ingredients;
}

/** What to say under the look-up, a line each. */
export function aboutLines(found: ProductFound): string[] {
  const lines: string[] = [];
  const en = found.english;
  if (en?.from === 'name') lines.push(`In English: ${en.name}.`);
  else if (en?.from === 'category')
    lines.push(`In English: ${en.name} (its category at Open Food Facts).`);
  else if (en?.from === 'list')
    lines.push(`In English, the nearest food in the food list is ${en.name}.`);
  else lines.push('Open Food Facts has no English name for it.');
  const sold = found.sold_in;
  if (sold.length)
    lines.push(
      `Sold in ${sold.length > 4 ? `${sold.slice(0, 4).join(', ')} and ${sold.length - 4} more countries` : list(sold)}.`,
    );
  if (found.registered)
    lines.push(
      `Its barcode was issued in ${found.registered} — where the brand got its number, not where it was made.`,
    );
  return lines;
}

const list = (xs: string[]) =>
  xs.length < 2 ? xs.join('') : `${xs.slice(0, -1).join(', ')} and ${xs[xs.length - 1]}`;
