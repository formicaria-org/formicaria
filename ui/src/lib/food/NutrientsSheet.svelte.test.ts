// The nutrients sheet says, for each entry without a nutrient, why; and a day still to come counts
// what is planned for it (2026-10-29: macadamia nuts planned for tomorrow read no selenium, and the
// sheet said only "No entry of this day carries it").
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { Nutrients, ObjectMeta } from '../types';

const { logNutrients, setPropertiesUndoable, createNotesUndoable } = vi.hoisted(() => ({
  logNutrients: vi.fn(),
  setPropertiesUndoable: vi.fn(),
  createNotesUndoable: vi.fn(),
}));
vi.mock('../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../ipc')>()),
  logNutrients,
}));
vi.mock('../undo.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../undo.svelte')>()),
  setPropertiesUndoable,
  createNotesUndoable,
}));
import NutrientsSheet from './NutrientsSheet.svelte';
import { today } from '../units';

const entry = (id: string, title: string, props: Record<string, string | number>): ObjectMeta =>
  ({
    id,
    type: 'note',
    title,
    preview: '',
    status: null,
    due: null,
    start: null,
    hard: false,
    tags: [],
    props,
  }) as unknown as ObjectMeta;
const day = (entries: ObjectMeta[]): Nutrients => ({
  day: today(),
  of: entries.length,
  needs_sex: false,
  entries,
  rows: [{ key: 'choline_ug', label: 'Choline', kind: 'AI', reference: 400000, sum: 0, n: 0 }],
  drinks: [],
  on_day: [],
  most_on_day: 6,
});

beforeEach(() => vi.clearAllMocks());

test('each entry without a nutrient says why', async () => {
  logNutrients.mockResolvedValueOnce(
    day([
      entry('E1', 'Turbot, raw', { source: 'foods@2026.10.5/crea:123', amount_g: 120, kcal: 120 }),
      entry('T1', 'Toast', { kcal: 120 }),
    ]),
  );
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /Choline/ }));
  expect(
    await screen.findByText(
      /CREA \(Italy\) has no choline for this food, and no close enough food/,
    ),
  ).toBeTruthy();
  expect(screen.getByText(/typed in by hand, without it/)).toBeTruthy();
  expect(screen.queryByText(/No entry of this day carries it/)).toBeNull();
});

test('a day still to come counts what is planned for it; today counts what was eaten', async () => {
  logNutrients.mockResolvedValue(day([]));
  render(NutrientsSheet, { view: 'Day', day: '2999-01-01', onclose: () => {} });
  await screen.findByText(/Nothing is logged/);
  expect(logNutrients).toHaveBeenLastCalledWith('Day', '2999-01-01', true);
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await vi.waitFor(() => expect(logNutrients).toHaveBeenLastCalledWith('Day', today(), false));
});

test('an estimated value is shown as an estimate, with what it was estimated from', async () => {
  logNutrients.mockResolvedValueOnce({
    ...day([
      entry('M1', 'Pork chop', {
        source: 'foods@2026.10.6/bls:U1',
        amount_g: 150,
        choline_ug: 120000,
        estimated: 'choline_ug=5 similar foods (meat)',
      }),
    ]),
    rows: [
      { key: 'choline_ug', label: 'Choline', kind: 'AI', reference: 400000, sum: 120000, n: 1 },
    ],
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /Choline/ }));
  expect(await screen.findByText(/estimated from 5 similar foods \(meat\)/)).toBeTruthy();
  expect(screen.getByText(/^≈ /)).toBeTruthy();
});

