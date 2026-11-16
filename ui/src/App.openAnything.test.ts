// ***Open…*** (the views list's first entry, on every device): one field for your notes and the web.
// A note found opens; a web address becomes a page item (on the phone the shell draws it).
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());
afterEach(() => (location.hash = ''));

it('a note found in Open… opens', async () => {
  await mock.handle('create_notes', {
    notes: [{ title: 'Leafcutter colonies', body: 'Fungus gardens.', props: {} }],
  });
  location.hash = '#open';
  render(App);
  const field = await screen.findByRole('searchbox', { name: 'search your notes or the web' });
  await fireEvent.input(field, { target: { value: 'example.org/ants' } });
  expect(await screen.findByRole('button', { name: 'Go to example.org/ants' })).toBeTruthy();
  expect(
    screen.getByRole('button', { name: 'Search the web for “example.org/ants”' }),
  ).toBeTruthy();

  await fireEvent.input(field, { target: { value: 'Leafcutter' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'Leafcutter colonies' }));
  expect((await screen.findAllByText('Fungus gardens.')).length).toBeGreaterThan(0);
  expect(screen.queryByRole('searchbox', { name: 'search your notes or the web' })).toBeNull();
});

it('a web address chosen in Open… becomes a page item', async () => {
  location.hash = '#open';
  render(App);
  const field = await screen.findByRole('searchbox', { name: 'search your notes or the web' });
  await fireEvent.input(field, { target: { value: 'example.org' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'Go to example.org' }));
  // Outside the phone a page item is its address, to open.
  await waitFor(() =>
    expect(screen.getByRole('link', { name: 'https://example.org' })).toBeTruthy(),
  );
});
