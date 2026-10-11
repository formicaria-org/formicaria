// Logging a food: what you ate before needs no keyboard, a food from the list gets a portion, and a
// food typed in by hand needs only a name and its energy.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import LogSheet from './LogSheet.svelte';
import { reset, seedTrackerDemo, handle } from '../mock';
import { clear as clearUndo, nextUndo, undo } from '../undo.svelte';
import { today } from '../units';
import type { LogDays, TotalsDef, ViewResult } from '../types';

let def: TotalsDef;
const entriesToday = async (slot: string) => {
  const r = await handle<LogDays>('log_days', {
    view: 'Day',
    from: today(),
    to: today(),
    rows: true,
  });
  return (r.days[0].rows ?? []).filter((x) => x.props.slot === slot);
};

beforeEach(async () => {
  seedTrackerDemo();
  def = (await handle<ViewResult>('run_view', { name: 'Day' })).totals!;
});
afterEach(() => {
  reset();
  clearUndo();
});

const open = (over = {}) => {
  const onclose = vi.fn();
  const onwrote = vi.fn();
  const { unmount } = render(LogSheet, {
    view: 'Day',
    def,
    day: today(),
    slot: 'lunch',
    onclose,
    onwrote,
    ...over,
  });
  return { onclose, onwrote, unmount };
};

describe('the food sheet', () => {
  it('opens on what was eaten before, and a row’s own plus logs it again in one tap', async () => {
    const { onwrote } = open();
    const again = await screen.findAllByRole('button', { name: /^add .* again$/ });
    expect(again.length).toBeGreaterThan(3);

    await fireEvent.click(again[0]);
    await waitFor(() => expect(onwrote).toHaveBeenCalled());
    expect(await screen.findByText('1 added')).toBeTruthy();
    expect(await entriesToday('lunch')).toHaveLength(1);
  });

  it('logging is one step to undo', async () => {
    open();
    await fireEvent.click((await screen.findAllByRole('button', { name: /^add .* again$/ }))[0]);
    await waitFor(() => expect(nextUndo()?.label).toMatch(/^logged/));
    await undo();
    expect(await entriesToday('lunch')).toHaveLength(0);
  });

  it('a food from the list gets a portion, and the entry records where it came from', async () => {
    const { onwrote } = open();
    await fireEvent.click(screen.getByRole('button', { name: 'Search' }));
    await fireEvent.input(screen.getByLabelText('search foods'), { target: { value: 'banana' } });
    await fireEvent.click(await screen.findByText('Banana'));

    // One medium banana, 118 g, at 89 kcal per 100 g.
    expect(await screen.findByText('105 kcal')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '100 g' }));
    expect(await screen.findByText('89 kcal')).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Add to Lunch' }));
    await waitFor(() => expect(onwrote).toHaveBeenCalled());
    const [made] = await entriesToday('lunch');
    expect(made.title).toBe('Banana');
    expect(made.props.amount_g).toBe(100);
    expect(made.props.kcal).toBe(89);
    expect(String(made.props.source)).toMatch(/f006$/);
  });

  it('a food typed in needs a name and its energy, and nothing else', async () => {
    open();
    await fireEvent.click(screen.getByRole('button', { name: 'Quick add' }));
    const add = screen.getByRole('button', { name: 'Add to Lunch' }) as HTMLButtonElement;
    expect(add.disabled).toBe(true);

    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Soup' } });
    await fireEvent.input(screen.getByLabelText('Energy (kcal)'), { target: { value: '180' } });
    expect(add.disabled).toBe(false);
    await fireEvent.click(add);

    await waitFor(async () => expect(await entriesToday('lunch')).toHaveLength(1));
    const [made] = await entriesToday('lunch');
    expect(made.props.kcal).toBe(180);
    expect('protein_mg' in made.props).toBe(false);
  });

  it('changing an amount and changing it back restores the exact numbers', async () => {
    const [oats] = await entriesToday('breakfast');
    const before = { ...oats.props };
    const first = open({ slot: null, entry: oats });
    await screen.findByText(oats.title!);
    const grams = screen.getByLabelText('amount in grams') as HTMLInputElement;
    await fireEvent.input(grams, { target: { value: '37' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(first.onclose).toHaveBeenCalled());

    const changed = (await entriesToday('breakfast')).find((x) => x.id === oats.id)!;
    expect(changed.props.amount_g).toBe(37);
    expect(changed.props.kcal).not.toBe(before.kcal);

    // Back to the amount it had: recomputed from the food, not rescaled from rounded numbers.
    first.unmount();
    const second = open({ slot: null, entry: changed });
    const again = (await screen.findByLabelText('amount in grams')) as HTMLInputElement;
    await fireEvent.input(again, { target: { value: String(before.amount_g) } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(second.onclose).toHaveBeenCalled());
    const back = (await entriesToday('breakfast')).find((x) => x.id === oats.id)!;
    expect(back.props).toEqual(before);
  });

  it('removing an entry says which one, so its owner can offer to take it back', async () => {
    const [oats] = await entriesToday('breakfast');
    const { onwrote, onclose } = open({ slot: null, entry: oats });
    await fireEvent.click(await screen.findByRole('button', { name: 'Remove' }));
    await waitFor(() => expect(onclose).toHaveBeenCalled());
    expect(onwrote).toHaveBeenCalledWith({ removed: oats.title });
    expect((await entriesToday('breakfast')).some((x) => x.id === oats.id)).toBe(false);
  });
});