test("the day's drinks: water by volume beside EFSA's, caffeine beside its limit, alcohol alone", async () => {
  logNutrients.mockResolvedValueOnce({
    ...day([entry('C1', 'Coffee', { kcal: 2 }), entry('B1', 'Beer', { kcal: 140 })]),
    drinks: [
      {
        key: 'water_mg',
        label: 'Water, from food and drinks',
        kind: 'AI',
        reference: 2_500_000,
        sum: 1_500_000,
        n: 2,
      },
      { key: 'caffeine_ug', label: 'Caffeine', kind: '', upper: 400_000, sum: 450_000, n: 1 },
      { key: 'alcohol_mg', label: 'Alcohol', kind: '', sum: 13_000, n: 1 },
    ],
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  expect(await screen.findByRole('heading', { name: 'Drinks and water' })).toBeTruthy();
  expect(screen.getByText('1.5 l')).toBeTruthy();
  expect(screen.getByText(/\/ 2\.5 l/)).toBeTruthy();
  // A limit alone is not an upper level: caffeine's is EFSA's safe intake.
  expect(screen.getByText('above the limit of 400 mg')).toBeTruthy();
  // Alcohol is an amount: no reference, no limit, no meter.
  const alcohol = screen.getByRole('button', { name: /Alcohol/ });
  expect(alcohol.textContent).toContain('13 g');
  expect(alcohol.querySelector('[role="meter"], meter, .meter')).toBeNull();
  expect(alcohol.textContent).not.toMatch(/limit|\//);
});

test('a value is put on Day and taken off it, kept in the profile note; six at most', async () => {
  const water = { key: 'water_mg', label: 'Water', kind: 'AI', sum: 800_000, n: 1 };
  const coffee = [entry('C1', 'Coffee', { kcal: 2 })];
  logNutrients.mockResolvedValue({
    ...day(coffee),
    drinks: [water],
    on_day: ['fiber_mg'],
    profile: 'P1',
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /^Water/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Show on Day' }));
  // Added after what Day already shows, as one undoable step, and the sheet asks again.
  await vi.waitFor(() =>
    expect(setPropertiesUndoable).toHaveBeenCalledWith(
      'P1',
      { day_values: 'fiber_mg, water_mg' },
      'put a value on Day',
    ),
  );
  await vi.waitFor(() => expect(logNutrients).toHaveBeenCalledTimes(2));
});

test('the last value taken off is written as none, so Day shows none rather than its own', async () => {
  const water = { key: 'water_mg', label: 'Water', kind: 'AI', sum: 800_000, n: 1 };
  logNutrients.mockResolvedValue({
    ...day([entry('C1', 'Coffee', { kcal: 2 })]),
    drinks: [water],
    on_day: ['water_mg'],
    profile: 'P1',
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /^Water/ }));
  const off = screen.getByRole('button', { name: 'Shown on Day · take it off' });
  expect(off.getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(off);
  await vi.waitFor(() =>
    expect(setPropertiesUndoable).toHaveBeenCalledWith(
      'P1',
      { day_values: 'none' },
      'took a value off Day',
    ),
  );
});

test('with no profile yet, putting a value on Day writes the profile note', async () => {
  const water = { key: 'water_mg', label: 'Water', kind: 'AI', sum: 800_000, n: 1 };
  logNutrients.mockResolvedValue({
    ...day([entry('C1', 'Coffee', { kcal: 2 })]),
    drinks: [water],
    on_day: ['fiber_mg', 'iron_ug'],
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /^Water/ }));
  await fireEvent.click(screen.getByRole('button', { name: 'Show on Day' }));
  await vi.waitFor(() => expect(createNotesUndoable).toHaveBeenCalled());
  const [[drafts]] = createNotesUndoable.mock.calls;
  expect(drafts[0].props).toEqual({ profile: 'me', day_values: 'fiber_mg, iron_ug, water_mg' });
});

test('with six on Day, a seventh waits until one is taken off', async () => {
  const water = { key: 'water_mg', label: 'Water', kind: 'AI', sum: 800_000, n: 1 };
  logNutrients.mockResolvedValue({
    ...day([entry('C1', 'Coffee', { kcal: 2 })]),
    drinks: [water],
    on_day: ['a_mg', 'b_mg', 'c_mg', 'd_mg', 'e_mg', 'f_mg'],
    profile: 'P1',
  });
  render(NutrientsSheet, { view: 'Day', day: today(), onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /^Water/ }));
  expect((screen.getByRole('button', { name: 'Show on Day' }) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect(screen.getByText('Day shows 6 at most: take one off first.')).toBeTruthy();
});

test('opened to choose, every value is one line to tick, up to Day’s most, even before anything is logged', async () => {
  logNutrients.mockResolvedValue({
    ...day([]),
    drinks: [{ key: 'water_mg', label: 'Water', kind: 'AI', sum: 0, n: 0 }],
    on_day: ['water_mg', 'fiber_mg', 'calcium_ug'],
    most_on_day: 3,
    profile: 'P1',
  });
  render(NutrientsSheet, { view: 'Day', day: today(), choose: true, onclose: () => {} });
  expect(await screen.findByRole('heading', { name: 'Follow on Day' })).toBeTruthy();
  expect(screen.queryByText(/Nothing is logged/)).toBeNull();
  const water = screen.getByRole('button', { name: /^Water/ });
  expect(water.getAttribute('aria-pressed')).toBe('true');
  // Three already: another waits until one is unticked.
  const choline = screen.getByRole('button', { name: /^Choline/ }) as HTMLButtonElement;
  expect(choline.disabled).toBe(true);
  expect(screen.getByText('Day shows 3: untick one to choose another.')).toBeTruthy();
  await fireEvent.click(water);
  await vi.waitFor(() =>
    expect(setPropertiesUndoable).toHaveBeenCalledWith(
      'P1',
      { day_values: 'fiber_mg, calcium_ug' },
      'took a value off Day',
    ),
  );
});
