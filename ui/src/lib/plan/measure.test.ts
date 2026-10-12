import { describe, expect, it } from 'vitest';
import {
  boundProps,
  dayAgainst,
  gap,
  fraction,
  planBody,
  shownAsk,
  shownBound,
  shownValue,
  storedBound,
  unitOf,
  verdict,
} from './measure';
import type { PlanCheck } from '../types';

const fibre: PlanCheck = { id: 'fibre', label: 'Fibre', kind: 'sum', key: 'fiber_mg', basis: '' };
const fat: PlanCheck = { id: 'fat', label: 'Fat', kind: 'share', key: 'fat_mg', basis: '' };
const minutes: PlanCheck = { id: 'minutes', label: 'Activity', kind: 'minutes', basis: '' };

describe('a plan’s numbers, shown and typed', () => {
  it('types grams where milligrams are stored, and percent, minutes and days as they are', () => {
    expect(unitOf(fibre)).toEqual({ unit: 'g', per: 1000 });
    expect(unitOf(fat).unit).toBe('%');
    expect(unitOf(minutes).unit).toBe('min');
    expect(shownBound(fibre, 25_000)).toBe('25');
    expect(shownBound(fibre, undefined)).toBe('');
    expect(storedBound(fibre, '25')).toBe(25_000);
    expect(storedBound(fibre, '27.5')).toBe(27_500);
    expect(storedBound(fat, 30)).toBe(30);
  });

  it('reads an empty or meaningless field as "do not measure this"', () => {
    for (const nothing of ['', null, undefined, '0', '-3', 'lots'])
      expect(storedBound(fibre, nothing)).toBeUndefined();
  });

  it('says a value and what is asked of it in a person’s words', () => {
    expect(shownValue({ unit: 'fiber_mg' }, 23_000)).toBe('23 g');
    expect(shownValue({ unit: 'percent' }, 31)).toBe('31%');
    expect(shownValue({ unit: 'days' }, 1)).toBe('1 day');
    expect(shownValue({ unit: 'sets' }, 12)).toBe('12 sets');
    expect(shownAsk({ unit: 'fiber_mg', min: 25_000 })).toBe('at least 25 g');
    expect(shownAsk({ unit: 'percent', max: 30 })).toBe('at most 30%');
    expect(shownAsk({ unit: 'percent', min: 45, max: 75 })).toBe('45–75%');
    expect(shownAsk({ unit: 'minutes' })).toBe('');
  });

  it('fills a bar towards what is asked, and never past it', () => {
    expect(fraction({ min: 150, value: 75 })).toBe(0.5);
    expect(fraction({ max: 2000, value: 2600 })).toBe(1);
    expect(fraction({ min: 150 })).toBe(0);
  });

  /// Two of three days by Wednesday is not a failure.
  it('calls a week still under way "so far", and only a finished one "under"', () => {
    expect(verdict({ state: 'under', per: 'week' }, false)).toBe('so far');
    expect(verdict({ state: 'under', per: 'week' }, true)).toBe('under');
    expect(verdict({ state: 'under', per: 'day' }, false)).toBe('under');
    expect(verdict({ state: 'none', per: 'day' }, false)).toBe('nothing logged');
    expect(verdict({ state: 'met', per: 'week' }, false)).toBe('on track');
  });

  it('writes each bound on the note, and a day’s total also where the day screen reads it', () => {
    const props = boundProps(
      [fibre, fat, { ...fibre, id: 'sodium', key: 'sodium_ug' }, minutes],
      { fibre: { min: 25_000 }, fat: { max: 30 }, sodium: { max: 2_000_000 }, minutes: {} },
      ['kcal', 'fiber_mg'],
    );
    expect(props).toEqual({
      p_fibre_min: 25_000,
      fiber_mg: 25_000,
      p_fat_max: 30,
      p_sodium_max: 2_000_000,
    });
  });

  it('writes a note that reads without the app: what is asked, and whose number each is', () => {
    const body = planBody(
      {
        name: 'WHO healthy diet',
        summary: 'Limits.',
        sources: [
          { by: 'WHO', title: 'Healthy diet', url: 'https://example.org/a' },
          { by: 'ACSM', title: 'A paper', url: '' },
        ],
      },
      [{ label: 'Fibre', ask: 'at least 25 g', basis: 'WHO: at least 25 g a day' }],
    );
    expect(body).toContain('# WHO healthy diet');
    expect(body).toContain('- **Fibre**: at least 25 g. WHO: at least 25 g a day');
    expect(body).toContain('- WHO: [Healthy diet](<https://example.org/a>)');
    expect(body).toContain('- ACSM: A paper');
  });
});

describe('a day against what is followed', () => {
  const c = (over: object) => ({
    id: 'x',
    label: 'X',
    kind: 'sum' as const,
    unit: 'fiber_mg',
    per: 'day' as const,
    state: 'met' as const,
    days: [],
    basis: '',
    ...over,
  });
  it('says how far a check is from what it asks, in the unit it is shown in', () => {
    expect(gap(c({ state: 'under', min: 25_000, value: 12_000 }))).toBe('13 g to go');
    expect(gap(c({ state: 'over', unit: 'percent', max: 10, value: 14 }))).toBe('4% over');
    expect(gap(c({ state: 'met', min: 25_000, value: 30_000 }))).toBe('');
    expect(gap(c({ state: 'none', min: 25_000 }))).toBe('');
  });
  it('shows a night without eating in hours, typed in hours and kept in minutes', () => {
    expect(shownValue({ unit: 'duration' }, 840)).toBe('14 h');
    expect(shownValue({ unit: 'duration' }, 990)).toBe('16 h 30 min');
    expect(storedBound({ kind: 'fast' }, '14')).toBe(840);
    expect(shownBound({ kind: 'fast' }, 720)).toBe('12');
  });
  it('counts only the checks of a day, lists the ones that are off, and never faults a week under way', () => {
    const a = dayAgainst([
      c({ id: 'a', state: 'met' }),
      c({ id: 'b', state: 'under', min: 2, value: 1 }),
      c({ id: 'w', per: 'week', state: 'under' }),
    ]);
    expect([a.daily, a.met, a.off.map((x) => x.id), a.weekly.map((x) => x.id)]).toEqual([
      2,
      1,
      ['b'],
      ['w'],
    ]);
    expect(dayAgainst([c({ state: 'none' })]).nothing).toBe(true);
  });
});
