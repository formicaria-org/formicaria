// The day of a log: what remains of the target, the entries under their slots, and how much of a
// value the day's entries actually carry.
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
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

  it('a meal is one line — its count and total — and pressed, its entries', async () => {
    render(Totals, props(day(328)));
    // 228 + 100, beside the slot's name, with how many foods; the foods themselves folded away.
    expect(screen.getByText('328', { selector: '.sum' })).toBeTruthy();
    expect(screen.getByText('2 ·')).toBeTruthy();
    expect(screen.queryByText('Oats')).toBeNull();
    const meal = screen.getByRole('button', { name: /^Breakfast/ });
    expect(meal.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(meal);
    expect(meal.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByText('Oats')).toBeTruthy();
    expect(screen.getByText('60 g')).toBeTruthy();
    await fireEvent.click(meal);
    await waitFor(() => expect(screen.queryByText('Oats')).toBeNull());
  });

  it('a meal that gains a food on the day shown opens, so what was added is in sight', async () => {
    const { rerender } = render(Totals, props(day(328)));
    expect(screen.queryByText('Oats')).toBeNull();
    const more = day(400);
    more.rows = [...(more.rows ?? []), row('c', 'Tea', { slot: 'breakfast', kcal: 2 })];
    await rerender(props(more));
    expect(await screen.findByText('Tea')).toBeTruthy();
    expect(screen.getByRole('button', { name: /^Breakfast/ }).getAttribute('aria-expanded')).toBe(
      'true',
    );
  });

  it('a day without slots — training — lists its entries as before, nothing folded', () => {
    render(Totals, props(day(328), { def: { ...def, slots: [] } }));
    expect(screen.getByText('Oats')).toBeTruthy();
    expect(document.querySelector('button.fold')).toBeNull();
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
    // An empty meal with yesterday's to offer is a line too; pressed, it offers it.
    await fireEvent.click(screen.getByRole('button', { name: /^Lunch/ }));
    const offers = screen.getAllByText(/Same as yesterday/);
    expect(offers).toHaveLength(1);
    await fireEvent.click(offers[0]);
    expect(oncopy).toHaveBeenCalledWith('lunch');
  });

  it('says so when a value comes from only some of the day’s entries', () => {
    render(Totals, props(day(328)));
    expect(screen.getByText('from 1 of 2')).toBeTruthy();
  });

  it('the values followed are rings: towards a target, else the sheet’s figure; a limit stays empty', async () => {
    const onchoose = vi.fn();
    const followDef: TotalsDef = {
      ...def,
      metrics: [
        ...def.metrics.filter((m) => m.role !== 'detail'),
        { key: 'water_mg', label: 'Water', role: 'detail' },
        { key: 'iron_ug', label: 'Iron', role: 'detail' },
        { key: 'caffeine_ug', label: 'Caffeine', role: 'detail' },
        { key: 'zinc_ug', label: 'Zinc', role: 'detail' },
      ],
    };
    const d = day(328, {
      sums: {
        kcal: { sum: 328, n: 2 },
        water_mg: { sum: 1_100_000, n: 2 },
        iron_ug: { sum: 4000, n: 1 },
        caffeine_ug: { sum: 450_000, n: 1 },
      },
    });
    render(
      Totals,
      props(d, {
        def: followDef,
        onchoose,
        references: { water_mg: { reference: 2_000_000 }, caffeine_ug: { upper: 400_000 } },
      }),
    );
    const follow = screen.getByRole('region', { name: 'what you follow' });
    // Three at most, in the order chosen.
    expect(within(follow).getAllByRole('img')).toHaveLength(3);
    expect(within(follow).queryByText('Zinc')).toBeNull();
    // Water against the sheet's figure, iron against the person's own target.
    expect(within(follow).getByRole('img', { name: 'Water: 1.1 l of 2 l' })).toBeTruthy();
    expect(within(follow).getByText('of 8 mg')).toBeTruthy();
    // A limit is said, not filled towards.
    expect(within(follow).getByText(/over the limit of\s+400 mg/)).toBeTruthy();
    await fireEvent.click(within(follow).getByRole('button', { name: 'Choose' }));
    expect(onchoose).toHaveBeenCalled();
  });

  it('reports a tap on an entry and leaves what happens next to its owner', async () => {
    const onedit = vi.fn();
    render(Totals, props(day(328), { onedit }));
    await fireEvent.click(screen.getByRole('button', { name: /^Breakfast/ }));
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
    await fireEvent.click(screen.getByRole('button', { name: /^Lunch/ }));
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
