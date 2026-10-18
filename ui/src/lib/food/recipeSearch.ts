// **Every recipe in one search** (the owner, 2026-10-18: *"italian recipes are not a different thing
// than the others … they must be equally searchable and usable"*). The person's own recipes and the
// collections carried in the app are found by the same words, in one list, each row labelled with
// where it came from. Pure: no store, no network.
import type { CarriedRecipe, Recipe, RecipeLibrary } from '../types';

/** One collection carried in the app, with the short name its rows are labelled with. */
export interface Shelf {
  label: string;
  lib: RecipeLibrary;
}

export type RecipeRow =
  | { kind: 'own'; key: string; title: string; recipe: Recipe }
  | {
      kind: 'carried';
      key: string;
      title: string;
      label: string;
      lib: RecipeLibrary;
      recipe: CarriedRecipe;
    };

/** Lower case, accents gone: "Tiramisù" is found by "tiramisu", "Città" by "citta". */
export const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** Every word typed must appear somewhere in what is known of the recipe. */
const matches = (words: string[], text: string) => {
  const hay = fold(text);
  return words.every((w) => hay.includes(w));
};

/** The recipes the words name — the person's own first, then each carried collection's — or every
 *  recipe when nothing is typed. Matched on the name, the ingredients and, where known, the region. */
export function searchRecipes(query: string, own: Recipe[], shelves: Shelf[]): RecipeRow[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  const rows: RecipeRow[] = [];
  for (const r of own) {
    const text = [r.title, ...r.ingredients.map((i) => i.title ?? '')].join('\n');
    if (matches(words, text))
      rows.push({ kind: 'own', key: `own/${r.id}`, title: r.title, recipe: r });
  }
  for (const s of shelves)
    for (const r of s.lib.recipes) {
      const text = [r.title, r.region ?? '', ...(r.ingredients ?? [])].join('\n');
      if (matches(words, text))
        rows.push({
          kind: 'carried',
          key: `${s.label}/${r.id}`,
          title: r.title,
          label: s.label,
          lib: s.lib,
          recipe: r,
        });
    }
  return rows;
}
