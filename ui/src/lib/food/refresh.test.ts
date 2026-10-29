// An entry is a snapshot; the food list improves (2026-10-29): eggs logged after the update still
// read no choline, because "＋ again" copied an older entry's numbers.
import { describe, expect, it } from 'vitest';
import type { FoodDetail } from '../types';
import { againFromList, freshValues, isNutrientKey, whyNot } from './refresh';

const egg: FoodDetail = {
  id: 'crea:181105',
  name: 'Chicken Egg, whole, boiled',
  src: 'crea',
  grp: '',
  per100: { kcal: 128, protein_mg: 12400, choline_ug: 268000, vitamin_b12_ng: 1100 },
  portions: [],
  pack: 'foods@2026.10.5',
  borrowed: {
    choline_ug: 'USDA: Egg, whole, boiled or poached',
    vitamin_b12_ng: 'BLS (Germany): Chicken egg boiled',
  },
};
const before = {
  source: 'foods@2026.10.3/crea:181105',
  amount_g: 120,
  kcal: 154,
  protein_mg: 14880,
  slot: 'breakfast',
};

describe('an entry and the food list now', () => {
  it('tells a nutrient amount from anything else an entry keeps', () => {
    expect(['kcal', 'choline_ug', 'vitamin_d_ng', 'protein_mg'].every(isNutrientKey)).toBe(true);
    expect(['amount_g', 'amount_ml', 'slot', 'source', 'borrowed'].some(isNutrientKey)).toBe(false);
  });

  it('adds only what the entry lacks, at its own amount, saying where it was borrowed from', () => {
    expect(freshValues(before, egg)).toEqual({
      choline_ug: 321600,
      vitamin_b12_ng: 1320,
      borrowed:
        'choline_ug=USDA: Egg, whole, boiled or poached; vitamin_b12_ng=BLS (Germany): Chicken egg boiled',
    });
    // What it has stays as logged; an old microgram key counts as having it.
    expect(freshValues({ ...before, choline_ug: 1, vitamin_b12_ug: 1 }, egg)).toBeNull();
    // Millilitres are never turned into grams.
    expect(freshValues({ ...before, amount_g: undefined, amount_ml: 200 }, egg)).toBeNull();
  });

  it('logs a food again with the values the list has now, keeping the amount and the meal', () => {
    expect(againFromList({ ...before, choline_ug: 0 }, egg)).toEqual({
      source: 'foods@2026.10.3/crea:181105',
      amount_g: 120,
      slot: 'breakfast',
      kcal: 154,
      protein_mg: 14880,
      choline_ug: 321600,
      vitamin_b12_ng: 1320,
      borrowed:
        'choline_ug=USDA: Egg, whole, boiled or poached; vitamin_b12_ng=BLS (Germany): Chicken egg boiled',
    });
  });

  it('says why an entry has no value, by what it was logged from', () => {
    expect(whyNot({}, 'choline_ug', undefined)).toBe('typed in by hand, without it');
    expect(whyNot({ source: 'product/01ABC' }, 'choline_ug', undefined)).toBe(
      'its label does not state choline',
    );
    expect(whyNot({ source: 'recipe/omelette' }, 'choline_ug', undefined)).toBe(
      'its ingredients give no choline',
    );
    expect(whyNot(before, 'choline_ug', undefined)).toBe('');
    expect(whyNot(before, 'choline_ug', egg)).toBe(
      'logged before the food list had choline for it',
    );
    expect(whyNot(before, 'iodine_ug', egg)).toBe(
      'CREA (Italy) has no iodine for this food, and no close enough food in another table has',
    );
  });
});
