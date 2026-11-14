// **Music** (`decisions.md#ui` `#data`, 2026-11-13, *a sixth group, Music*): a song is a note with
// `log: music`; the plus uploads its file like any attachment and the file's tags fill the form;
// the Songs screen searches as you type, and a file already in the library is not added twice.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, setAudioTags } from './lib/mock';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';

const store = new Map<string, string>([['fm-trackers', '1']]);
beforeEach(() => {
  store.clear();
  store.set('fm-trackers', '1');
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  clearFaults();
  nav.reset();
  trackers.reload();
});
afterEach(() => {
  vi.unstubAllGlobals();
  reset();
  clearUndo();
});

const openMusic = async () => {
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Music' }));
  // One view so far, so no tabs: the group shows Songs straight away.
  await screen.findByText('No songs yet.');
};
/** Hand files to the sheet's file picker, as choosing them in the dialog does. */
const choose = async (...names: string[]) => {
  // The sheet is loaded when first opened.
  await screen.findByRole('dialog', { name: 'add a song' });
  const input = screen.getByLabelText('choose songs') as HTMLInputElement;
  const files = names.map((n) => new File(['x'], n, { type: 'audio/mpeg' }));
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  await fireEvent.change(input);
};

test('the plus adds a song that fills itself from its file, and Songs finds it as you type', async () => {
  setAudioTags('night train.mp3', {
    tag_title: 'Night Train',
    artist: 'The Example Band',
    album: 'Invented Songs',
    track: 4,
    duration_s: 214,
  });
  render(App);
  await openMusic();

  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  const sheet = await screen.findByRole('dialog', { name: 'add a song' });
  await choose('night train.mp3');
  // The tags fill the form: the title from the file, not its name.
  await waitFor(() =>
    expect((within(sheet).getByLabelText('Title') as HTMLInputElement).value).toBe('Night Train'),
  );
  expect((within(sheet).getByLabelText('Artist') as HTMLInputElement).value).toBe(
    'The Example Band',
  );
  expect((within(sheet).getByLabelText('Track') as HTMLInputElement).value).toBe('4');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'add a song' })).toBeNull());

  const list = await screen.findByRole('list', { name: 'songs' });
  expect(within(list).getByText('Night Train')).toBeTruthy();
  expect(within(list).getByText('The Example Band · Invented Songs')).toBeTruthy();
  expect(within(list).getByText('3:34')).toBeTruthy();

  // Search folds case and accents, and every word must match.
  const find = screen.getByLabelText('search songs');
  await fireEvent.input(find, { target: { value: 'EXAMPLE train' } });
  expect(within(screen.getByRole('list', { name: 'songs' })).getByText('Night Train')).toBeTruthy();
  await fireEvent.input(find, { target: { value: 'nothing like it' } });
  expect(await screen.findByText('Nothing matches “nothing like it”.')).toBeTruthy();
});

test('a file already in the library is not added twice', async () => {
  setAudioTags('arrival.mp3', { tag_title: 'Arrival', artist: 'The Example Band' });
  render(App);
  await openMusic();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  await choose('arrival.mp3');
  const sheet = screen.getByRole('dialog', { name: 'add a song' });
  await waitFor(() =>
    expect((within(sheet).getByLabelText('Title') as HTMLInputElement).value).toBe('Arrival'),
  );
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));
  await screen.findByRole('list', { name: 'songs' });

  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  await choose('arrival.mp3');
  expect(await screen.findByText(/already in your music as “Arrival”/)).toBeTruthy();
  expect(
    (screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled,
    'nothing new to save',
  ).toBe(true);
});

test('many files at once: one line each, a file with no tags is read by its name', async () => {
  setAudioTags('one.mp3', { tag_title: 'First Light', artist: 'The Example Band' });
  render(App);
  await openMusic();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  await choose('one.mp3', '05 - Some Artist - Some Title.mp3');
  const lines = await screen.findByRole('list', { name: 'songs to add' });
  await waitFor(() => expect(within(lines).getByText('First Light')).toBeTruthy());
  await waitFor(() => expect(within(lines).getByText('Some Title')).toBeTruthy());
  expect(within(lines).getByText('Some Artist')).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: 'Save 2 songs' }));
  const list = await screen.findByRole('list', { name: 'songs' });
  await waitFor(() => expect(within(list).getAllByRole('listitem')).toHaveLength(2));
  // Sorted by title.
  const titles = within(list)
    .getAllByRole('listitem')
    .map((li) => li.querySelector('.title')?.textContent);
  expect(titles).toEqual(['First Light', 'Some Title']);
});
