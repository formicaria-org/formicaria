// Trends shows one thing, picked from a menu, and the rest below it
// (`decisions.md#ui`, 2026-10-12: a screen shows what was picked, and at most scrolls).
import { describe, expect, it } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Trends from './Trends.svelte';
import type { DayTotals, TotalsDef } from '../lib/types';

const def: TotalsDef = {
  log: 'food',
  add: 'food',
  span: 'range',
  metrics: [
    { key: 'kcal', label: 'Energy', role: 'primary' },
    { key: 'protein_mg', label: 'Protein', role: 'secondary' },
    { key: 'fiber_mg', label: 'Fibre', role: 'detail' },
  ],
  slots: [],
};
const days: DayTotals[] = ['2026-10-10', '2026-10-11', '2026-10-12'].map((day, i) => ({
  day,
  of: 2,
  sums: {
    kcal: { sum: 2000 + i * 100, n: 2 },
    protein_mg: { sum: 90_000, n: 2 },
    fiber_mg: { sum: 25_000, n: 2 },
  },
  targets: {},
}));
const open = () => render(Trends, { def, days, span: 7, onspan: () => {}, onday: () => {} });

describe('trends', () => {
  it('draws the view’s first value until another is picked', () => {
    open();
    const menu = screen.getByLabelText('what to show') as HTMLSelectElement;
    expect(menu.value).toBe('kcal');
    expect(Array.from(menu.options).map((o) => o.textContent)).toEqual([
      'Energy',
      'Protein',
      'Fibre',
    ]);
    // Three days, as three bars of the value picked.
    expect(screen.getAllByRole('button', { name: /kcal$/ })).toHaveLength(3);
  });

  it('picking a value draws that one, and the one it replaced joins the rest below', async () => {
    open();
    await fireEvent.change(screen.getByLabelText('what to show'), {
      target: { value: 'protein_mg' },
    });
    expect(screen.queryAllByRole('button', { name: /kcal$/ })).toHaveLength(0);
    expect(screen.getAllByRole('button', { name: /: 90 g$/ })).toHaveLength(3);
    // On the menu, and now among the rest.
    expect(screen.getAllByText('Energy')).toHaveLength(2);
  });
});
