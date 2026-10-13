import { expect, test } from 'vitest';
import { per100, typedFrom, unitOf, type LabelForm } from './label';

const form = (over: Partial<LabelForm>): LabelForm => ({
  basis: '100g',
  serving: null,
  servingUnit: 'g',
  energyIn: 'kcal',
  sodiumAs: 'salt',
  values: {},
  ...over,
});

test('a European label is kept as it is printed', () => {
  // Penne: per 100 g, 359 kcal, 1.5 g fat, 0.01 g salt.
  const kept = per100(form({ values: { kcal: 359, fat_mg: 1.5, sodium_ug: 0.01 } }));
  expect(kept).toEqual({ kcal: 359, fat_mg: 1500, sodium_ug: 4000 });
});

test('an American label is per serving, with sodium in milligrams', () => {
  // A 30 g serving: 150 kcal, 8 g fat, 170 mg sodium.
  const f = form({
    basis: 'serving',
    serving: 30,
    energyIn: 'kcal',
    sodiumAs: 'sodium',
    values: { kcal: 150, fat_mg: 8, sodium_ug: 170 },
  });
  expect(per100(f)).toEqual({ kcal: 500, fat_mg: 26_667, sodium_ug: 566_667 });
  expect(unitOf(f)).toBe('g');
});

test('figures per serving cannot be kept until the serving has a size', () => {
  expect(per100(form({ basis: 'serving', values: { kcal: 150 } }))).toBeNull();
});

test('energy given only in kilojoules is turned into kilocalories', () => {
  // 1,700 kJ ÷ 4.184 = 406.3
  expect(per100(form({ energyIn: 'kJ', values: { kcal: 1700 } }))).toEqual({ kcal: 406 });
});

test('a drink is per 100 ml and is measured in millilitres', () => {
  const f = form({ basis: '100ml', values: { kcal: 42, sugars_mg: 10.6 } });
  expect(per100(f)).toEqual({ kcal: 42, sugars_mg: 10_600 });
  expect(unitOf(f)).toBe('ml');
  // A serving in millilitres says the same of a label that is per serving.
  expect(unitOf(form({ basis: 'serving', servingUnit: 'ml', serving: 250 }))).toBe('ml');
});

test('salt and sodium are the same amount, said two ways', () => {
  const salt = per100(form({ sodiumAs: 'salt', values: { sodium_ug: 1 } }));
  const sodium = per100(form({ sodiumAs: 'sodium', values: { sodium_ug: 400 } }));
  expect(salt).toEqual(sodium);
});

test('what was not typed is not kept, and nothing below zero', () => {
  expect(per100(form({ values: { kcal: '', fat_mg: -1, protein_mg: '7.5' } }))).toEqual({
    protein_mg: 7500,
  });
});

test('a product looked up fills the form as a European label reads', () => {
  expect(typedFrom({ kcal: 539, fat_mg: 30_900, sodium_ug: 43_000 })).toEqual({
    kcal: 539,
    fat_mg: 30.9,
    sodium_ug: 0.11,
  });
});
