// **Text brought in from another app on the phone** (`decisions.md` 2026-11-16, *the phone's Bring
// in*): Android's Share with text (a page's link, a passage) or a selection whose menu offered
// formicaria. The sheet asks where; the passage arrives as the browser's do, quoted with its source.
// The shell is stood in for by a fake `__fmShare` holding texts, shaped as `MainActivity.kt`'s.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

let texts: { text: string; subject: string }[] = [];
let taken = 0;

beforeEach(() => {
  mock.reset();
  texts = [];
  taken = 0;
  (globalThis as { __fmShare?: unknown }).__fmShare = {
    pending: () => JSON.stringify({ files: [], refused: [], texts }),
    read: () => '',
    done: () => {},
    doneTexts: (n: number) => {
      texts = texts.slice(n);
      taken += n;
    },
  };
});
afterEach(() => {
  delete (globalThis as { __fmShare?: unknown }).__fmShare;
});

it('a passage shared from another app goes into the note chosen, quoted with its source', async () => {
  render(App);
  await screen.findByRole('button', { name: 'make something new' });
  texts = [{ text: 'Ants farm fungus in their nests. https://a.org/ants', subject: 'Ants' }];
  window.dispatchEvent(new Event('fm-share'));
  const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
  expect(within(sheet).getByRole('heading').textContent?.trim()).toBe('Bring this into…');
  expect(taken).toBe(1);
  await fireEvent.click(within(sheet).getByRole('button', { name: /New note/ }));
  await waitFor(
    async () => {
      const hits = (await mock.handle('search', { query: 'fungus' })) as { id: string }[];
      expect(hits.length).toBe(1);
      const note = (await mock.handle('get', { id: hits[0].id })) as { body: string };
      expect(note.body).toContain('> Ants farm fungus in their nests.');
      expect(note.body).toContain('— [Ants](https://a.org/ants)');
    },
    { timeout: 4000 },
  );
});
