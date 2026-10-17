// **A recipe is one note, reached from two places** (owner, 2026-10-18): its note is the text —
// method, times, pictures — and offers the way to its ingredients in the food view.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, expect, test, vi } from 'vitest';
import NotePanel from './NotePanel.svelte';
import { handle, reset } from './mock';
import type { ObjectMeta } from './types';

afterEach(() => reset());

const make = async (props: Record<string, string | number>, tags = ['recipe']) =>
  (
    await handle<ObjectMeta[]>('create_notes', {
      notes: [{ title: 'Tiramisù', body: '## Method\n\n1. Whip the eggs.\n', tags, props }],
    })
  )[0].id;

test('a recipe note offers its ingredients and servings, by its word', async () => {
  const id = await make({ recipe: 'w7', servings: 6 });
  const onopenrecipe = vi.fn();
  render(NotePanel, { id, onclose: vi.fn(), onopenrecipe });
  await fireEvent.click(await screen.findByRole('button', { name: 'Ingredients and servings' }));
  expect(onopenrecipe).toHaveBeenCalledWith('w7');
});

test('a note that is not a recipe offers nothing of the kind', async () => {
  const id = await make({}, ['dessert']);
  render(NotePanel, { id, onclose: vi.fn(), onopenrecipe: vi.fn() });
  await screen.findByText('Whip the eggs.');
  expect(screen.queryByRole('button', { name: 'Ingredients and servings' })).toBeNull();
});
