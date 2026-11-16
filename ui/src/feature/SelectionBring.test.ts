// **Select, then bring it somewhere** (`decisions.md` 2026-11-17, *one frame*): a selection inside
// formicaria shows its icon beside it; pressed, it asks where; the passage arrives quoted, with a
// link back to the note it came from. Android's *Bring into note* (`fm-bring-selection`) opens the
// same chooser.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';

const store = new Map<string, string>();
beforeEach(() => {
  mock.reset();
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
});
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.unstubAllGlobals();
});

async function noteWithSelection() {
  const made = (await mock.handle('create_notes', {
    notes: [
      { title: 'Field notes', body: 'Ants carry leaves home.', props: {} },
      { title: 'Questions', body: 'Why leaves?', props: {} },
    ],
  })) as { id: string }[];
  history.replaceState(null, '', `/f/note/${made[0].id}`);
  render(Feature);
  const passage = await screen.findByText('Ants carry leaves home.');
  const r = document.createRange();
  r.selectNodeContents(passage);
  getSelection()!.removeAllRanges();
  getSelection()!.addRange(r);
  document.dispatchEvent(new Event('selectionchange'));
  return made;
}

it('the icon by a selection brings it into a note found by typing, with a link back', async () => {
  const [from, to] = await noteWithSelection();
  await fireEvent.click(
    await screen.findByRole('button', { name: 'bring the selection into a note' }),
  );
  const field = screen.getByRole('searchbox', { name: 'find the note to bring it into' });
  await fireEvent.input(field, { target: { value: 'Questions' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'Questions' }));
  await waitFor(async () => {
    const n = (await mock.handle('get', { id: to.id })) as { body: string };
    expect(n.body).toContain('> Ants carry leaves home.');
    expect(n.body).toContain(`(note:${from.id})`);
  });
  expect(await screen.findByRole('status')).toBeTruthy();
});

it('Android’s Bring into note opens the same chooser', async () => {
  await noteWithSelection();
  window.dispatchEvent(new Event('fm-bring-selection'));
  expect(await screen.findByRole('dialog', { name: 'where it goes' })).toBeTruthy();
});
