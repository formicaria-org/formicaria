import { describe, it, expect } from 'vitest';
import { show, showWith, typed, whole, addDays, daysBetween } from './units';

describe('units', () => {
  it('reads the unit from the end of the key', () => {
    expect(showWith('kcal', 1240)).toBe('1,240 kcal');
    expect(showWith('protein_mg', 82000)).toBe('82 g');
    expect(showWith('sodium_mg', 640)).toBe('640 mg');
    expect(showWith('iron_ug', 8500)).toBe('8.5 mg');
    expect(showWith('b12_ug', 2)).toBe('2 µg');
    expect(showWith('load_g', 60000)).toBe('60 kg');
    expect(show('duration_s', 88).text).toBe('1:28');
  });

  it('shows a value in its target’s unit, so the two never mix', () => {
    // 800 mg against a 30 g target reads 0.8 g, not 800 mg.
    expect(showWith('fiber_mg', 800, 30000)).toBe('0.8 g');
  });

  it('types grams for a milligram key', () => {
    expect(typed('protein_mg')).toEqual({ unit: 'g', per: 1000 });
    expect(typed('kcal')).toEqual({ unit: 'kcal', per: 1 });
    expect(typed('weight_g')).toEqual({ unit: 'kg', per: 1000 });
    expect(showWith('weight_g', 78400)).toBe('78.4 kg');
  });

  it('refuses a value that is not a whole number', () => {
    expect(whole(12)).toBe(12);
    expect(whole('12')).toBe(12);
    expect(whole('12.5')).toBeNull();
    expect(whole(undefined)).toBeNull();
    expect(whole('')).toBeNull();
  });

  it('walks days across a month end', () => {
    expect(addDays('2026-10-31', 1)).toBe('2026-11-01');
    expect(daysBetween('2026-10-30', '2026-11-01')).toEqual([
      '2026-10-30',
      '2026-10-31',
      '2026-11-01',
    ]);
  });
});

// Vitamin B12 and D are kept in nanograms and read in micrograms, with the decimal whole micrograms
// lost (2026-10-29: milk's B12, about 0.2 µg per 100 g, had been 0).
describe('nanograms', () => {
  it('are read in micrograms, to the tenth below ten', () => {
    expect(show('vitamin_b12_ng', 200)).toEqual({ text: '0.2', unit: 'µg' });
    expect(show('vitamin_b12_ng', 4000)).toEqual({ text: '4', unit: 'µg' });
    expect(show('vitamin_d_ng', 15000)).toEqual({ text: '15', unit: 'µg' });
  });
  it('are typed in micrograms', () => {
    expect(typed('vitamin_b12_ng')).toEqual({ unit: 'µg', per: 1000 });
  });
});
