// **⋯ → Export and History** (`decisions.md` 2026-11-22, later): a note's options open from ⋯; its
// history lists versions, one can be read and its text put back as a new change; its Markdown is
// downloaded as the file it is kept as. And a card's empty status is a ring, not a ＋.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';
import { clear } from '../lib/undo.svelte';

beforeEach(() => {
  mock.reset();
  clear();
});
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function openNote(title: string, body: string) {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title, body, props: {} }],
  })) as { id: string }[];
  history.replaceState(null, '', `/f/note/${made.id}`);
  render(Feature);
  await screen.findByText(body.trim());
  return made.id;
}

it('a note’s options open from ⋯', async () => {
  await openNote('Plan', 'Now.');
  const more = screen.getByRole('button', { name: 'note options' });
  expect(more.textContent?.trim()).toBe('⋯');
});

it('⋯ → History reads a version and puts its text back', async () => {
  const id = await openNote('Plan', 'Now.');
  mock.setHistory(id, [
    { commit: 'aaaaaaa1', author: 'Ana', time: '2026-11-20T10:00:00Z', body: 'Now.' },
    { commit: 'bbbbbbb2', author: 'Ana', time: '2026-11-18T10:00:00Z', body: 'Before.' },
  ]);
  await fireEvent.click(screen.getByRole('button', { name: 'note options' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'History…' }));
  const panel = await screen.findByRole('dialog', { name: 'history of this note' });
  const list = await within(panel).findByRole('list', { name: 'versions' });
  await fireEvent.click(within(list).getAllByRole('button')[1]);
  expect((await within(panel).findByLabelText("this version's text")).textContent).toBe('Before.');
  await fireEvent.click(
    within(panel).getByRole('button', { name: "Put this version's text back" }),
  );
  await vi.waitFor(async () =>
    expect(((await mock.handle('get', { id })) as { body: string }).body).toBe('Before.'),
  );
});

it('⋯ → Export → Markdown downloads the note’s file', async () => {
  await openNote('Plan', 'Now.');
  const made: Blob[] = [];
  vi.stubGlobal('URL', {
    ...URL,
    createObjectURL: (b: Blob) => (made.push(b), 'blob:x'),
    revokeObjectURL: () => {},
  });
  await fireEvent.click(screen.getByRole('button', { name: 'note options' }));
  await fireEvent.click(await screen.findByRole('button', { name: /^Export/ }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Markdown (.md)' }));
  await vi.waitFor(() => expect(made.length).toBe(1));
  expect(await made[0].text()).toContain('Now.');
});
