// **A blank note is not kept** (`decisions.md#data`, 2026-10-27). Owner: *"if one opens a note and
// does nothing with it, we do not keep it."* New note → close the window: the note is gone, the
// board does not show it, and Undo has no "created a note" left to fail on. Anything in it — text
// saved, or a title still being typed — keeps it.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { made } = vi.hoisted(() => ({ made: [] as string[] }));
vi.mock('./lib/ipc', async (importOriginal) => {
  const ipc = await importOriginal<typeof import('./lib/ipc')>();
  return {
    ...ipc,
    capture: async (...args: Parameters<typeof ipc.capture>) => {
      const meta = await ipc.capture(...args);
      made.push(meta.id);
      return meta;
    },
  };
});

import App from './App.svelte';
import * as mock from './lib/mock';
import { clear, nextUndo } from './lib/undo.svelte';

beforeEach(() => {
  mock.reset();
  clear();
  made.length = 0;
});

async function newNote(): Promise<string> {
  await fireEvent.click(await screen.findByRole('button', { name: /make something new/i }));
  await fireEvent.click(await screen.findByText('New note'));
  await vi.waitFor(() => expect(made.length).toBe(1));
  await screen.findByRole('button', { name: 'close note' });
  return made[0];
}

const exists = async (id: string) => (await mock.handle('get', { id })) !== null;
const settle = () => new Promise((r) => setTimeout(r, 50));

describe('a new note left as it came', () => {
  it('is not kept once its window is closed, and leaves nothing to undo', async () => {
    render(App);
    const id = await newNote();
    expect(nextUndo()?.label).toBe('created a note');
    await fireEvent.click(screen.getByRole('button', { name: 'close note' }));
    await vi.waitFor(async () => expect(await exists(id)).toBe(false));
    expect(nextUndo()?.label).not.toBe('created a note');
  });

  it('is kept when something was written in it', async () => {
    render(App);
    const id = await newNote();
    await mock.handle('update_body', { id, body: 'a thought', base: '' });
    await fireEvent.click(screen.getByRole('button', { name: 'close note' }));
    await settle();
    expect(await exists(id)).toBe(true);
  });

  it('is kept while a title is still being typed into it', async () => {
    render(App);
    const id = await newNote();
    // The properties are folded away while writing: the note's ＋, then Details.
    await fireEvent.click(await screen.findByRole('button', { name: 'note options' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Details' }));
    await fireEvent.input(await screen.findByLabelText('title'), { target: { value: 'Plan' } });
    await fireEvent.click(screen.getByRole('button', { name: 'close note' }));
    await settle();
    expect(await exists(id)).toBe(true);
  });
});
