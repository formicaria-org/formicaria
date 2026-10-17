// A recipe's own note, as text — `decisions.md` 2026-10-18, *a recipe is one note, reached from the
// notes and from the food view*. Everything a recipe is besides what is counted lives in its text:
// the ingredients as written, the method, the times, the notes, where it came from. What is counted
// is the ingredients matched to foods, kept apart, each remembering the line it was matched from.
//
// Pure: no store, no network. The text is written once, when a recipe is made from a page or a
// carried recipe, and read back for its ingredient lines whenever its screen opens — so lines added
// or edited by hand in the note are matched the same way.

/** What a recipe can come with, from wherever it came. Every field the source's own text. */
export interface RecipeSource {
  ingredients?: string[];
  steps?: string[];
  /** Paragraphs, when the source writes its method as prose. */
  text?: string[];
  notes?: string[];
  prepMinutes?: number;
  cookMinutes?: number;
  totalMinutes?: number;
  /** Time fields in the source's own words, when it gives words rather than numbers. */
  times?: Record<string, string>;
  /** "4 persone", as the source says it. */
  yieldText?: string;
  /** Where it came from: the page, and how it is named and credited. */
  url?: string;
  credit?: string;
  image?: string;
}

const minutes = (n: number) =>
  n >= 60 ? `${Math.floor(n / 60)} h${n % 60 ? ` ${n % 60} min` : ''}` : `${n} min`;

/** The note's text for a recipe from a source: Ingredients, Method, Times, Notes, then the credit. */
export function recipeBody(s: RecipeSource): string {
  const out: string[] = [];
  out.push('## Ingredients', '');
  for (const line of s.ingredients ?? []) out.push(`- ${line}`);
  if (!(s.ingredients ?? []).length) out.push('- ');
  out.push('', '## Method', '');
  if (s.steps?.length) s.steps.forEach((step, i) => out.push(`${i + 1}. ${step}`));
  else if (s.text?.length) out.push(s.text.join('\n\n'));
  else out.push('1. ');
  const times: string[] = [];
  if (s.prepMinutes) times.push(`Preparation: ${minutes(s.prepMinutes)}`);
  if (s.cookMinutes) times.push(`Cooking: ${minutes(s.cookMinutes)}`);
  if (s.totalMinutes && !(s.prepMinutes && s.cookMinutes))
    times.push(`In all: ${minutes(s.totalMinutes)}`);
  for (const [k, v] of Object.entries(s.times ?? {})) times.push(`${k}: ${v}`);
  if (s.yieldText) times.push(`Makes: ${s.yieldText}`);
  out.push('', '## Times', '', times.join(' · '));
  out.push('', '## Notes', '');
  for (const n of s.notes ?? []) out.push(n, '');
  const from: string[] = [];
  if (s.url) from.push(`From [${s.credit ?? s.url}](${s.url}).`);
  else if (s.credit) from.push(`From ${s.credit}.`);
  // A link, never the picture itself: it is the site's, and the app fetches nothing to show it.
  if (s.image) from.push(`[The source's photo](${s.image})`);
  if (from.length) out.push('', '---', '', from.join(' '));
  return out.join('\n').replace(/\n{3,}/g, '\n\n') + '\n';
}

/** The lines of the note's **Ingredients** list (or *Ingredienti*), as written — what is matched. */
export function ingredientLines(body: string): string[] {
  const lines = body.split('\n');
  const out: string[] = [];
  let inside = false;
  for (const raw of lines) {
    const line = raw.trim();
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    if (heading) {
      inside = /^(ingredients|ingredienti)\b/i.test(heading[1].trim());
      continue;
    }
    if (!inside) continue;
    const item = /^(?:[-*+]|\d+[.)])\s+(.*)$/.exec(line);
    if (item && item[1].trim()) out.push(item[1].trim());
  }
  return out;
}

const UNITS =
  /^(g|gr|grammi|grammo|kg|chilo|chili|mg|ml|cl|dl|l|litro|litri|cucchiai[oa]?|cucchiaini?|cucchiaio|tazz[ae]|bicchier[ei]|spicchi[o]?|pizzico|pizzichi|foglie|foglia|rametto|rametti|fett[ae]|n\.?|pz|pezzi|q\.?b\.?|qb|cup|cups|tbsp|tsp|oz|lb)$/i;

/** A guess at the food a line names and its weight in grams, to start the food search with.
 *  Only a guess: the person picks the food and sees the amount before anything is added. */
export function guessFood(line: string): { name: string; grams: number | null } {
  const text = line
    .replace(/\(.*?\)/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  let grams: number | null = null;
  const g = /(\d+(?:[.,]\d+)?)\s*(kg|chili|chilo|g|gr|grammi|grammo)\b/i.exec(text);
  if (g) {
    const n = Number(g[1].replace(',', '.'));
    grams = /^k|^chil/i.test(g[2]) ? Math.round(n * 1000) : Math.round(n);
  } else {
    const ofG = /\b(?:grammi|grammo|gr)\.?\s*(\d+)/i.exec(text); // "Riso, grammi 500"
    if (ofG) grams = Number(ofG[1]);
  }
  // The name: what is left once amounts, units and the joining words are taken away.
  const head = text.split(/[,;:]/)[0];
  const tokens = head.split(' ');
  // "00" or "0" names a flour, never a count, so it is kept as part of the name.
  const isNumber = (w: string) => /^[\d.,/½¼¾⅓⅔-]+$/.test(w) && !/^0\d*$/.test(w);
  const isUnit = (w: string) => UNITS.test(w.replace(/[.,]$/, ''));
  // A number is an amount when it starts the line or a unit follows it; "farina 00" keeps its 00.
  const words = tokens.filter(
    (w, i) =>
      !isUnit(w) &&
      !(isNumber(w) && (i === 0 || isUnit(tokens[i + 1] ?? '') || i === tokens.length - 1)),
  );
  while (words.length && /^(di|d'|del|della|dello|dei|degli|delle|of|a|an)$/i.test(words[0]))
    words.shift();
  const name = words
    .join(' ')
    .replace(/^d['’]/i, '')
    .trim();
  return { name: name || head.trim(), grams: grams && grams > 0 ? grams : null };
}
