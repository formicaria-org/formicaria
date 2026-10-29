// Why an entry has no value for a nutrient (2026-10-29): under choline the sheet said only "No entry
// of this day carries it".
import { describe, expect, it } from 'vitest';
import { fromList, whyNot } from './refresh';

describe('why an entry has no value', () => {
  it('tells an entry of the food list from anything else', () => {
    expect(fromList({ source: 'foods@2026.10.5/crea:181105' })).toBe(true);
    expect(fromList({ source: 'product/01ABC' })).toBe(false);
    expect(fromList({})).toBe(false);
  });

  it('says why, by what the entry was logged from', () => {
    expect(whyNot({}, 'choline_ug')).toBe('typed in by hand, without it');
    expect(whyNot({ source: 'product/01ABC' }, 'choline_ug')).toBe(
      'its label does not state choline',
    );
    expect(whyNot({ source: 'recipe/omelette' }, 'choline_ug')).toBe(
      'its ingredients give no choline',
    );
    expect(whyNot({ source: 'foods@2026.10.5/crea:181105' }, 'iodine_ug')).toBe(
      'CREA (Italy) has no iodine for this food, and no close enough food in another table has',
    );
  });
});
