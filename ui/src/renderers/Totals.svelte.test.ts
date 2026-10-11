// The day of a log: what remains of the target, the entries under their slots, and how much of a
// value the day's entries actually carry.
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Totals from './Totals.svelte';
import Trends from './Trends.svelte';
import type { DayTotals, ObjectMeta, TotalsDef } from '../lib/types';

const def: TotalsDef = {
  log: 'food',
  add: 'food',
  span: 'day',
  metrics: [
    { key: 'kcal', label: 'Energy', role: 'primary' },
    { key: 'protein_mg', label: 'Protein', role: 'secondary' },
    { key: 'iron_ug', label: 'Iron', role: 'detail' },
  ],
  slots: [
    { value: 'breakfast', label: 'Breakfast' },
    { value: 'lunch', label: 'Lunch' },
  ],
};
const row = (id: string, title: string, props: Record<string, unknown>) =>
  ({
    id,
    title,
    props,
    tags: [],
    assets: [],
    created: id,
    start: '2026-10-11',
  }) as unknown as ObjectMeta;
const day = (kcal: number, over: Partial<DayTotals> = {}): DayTotals => ({
  day: '2026-10-11',
  of: 2,
  sums: {
    kcal: { sum: kcal, n: 2 },
    protein_mg: { sum: 30000, n: 2 },
    iron_ug: { sum: 4000, n: 1 },
  },
  targets: { kcal: 2000, protein_mg: 120000, iron_ug: 8000 },
  rows: [
    row('a', 'Oats', { slot: 'breakfast', amount_g: 60, kcal: 228 }),
    row('b', 'Milk', { slot: 'breakfast', amount_g: 200, kcal: 100 }),
  ],
  ...over,
});
const props = (d: DayTotals | null, more = {}) => ({
  def,
  day: d,
  strip: d ? [d] : [],
  selected: '2026-10-11',
  today: '2026-10-11',
  onday: vi.fn(),
  onadd: vi.fn(),
  onedit: vi.fn(),
  ...more,
});

describe('a day of a log', () => {
  it('says what remains of the target', () => {
    render(Totals, props(day(328)));
    expect(screen.getByText('1,672')).toBeTruthy();
    expect(screen.getByText('left')).toBeTruthy();
  });

  it('says how far past the target, in the same calm words', () => {
    render(Totals, props(day(2300)));
    expect(screen.getByText('300')).toBeTruthy();
    expect(screen.getByText('over')).toBeTruthy();
  });

  it('with no target, shows the plain total', () => {
    render(Totals, props(day(328, { targets: {} })));
    expect(screen.getByText('328', { selector: '.big' })).toBeTruthy();
    expect(screen.queryByText('left')).toBeNull();
  });

  it('lists entries under their slot, with the slot’s own total', () => {
    render(Totals, props(day(328)));
    expect(screen.getByText('Oats')).toBeTruthy();
    expect(screen.getByText('60 g')).toBeTruthy();
    // 228 + 100, beside the slot's name.
    expect(screen.getByText('328', { selector: '.sum' })).toBeTruthy();
  });

  it('an empty slot is where you add to it — the empty state is the first action', async () => {
    const onadd = vi.fn();
    render(Totals, props(day(328), { onadd }));
    await fireEvent.click(screen.getByRole('button', { name: 'add to Lunch' }));
    expect(onadd).toHaveBeenCalledWith('lunch');
  });

  it('offers yesterday’s meal for an empty slot, and only for an empty one', async () => {
    const oncopy = vi.fn();
    render(Totals, props(day(328), { oncopy, yesterday: { breakfast: 2, lunch: 3 } }));
    const offers = screen.getAllByText(/Same as yesterday/);
    expect(offers).toHaveLength(1);
    await fireEvent.click(offers[0]);
    expect(oncopy).toHaveBeenCalledWith('lunch');
  });

  it('says so when a value comes from only some of the day’s entries', () => {
    render(Totals, props(day(328)));
    expect(screen.getByText('from 1 of 2')).toBeTruthy();
  });

  it('reports a tap on an entry and leaves what happens next to its owner', async () => {
    const onedit = vi.fn();
    render(Totals, props(day(328), { onedit }));
    await fireEvent.click(screen.getByText('Milk'));
    expect(onedit.mock.calls[0][0].id).toBe('b');
  });
});

describe('an entry that is planned', () => {
  const planned = () =>
    day(328, {
      rows: [
        row('a', 'Oats', { slot: 'breakfast', amount_g: 60, kcal: 228 }),
        row('p', 'Salmon', { slot: 'lunch', kcal: 300, planned: 1 }),
      ],
    });

  it('is on its day with a control to confirm it, and in no slot’s total', async () => {
    const onconfirm = vi.fn();
    render(Totals, props(planned(), { onconfirm }));
    await fireEvent.click(screen.getByRole('button', { name: 'confirm Salmon' }));
    expect(onconfirm.mock.calls[0][0].id).toBe('p');
    // Lunch holds only the planned entry: its total is nothing yet.
    expect(screen.getByText('0', { selector: '.sum' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'confirm Oats' })).toBeNull();
  });

  it('a day still to come says it is a plan', () => {
    render(Totals, props(planned(), { selected: '2026-10-12', strip: [planned()] }));
    expect(screen.getByText('Planned')).toBeTruthy();
  });
});

describe('a run of days', () => {
  const none: DayTotals = { day: '2026-10-10', of: 0, sums: {}, targets: { kcal: 2000 } };

  it('averages over the days that have entries, and says how many those are', () => {
    render(Trends, {
      def,
      days: [none, day(1000), { ...day(2000), day: '2026-10-12' }],
      span: 7,
      onspan: vi.fn(),
      onday: vi.fn(),
    });
    expect(screen.getByText('1,500')).toBeTruthy();
    expect(screen.getByText('Logged 2 of 3 days')).toBeTruthy();
  });

  it('a measured value is where it is now and how far it moved, not an average of sums', () => {
    const weight: TotalsDef = {
      log: 'body',
      add: 'body',
      span: 'range',
      metrics: [{ key: 'weight_g', label: 'Weight', role: 'primary', agg: 'mean' }],
      slots: [],
    };
    const on = (d: string, sum: number, n: number): DayTotals => ({
      day: d,
      of: n,
      sums: { weight_g: { sum, n } },
      targets: {},
    });
    render(Trends, {
      def: weight,
      // Two readings on the first day: their mean, 79.0 kg, is that day's weight.
      days: [on('2026-10-09', 158_000, 2), on('2026-10-10', 0, 0), on('2026-10-11', 78_400, 1)],
      span: 7,
      onspan: vi.fn(),
      onday: vi.fn(),
    });
    expect(screen.getByText('78.4')).toBeTruthy();
    expect(screen.getByText(/−0\.6 kg over\s+these days/)).toBeTruthy();
    expect(screen.getByText('Logged 2 of 3 days')).toBeTruthy();
    expect(screen.getByRole('img', { name: 'Weight over 3 days' })).toBeTruthy();
  });

  it('explains itself when nothing was logged', () => {
    render(Trends, { def, days: [none], span: 7, onspan: vi.fn(), onday: vi.fn() });
    expect(screen.getByText('Nothing logged in these days')).toBeTruthy();
  });
});
