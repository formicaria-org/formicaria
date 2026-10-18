import { expect, test } from 'vitest';
import { searchRecipes, type Shelf } from './recipeSearch';
import type { Recipe, RecipeLibrary } from '../types';

const lib = (source: string, recipes: RecipeLibrary['recipes']): RecipeLibrary => ({
  key: source.toLowerCase(),
  label: source,
  source,
  home: '',
  seen: '',
  recipes,
});
const shelves: Shelf[] = [
  {
    label: 'NHLBI',
    lib: lib('NHLBI', [
      { id: 'chili', title: 'Turkey Chili', ingredients: ['1 lb ground turkey'] },
    ]),
  },
  {
    label: 'Wikibooks',
    lib: lib('Wikibooks', [
      {
        id: 'tira',
        title: 'Tiramisù',
        ingredients: ['500 g di mascarpone', '5 uova'],
        region: 'Veneto',
      },
      {
        id: 'pesto',
        title: 'Pesto alla genovese',
        ingredients: ['basilico', 'pinoli'],
        region: 'Liguria',
      },
    ]),
  },
];
const mine = [
  {
    id: 'n1',
    recipe: 'w1',
    title: 'Tiramisù della nonna',
    servings: 6,
    of: 1,
    sums: {},
    ingredients: [{ title: 'Mascarpone' }],
  },
] as unknown as Recipe[];

test('one search finds a recipe whatever it came from, with the person’s own first', () => {
  const rows = searchRecipes('tiramisu', mine, shelves);
  expect(rows.map((r) => [r.kind, r.title])).toEqual([
    ['own', 'Tiramisù della nonna'],
    ['carried', 'Tiramisù'],
  ]);
  expect(rows[1].kind === 'carried' && rows[1].label).toBe('Wikibooks');
});

test('it finds by ingredient and by region, every word typed counting', () => {
  expect(searchRecipes('mascarpone', [], shelves).map((r) => r.title)).toEqual(['Tiramisù']);
  expect(searchRecipes('liguria', [], shelves).map((r) => r.title)).toEqual([
    'Pesto alla genovese',
  ]);
  expect(searchRecipes('turkey chili', [], shelves).map((r) => r.title)).toEqual(['Turkey Chili']);
  expect(searchRecipes('turkey pesto', [], shelves)).toEqual([]);
});

test('nothing typed is every recipe, in the same order', () => {
  expect(searchRecipes('  ', mine, shelves)).toHaveLength(4);
});
