// Logging a food: what you ate before needs no keyboard, a food from the list gets a portion, and a
// food typed in by hand needs only a name and its energy.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import LogSheet from './LogSheet.svelte';
import { reset, seedTrackerDemo, handle } from '../mock';
import { clear as clearUndo, nextUndo, undo } from '../undo.svelte';
import { today } from '../units';
import type { LogDays, Recipe, TotalsDef, ViewResult } from '../types';

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

  it('opened without a meal it guesses none, and adds nothing until one is named', async () => {
    const { onwrote } = open({ slot: null });
    const again = await screen.findAllByRole('button', { name: /^add .* again$/ });
    expect((again[0] as HTMLButtonElement).disabled).toBe(true);

    const meal = screen.getByRole('combobox') as HTMLSelectElement;
    expect(meal.value).toBe('');
    await fireEvent.change(meal, { target: { value: 'dinner' } });
    const now = await screen.findAllByRole('button', { name: /^add .* again$/ });
    await waitFor(() => expect((now[0] as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(now[0]);
    await waitFor(() => expect(onwrote).toHaveBeenCalled());
    expect(await entriesToday('dinner')).toHaveLength(1);
  });

  it('a recipe is foods by weight and its servings, and a serving of it is logged as their sum', async () => {
    open();
    await fireEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    expect(await screen.findByText(/No recipes yet/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'New recipe' }));
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Rice bowl' } });
    await fireEvent.input(screen.getByLabelText('Servings it makes'), { target: { value: '2' } });
    const save = screen.getByRole('button', { name: 'Save recipe' }) as HTMLButtonElement;
    expect(save.disabled, 'a recipe needs something in it').toBe(true);

    await fireEvent.click(screen.getByRole('button', { name: 'Add an ingredient' }));
    await fireEvent.input(screen.getByLabelText('search foods'), { target: { value: 'rice' } });
    await fireEvent.click(await screen.findByText('Rice, brown, cooked'));
    // 195 g of a food at 123 kcal per 100 g is 240 kcal; halved, a serving is 120.
    await fireEvent.click(await screen.findByRole('button', { name: 'Add to the recipe' }));
    expect(await screen.findByText(/divided by 2/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Save recipe' }));

    await fireEvent.click(await screen.findByText('Rice bowl'));
    expect(await screen.findByText(/nothing is adjusted for\s+cooking/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'more' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Add to Lunch' }));
    await waitFor(async () => expect(await entriesToday('lunch')).toHaveLength(1));
    const [bowl] = await entriesToday('lunch');
    expect([bowl.title, bowl.props.kcal, bowl.props.servings_pct]).toEqual(['Rice bowl', 150, 125]);
    expect(String(bowl.props.source)).toMatch(/^recipe\//);
    expect(bowl.props.amount_g, 'the weight that went in is not the weight served').toBeUndefined();

    // Saving the recipe was one step, and the serving another.
    expect(await undo()).toBe('logged “Rice bowl”');
    expect(await undo()).toBe('saved the recipe “Rice bowl”');
  });

  it('a kept recipe can be changed: what it makes, what goes in it, and how much', async () => {
    await handle('create_notes', {
      notes: [
        { title: 'Rice bowl', tags: ['recipe'], props: { recipe: 'w1', servings: 2 } },
        {
          title: 'Rice, brown, cooked',
          props: { log: 'food', recipe: 'w1', amount_g: 200, kcal: 246, source: 'demo/f011' },
        },
      ],
    });
    const recipe = async () =>
      (await handle<Recipe[]>('log_recipes', { view: 'Day' })).find((r) => r.recipe === 'w1')!;
    open();
    await fireEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    await fireEvent.click(await screen.findByText('Rice bowl'));

    // What it makes.
    await fireEvent.input(screen.getByLabelText('Servings it makes'), { target: { value: '4' } });
    await fireEvent.change(screen.getByLabelText('Servings it makes'));
    await waitFor(async () => expect((await recipe()).servings).toBe(4));

    // One more thing in it.
    await fireEvent.click(await screen.findByRole('button', { name: 'Add an ingredient' }));
    await fireEvent.input(screen.getByLabelText('search foods'), { target: { value: 'oats' } });
    await fireEvent.click(await screen.findByText('Oats, rolled'));
    await fireEvent.click(await screen.findByRole('button', { name: 'Add to the recipe' }));
    await waitFor(async () => expect((await recipe()).of).toBe(2));
    // Back on the recipe, with both.
    expect(await screen.findByRole('button', { name: 'change Oats, rolled' })).toBeTruthy();

    // How much of one of them: recomputed from the food, not rescaled from a rounded number.
    await fireEvent.click(screen.getByRole('button', { name: 'change Rice, brown, cooked' }));
    await fireEvent.input(await screen.findByLabelText('amount in grams'), {
      target: { value: '100' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(async () => {
      const rice = (await recipe()).ingredients.find((i) => i.title === 'Rice, brown, cooked')!;
      expect([rice.props.amount_g, rice.props.kcal]).toEqual([100, 123]);
    });

    // And out again.
    await fireEvent.click(await screen.findByRole('button', { name: 'change Oats, rolled' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Take out' }));
    await waitFor(async () => expect((await recipe()).of).toBe(1));
    expect(await undo()).toBe('deleted “Oats, rolled”');
    await waitFor(async () => expect((await recipe()).of).toBe(2));
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
