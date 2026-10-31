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

test('a supplement is per piece, kept per 100 pieces, its vitamins in the unit printed', () => {
  // Vitamin D3 1,000 IU = 25 µg, 100 mg magnesium, 400 µg folic acid; no energy printed.
  const f = form({
    basis: 'each',
    piece: 'capsule',
    micros: [
      { key: 'vitamin_d_ng', unit: 'µg' },
      { key: 'magnesium_ug', unit: 'mg' },
      { key: 'folate_dfe_ug', unit: 'µg' },
    ],
    values: { vitamin_d_ng: 25, magnesium_ug: 100, folate_dfe_ug: 400 },
  });
  expect(unitOf(f)).toBe('each');
  // 25 µg is 25,000 ng a capsule; folic acid is 1.7 folate equivalents each.
  expect(per100(f)).toEqual({
    vitamin_d_ng: 2_500_000,
    magnesium_ug: 10_000_000,
    folate_dfe_ug: 68_000,
  });
  // Opened again, it reads per capsule, as the label prints it.
  const back = typedFrom(per100(f)!, 100);
  expect(back).toEqual({ vitamin_d_ng: 25, magnesium_ug: 100, folate_dfe_ug: 400 });
});

test('a vitamin typed in another unit than usual is kept the same', () => {
  const f = form({
    micros: [{ key: 'vitamin_c_ug', unit: 'µg' }],
    values: { vitamin_c_ug: 80_000 },
  });
  // 80,000 µg per 100 g is 80 mg per 100 g.
  expect(per100(f)).toEqual({ vitamin_c_ug: 80_000 });
  // A line taken off the form is not kept, whatever was typed in it.
  expect(per100(form({ values: { vitamin_c_ug: 80 } }))).toEqual({});
});
