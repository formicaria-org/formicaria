// **Any two of formicaria's items together** (a formicaria tab dragged beside another in formicaria's
// browser, `#pair=`): a note and a view, the first shown with the second.
import { render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());
afterEach(() => (location.hash = ''));

it('a note and the board, together', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Beside the board', body: 'Plans.', props: {} }],
  })) as { id: string }[];
  location.hash = `#pair=${encodeURIComponent(`note:${made.id}`)},${encodeURIComponent('view:board')}`;
  render(App);
  await screen.findAllByText('Plans.');
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('paired')).toBe(true),
  );
  const shown = [...document.querySelectorAll('.cell')].filter(
    (c) => c.classList.contains('active') || c.classList.contains('beside'),
  );
  expect(shown).toHaveLength(2);
});
