// The nutrients sheet says, for each entry without a nutrient, why — and offers to bring an entry
// logged before the food list had a value up to date (2026-10-29: eggs logged after the update still
// read no choline, and the sheet said only "No entry of this day carries it").
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { FoodDetail, Nutrients, ObjectMeta } from '../types';

const { logNutrients, foodGet, setProperties } = vi.hoisted(() => ({
  logNutrients: vi.fn(),
  foodGet: vi.fn(),
  setProperties: vi.fn(),
}));
vi.mock('../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../ipc')>()),
  logNutrients,
  foodGet,
  setProperties,
}));
import NutrientsSheet from './NutrientsSheet.svelte';

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

const eggs = entry('E1', 'Chicken Egg, whole, boiled', {
  source: 'foods@2026.10.4/crea:181105',
  amount_g: 120,
  kcal: 154,
});
const toast = entry('T1', 'Toast', { kcal: 120 });
const boiled: FoodDetail = {
  id: 'crea:181105',
  name: 'Chicken Egg, whole, boiled',
  src: 'crea',
  grp: '',
  per100: { kcal: 128, choline_ug: 268000 },
  portions: [],
  pack: 'foods@2026.10.5',
  borrowed: { choline_ug: 'USDA: Egg, whole, boiled or poached' },
};
const day = (entries: ObjectMeta[], choline: { sum: number; n: number }): Nutrients => ({
  day: '2026-10-29',
  of: entries.length,
  needs_sex: false,
  entries,
  rows: [{ key: 'choline_ug', label: 'Choline', kind: 'AI', reference: 400000, ...choline }],
});

beforeEach(() => {
  vi.clearAllMocks();
  foodGet.mockResolvedValue(boiled);
  setProperties.mockResolvedValue({ previous: {} });
});

test('each entry without a nutrient says why, and one logged before the list had it can be updated', async () => {
  logNutrients.mockResolvedValueOnce(day([eggs, toast], { sum: 0, n: 0 }));
  const onwrote = vi.fn();
  render(NutrientsSheet, { view: 'Day', day: '2026-10-29', onclose: () => {}, onwrote });

  const banner = await screen.findByRole('status');
  expect(banner.textContent?.replace(/\s+/g, ' ')).toMatch(
    /One entry was logged before the food list had/,
  );
  await fireEvent.click(screen.getByRole('button', { name: /Choline/ }));
  expect(await screen.findByText(/logged before the food list had choline for it/)).toBeTruthy();
  expect(screen.getByText(/typed in by hand, without it/)).toBeTruthy();
  expect(screen.queryByText(/No entry of this day carries it/)).toBeNull();

  logNutrients.mockResolvedValueOnce(
    day([{ ...eggs, props: { ...eggs.props, choline_ug: 321600 } } as ObjectMeta, toast], {
      sum: 321600,
      n: 1,
    }),
  );
  await fireEvent.click(within(banner).getByRole('button', { name: 'Update it' }));
  await waitFor(() =>
    expect(setProperties).toHaveBeenCalledWith('E1', {
      choline_ug: 321600,
      borrowed: 'choline_ug=USDA: Egg, whole, boiled or poached',
    }),
  );
  await waitFor(() => expect(onwrote).toHaveBeenCalled());
  await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
});

test('a food no table has a value for says so, and nothing is offered', async () => {
  foodGet.mockResolvedValue({ ...boiled, per100: { kcal: 128 }, borrowed: {} });
  logNutrients.mockResolvedValueOnce(day([eggs], { sum: 0, n: 0 }));
  render(NutrientsSheet, { view: 'Day', day: '2026-10-29', onclose: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: /Choline/ }));
  expect(
    await screen.findByText(
      /CREA \(Italy\) has no choline for this food, and no close enough food in another table has/,
    ),
  ).toBeTruthy();
  expect(screen.queryByRole('status')).toBeNull();
});
