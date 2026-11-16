// **Two together, only when asked** (the owner, 2026-11-16): a card's *with this one* shows that item
// with the one in front; *One at a time* (or choosing a card) goes back to one. **Web on the
// phone**: from the view menu; a web address there opens a page item.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());
afterEach(() => (location.hash = ''));

async function twoNotes() {
  const made = (await mock.handle('create_notes', {
    notes: [
      { title: 'Reading', body: 'One.', props: {} },
      { title: 'Writing', body: 'Two.', props: {} },
    ],
  })) as { id: string }[];
  location.hash = `#note=${made[0].id}`;
  render(App);
  await screen.findAllByText('One.');
  location.hash = `#note=${made[1].id}`;
  window.dispatchEvent(new HashChangeEvent('hashchange'));
  await screen.findAllByText('Two.');
}
const shown = () =>
  [...document.querySelectorAll('.cell')].filter(
    (c) => c.classList.contains('active') || c.classList.contains('beside'),
  ).length;

it('a card shows its item together with the one in front, and one at a time again', async () => {
  await twoNotes();
  await fireEvent.click(await screen.findByRole('button', { name: /open notes/ }));
  await fireEvent.click(screen.getByRole('menuitem', { name: 'show Reading with this one' }));
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('paired')).toBe(true),
  );
  expect(shown()).toBe(2);

  await fireEvent.click(screen.getByRole('button', { name: /open notes/ }));
  await fireEvent.click(screen.getByRole('menuitem', { name: 'One at a time' }));
  await waitFor(() =>
    expect(document.querySelector('.app')!.classList.contains('paired')).toBe(false),
  );
  expect(shown()).toBe(1);
});

it('Web on the phone: a web address opens the page as an item of the collection', async () => {
  const shown: string[] = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: (_id: string, url: string) => shown.push(url),
    hide: () => {},
    close: () => {},
    back: () => {},
    bring: () => {},
  };
  try {
    render(App);
    await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Web' }));
    const field = await screen.findByRole('searchbox', { name: 'a web address or a search' });
    await fireEvent.input(field, { target: { value: 'example.org/ants' } });
    await fireEvent.click(await screen.findByRole('button', { name: 'Go to example.org/ants' }));
    // A page item: the box the shell draws the page over, with nothing on top of it.
    await waitFor(() => expect(document.querySelector('.page-box')).toBeTruthy());
  } finally {
    delete g.__fmPage;
  }
});
