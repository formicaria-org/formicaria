// **No shelf to place a note on** (`decisions.md#ui`, 2026-11-25, *the shelves go*): *Place on a
// shelf…* left the note's ＋ (Options) with the Shelves view it filled.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

it('a note offers no shelf to place it on', async () => {
  mock.reset();
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Saved page', body: 'Text.', props: { source: 'web' }, tags: ['web'] }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'note options' }));
  expect(await screen.findByRole('button', { name: 'Bring into another note…' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: /shelf/i })).toBeNull();
});
