// Where a logged value came from, in words (2026-10-29): a user logged eggs, saw no choline, and
// nothing said the egg's table never measures it.
import { describe, expect, it } from 'vitest';
import { borrowedOf, borrowedProp, nutrientName, tableOf, valueOf, whyMissing } from './provenance';

describe('provenance', () => {
  it('reads the table from an entry’s source', () => {
    expect(tableOf('foods@2026.10.4/ciqual:22000')).toBe('ciqual');
    expect(tableOf('foods@2026.10.4/crea:812')).toBe('crea');
    expect(tableOf('a label')).toBeNull();
    expect(tableOf(undefined)).toBeNull();
  });

  it('keeps borrowed values only for the keys the entry carries, and reads them back', () => {
    const prop = borrowedProp(
      { choline_ug: 'USDA: Egg, whole, raw', caffeine_ug: 'USDA: Egg, whole, raw' },
      ['kcal', 'choline_ug'],
    );
    expect(prop).toBe('choline_ug=USDA: Egg, whole, raw');
    expect(borrowedOf({ borrowed: prop })).toEqual({ choline_ug: 'USDA: Egg, whole, raw' });
    expect(borrowedOf({})).toEqual({});
  });

  it('reads an entry from before B12 and D moved to nanograms in the new unit', () => {
    expect(valueOf({ vitamin_b12_ug: 2 }, 'vitamin_b12_ng')).toBe(2000);
    expect(valueOf({ vitamin_b12_ng: 450 }, 'vitamin_b12_ng')).toBe(450);
    expect(valueOf({}, 'vitamin_d_ng')).toBeNull();
  });

  it('says why the entries that lack a nutrient lack it', () => {
    const entries = [
      { props: { source: 'foods@2026.10.4/crea:812' } },
      { props: { source: 'foods@2026.10.4/ciqual:22000' } },
      { props: { source: 'foods@2026.10.4/usda:1', choline_ug: 335000 } },
      { props: { kcal: 200 } },
    ];
    expect(whyMissing(entries, 'choline_ug')).toBe(
      'not measured in CREA (Italy) or Ciqual (France); one entry typed without it',
    );
    expect(whyMissing([{ props: { choline_ug: 1 } }], 'choline_ug')).toBe('');
  });

  it('names a nutrient key as a word', () => {
    expect(nutrientName('choline_ug')).toBe('choline');
    expect(nutrientName('vitamin_b12_ng')).toBe('vitamin B12');
    expect(nutrientName('vitamin_k1_ug')).toBe('vitamin K1');
    expect(nutrientName('vitamin_a_rae_ug')).toBe('vitamin A');
    expect(nutrientName('folate_dfe_ug')).toBe('folate');
  });
});
