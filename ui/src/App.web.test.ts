// **The web, by name** (the owner on 0.47: *"I see no browser in the phone"*): *Web* in the views list
// asks only for an address or a search, and opens the page (inside formicaria on the phone).
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());

it('Web is in the views list, and an address there opens the page', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Web' }));
  const field = await screen.findByRole('searchbox', { name: 'a web address or a search' });
  await fireEvent.input(field, { target: { value: 'ants' } });
  expect(await screen.findByRole('button', { name: 'Search the web for “ants”' })).toBeTruthy();
  await fireEvent.input(field, { target: { value: 'example.org' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'Go to example.org' }));
  await waitFor(() =>
    expect(screen.getByRole('link', { name: 'https://example.org' })).toBeTruthy(),
  );
});

it('there is no Open… beside the views: they open the features, search finds notes', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await screen.findByRole('menuitem', { name: 'Web' });
  expect(screen.queryByRole('menuitem', { name: 'Open…' })).toBeNull();
});
