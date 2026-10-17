// **Dictate a recipe** (2026-10-18; the owner: *"writing on phone is hard"*). In Food, a recipe can be
// said: what was said becomes a note of its own, and `@Myrme /recipe` is asked there, so the recipe
// arrives as a proposal to accept or change. Nothing said is lost — when it cannot be written down,
// the note keeps the recording, and no question is put to Myrme that it could not answer.
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

const { stop, reply } = vi.hoisted(() => ({ stop: vi.fn(), reply: vi.fn() }));
vi.mock('../record', () => ({ startRecording: async () => ({ stop, cancel: () => {} }) }));
vi.mock('../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../ipc')>()),
  reply,
}));

import LogSheet from './LogSheet.svelte';
import { reset, seedTrackerDemo, handle, setMockDictation } from '../mock';
import { clear as clearUndo } from '../undo.svelte';
import { today } from '../units';
import type { ObjectMeta, ViewResult } from '../types';

beforeEach(async () => {
  seedTrackerDemo();
  reply.mockResolvedValue({});
  stop.mockResolvedValue(new File([new Uint8Array(3200)], 'dictation.wav', { type: 'audio/wav' }));
  const def = (await handle<ViewResult>('run_view', { name: 'Day' })).totals!;
  render(LogSheet, {
    view: 'Day',
    def,
    day: today(),
    slot: 'lunch',
    onclose: vi.fn(),
    onwrote: vi.fn(),
  });
});
afterEach(() => {
  setMockDictation('Cinque uova, 500 grammi di mascarpone.', false);
  reset();
  clearUndo();
  vi.clearAllMocks();
});

const said = async () => {
  await fireEvent.click(screen.getByRole('button', { name: 'Recipes' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Dictate a recipe' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Stop & make the recipe' }));
};
const dictated = async () =>
  (await handle<ObjectMeta[]>('recent', {})).find((n) =>
    (n.title ?? '').startsWith('Dictated recipe'),
  );

it('keeps what was said as a note and asks Myrme to make the recipe of it', async () => {
  setMockDictation('Tiramisù per sei. Cinque uova, 500 grammi di mascarpone.');
  await said();
  expect(await screen.findByText(/Myrme is sorting what you said into a recipe/)).toBeTruthy();
  const note = await dictated();
  expect(note).toBeTruthy();
  const body = (await handle<{ body: string }>('get', { id: note!.id })).body;
  expect(body).toContain('Cinque uova, 500 grammi di mascarpone.');
  expect(reply).toHaveBeenCalledWith(note!.id, '@Myrme /recipe');
});

it('keeps the recording when it cannot be written down, and asks Myrme nothing', async () => {
  setMockDictation('', false);
  await said();
  expect(await screen.findByText(/Turn it on in Settings.*recording is kept/)).toBeTruthy();
  const note = await dictated();
  const body = (await handle<{ body: string }>('get', { id: note!.id })).body;
  expect(body).toMatch(/asset:sha256-/);
  expect(reply).not.toHaveBeenCalled();
});
