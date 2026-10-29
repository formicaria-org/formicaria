// A scanned product in English as well (2026-10-29): a tin bought in Italy came back in German.
import { describe, expect, it } from 'vitest';
import type { ProductFound } from '../types';
import { aboutLines, ingredientsFor, nameFor } from './productInfo';

const tin: ProductFound = {
  name: 'Kichererbsen (Freshona)',
  english: { name: 'Chickpeas', from: 'category' },
  sold_in: ['Germany', 'Italy'],
  registered: 'Germany',
  ingredients: 'Kichererbsen, Wasser, Salz',
  ingredients_en: 'Chickpeas, water, salt',
  barcode: '4056489123457',
  per100: {},
  page: 'https://world.openfoodfacts.org/product/4056489123457',
};

describe('a scanned product in English', () => {
  it('names it in English first, then as typed', () => {
    expect(nameFor(tin)).toBe('Chickpeas — Kichererbsen (Freshona)');
    expect(nameFor({ ...tin, english: undefined })).toBe('Kichererbsen (Freshona)');
    // The food list's nearest food is a kind of food, not this product: offered, not used.
    expect(nameFor({ ...tin, english: { name: 'Chickpea, dried', from: 'list' } })).toBe(
      'Kichererbsen (Freshona)',
    );
  });

  it('keeps the ingredients as typed, with the English under them', () => {
    expect(ingredientsFor(tin)).toBe(
      'Kichererbsen, Wasser, Salz\n\nIn English: Chickpeas, water, salt',
    );
    expect(ingredientsFor({ ...tin, ingredients_en: '' })).toBe('Kichererbsen, Wasser, Salz');
  });

  it('says where the English came from, where it is sold and where its barcode was issued', () => {
    expect(aboutLines(tin)).toEqual([
      'In English: Chickpeas (its category at Open Food Facts).',
      'Sold in Germany and Italy.',
      'Its barcode was issued in Germany — where the brand got its number, not where it was made.',
    ]);
    const many = {
      ...tin,
      sold_in: ['A', 'B', 'C', 'D', 'E', 'F'],
      registered: undefined,
      english: undefined,
    };
    expect(aboutLines(many)).toEqual([
      'Open Food Facts has no English name for it.',
      'Sold in A, B, C, D and 2 more countries.',
    ]);
    expect(aboutLines({ ...tin, english: { name: 'Chickpea, dried', from: 'list' } })[0]).toBe(
      'In English, the nearest food in the food list is Chickpea, dried.',
    );
  });
});
