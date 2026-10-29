// The nutrients sheet says, for each entry without a nutrient, why; and a day still to come counts
// what is planned for it (2026-10-29: macadamia nuts planned for tomorrow read no selenium, and the
// sheet said only "No entry of this day carries it").
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { Nutrients, ObjectMeta } from '../types';

const { logNutrients } = vi.hoisted(() => ({ logNutrients: vi.fn() }));
vi.mock('../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../ipc')>()),
  logNutrients,
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
