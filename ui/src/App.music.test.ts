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
import * as player from './lib/music/player.svelte';

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
  player.restore(); // this test's empty storage: no queue
  // jsdom has no media playback: a play or pause only says it happened, as a browser's would.
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockImplementation(function (
    this: HTMLMediaElement,
  ) {
    Object.defineProperty(this, 'paused', { value: false, configurable: true });
    this.dispatchEvent(new Event('play'));
    return Promise.resolve();
  });
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(function (
    this: HTMLMediaElement,
  ) {
    Object.defineProperty(this, 'paused', { value: true, configurable: true });
    this.dispatchEvent(new Event('pause'));
  });
});
afterEach(() => {
  player.stop();
  vi.restoreAllMocks();
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
  const find = screen.getByLabelText('search music');
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

/** Add songs through the sheet, with their tags, and come back to the list. */
async function addSongs(songs: { file: string; title: string; artist: string }[]) {
  for (const x of songs)
    setAudioTags(x.file, { tag_title: x.title, artist: x.artist, duration_s: 200 });
  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  await choose(...songs.map((x) => x.file));
  const label = songs.length > 1 ? `Save ${songs.length} songs` : 'Save';
  await waitFor(() =>
    expect((screen.getByRole('button', { name: label }) as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(screen.getByRole('button', { name: label }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'add a song' })).toBeNull());
  return screen.findByRole('list', { name: 'songs' });
}

test('the Music view is a player: pressing a song plays it, and the list is the queue', async () => {
  render(App);
  await openMusic();
  // The deck is there before anything plays; the slim bar is not, here or anywhere.
  const deck = screen.getByRole('region', { name: 'music player' });
  expect(within(deck).getByText('READY — PICK A SONG BELOW')).toBeTruthy();
  expect((within(deck).getByRole('button', { name: 'play' }) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect(screen.queryByRole('region', { name: 'player' })).toBeNull();
  const list = await addSongs([
    { file: 'a.mp3', title: 'Arrival', artist: 'The Example Band' },
    { file: 'b.mp3', title: 'Bridges', artist: 'Another Example' },
    { file: 'c.mp3', title: 'Coda', artist: 'The Example Band' },
  ]);
  await waitFor(() => expect(within(list).getAllByRole('listitem')).toHaveLength(3));

  await fireEvent.click(within(list).getByRole('button', { name: 'play Bridges' }));
  await waitFor(() => expect(within(deck).getByText('now playing: Bridges')).toBeTruthy());
  expect(within(deck).getByText('TRK 02/03')).toBeTruthy();
  expect(within(deck).getByRole('button', { name: 'pause' })).toBeTruthy();
  // While the deck is on screen, the slim bar steps aside.
  expect(screen.queryByRole('region', { name: 'player' })).toBeNull();
  expect(
    within(screen.getByRole('list', { name: 'songs' }))
      .getByRole('button', { name: 'play Bridges' })
      .getAttribute('aria-current'),
  ).toBe('true');

  await fireEvent.click(within(deck).getByRole('button', { name: 'pause' }));
  expect(within(deck).getByRole('button', { name: 'play' })).toBeTruthy();
  await fireEvent.click(within(deck).getByRole('button', { name: 'next song' }));
  expect(within(deck).getByText('now playing: Coda')).toBeTruthy();
  // The last song, repeat off: nothing after it.
  expect(
    (within(deck).getByRole('button', { name: 'next song' }) as HTMLButtonElement).disabled,
  ).toBe(true);

  // The queue, the one playing marked; a press on another plays it.
  await fireEvent.click(within(deck).getByRole('button', { name: 'queue' }));
  const q = within(deck).getByRole('list', { name: 'queue' });
  const items = within(q)
    .getAllByRole('button')
    .filter((b) => b.classList.contains('q'));
  expect(items.map((b) => b.querySelector('.qt')?.firstChild?.textContent)).toEqual([
    'Arrival',
    'Bridges',
    'Coda',
  ]);
  expect(items[2].getAttribute('aria-current')).toBe('true');
  await fireEvent.click(items[0]);
  expect(within(deck).getByText('now playing: Arrival')).toBeTruthy();

  // It keeps playing in another view, where the slim bar takes over.
  const rail = screen.getByRole('navigation', { name: 'views' });
  await fireEvent.click(
    within(rail).getByRole('button', { name: /open Timeline|open Study rooms/ }),
  );
  const bar = await screen.findByRole('region', { name: 'player' });
  expect(within(bar).getByRole('button', { name: 'now playing: Arrival' })).toBeTruthy();
  await fireEvent.click(within(bar).getByRole('button', { name: 'stop and close the player' }));
  expect(screen.queryByRole('region', { name: 'player' })).toBeNull();
});

test('shuffle, repeat, mute and the volume are kept on this device', async () => {
  render(App);
  await openMusic();
  const deck = screen.getByRole('region', { name: 'music player' });
  const list = await addSongs([
    { file: 'a.mp3', title: 'Arrival', artist: 'The Example Band' },
    { file: 'b.mp3', title: 'Bridges', artist: 'Another Example' },
  ]);
  await fireEvent.click(within(list).getByRole('button', { name: 'play Bridges' }));

  // Repeat: off → all → one → off. On "all", the last song has a next: the first.
  await fireEvent.click(within(deck).getByRole('button', { name: 'repeat: off' }));
  expect(within(deck).getByRole('button', { name: 'repeat: all' })).toBeTruthy();
  await fireEvent.click(within(deck).getByRole('button', { name: 'next song' }));
  expect(within(deck).getByText('now playing: Arrival')).toBeTruthy();
  await fireEvent.click(within(deck).getByRole('button', { name: 'repeat: all' }));
  expect(within(deck).getByRole('button', { name: 'repeat: one' })).toBeTruthy();

  await fireEvent.click(within(deck).getByRole('button', { name: 'shuffle' }));
  expect(within(deck).getByRole('button', { name: 'shuffle' }).getAttribute('aria-pressed')).toBe(
    'true',
  );
  await fireEvent.input(within(deck).getByLabelText('volume'), { target: { value: '0.4' } });
  await fireEvent.click(within(deck).getByRole('button', { name: 'mute' }));
  expect(within(deck).getByRole('button', { name: 'unmute' })).toBeTruthy();

  const kept = JSON.parse(store.get('fm-player') ?? '{}');
  expect(kept).toMatchObject({ volume: 0.4, muted: true, shuffle: true, repeat: 'one' });
  // As after a restart: the settings come back with the queue.
  player.restore();
  expect(player.shuffle()).toBe(true);
  expect(player.repeat()).toBe('one');
  expect(player.volume()).toBe(0.4);
  expect(player.muted()).toBe(true);
});

test('the deck has three looks, switched in turn and kept on this device', async () => {
  render(App);
  await openMusic();
  const deck = screen.getByRole('region', { name: 'music player' });
  const list = await addSongs([{ file: 'a.mp3', title: 'Arrival', artist: 'The Example Band' }]);
  await fireEvent.click(within(list).getByRole('button', { name: 'play Arrival' }));
  expect(within(deck).getByText('TRK 01/01')).toBeTruthy();

  await fireEvent.click(within(deck).getByRole('button', { name: 'skin: Classic LCD' }));
  expect(deck.classList.contains('skin-pixel')).toBe(true);
  // The same display and keys under the 8-bit look.
  expect(within(deck).getByText('TRK 01/01')).toBeTruthy();

  await fireEvent.click(within(deck).getByRole('button', { name: 'skin: 8-bit' }));
  expect(deck.classList.contains('skin-arcade')).toBe(true);
  // The arcade look names the round, and still has the song's position to drag.
  expect(within(deck).getByText(/ROUND 01/)).toBeTruthy();
  expect(within(deck).getByLabelText('position in the song')).toBeTruthy();
  expect(within(deck).getByText('now playing: Arrival')).toBeTruthy();

  expect(JSON.parse(store.get('fm-player') ?? '{}').skin).toBe('arcade');
  player.restore();
  expect(player.skin()).toBe('arcade');
  await fireEvent.click(within(deck).getByRole('button', { name: 'skin: Arcade' }));
  expect(player.skin()).toBe('lcd');
});

test('albums and artists group the songs, in track order, and play from there', async () => {
  for (const [file, title, track, album] of [
    ['1.mp3', 'Zebra', 1, 'Invented Songs'],
    ['2.mp3', 'Apple', 2, 'Invented Songs'],
    ['3.mp3', 'Moon', 1, 'Other Things'],
  ] as const)
    setAudioTags(file, { tag_title: title, artist: 'The Example Band', album, track });
  render(App);
  await openMusic();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Music' }));
  await choose('1.mp3', '2.mp3', '3.mp3');
  await waitFor(() =>
    expect(
      (screen.getByRole('button', { name: 'Save 3 songs' }) as HTMLButtonElement).disabled,
    ).toBe(false),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Save 3 songs' }));
  await screen.findByRole('list', { name: 'songs' });

  await fireEvent.click(screen.getByRole('tab', { name: 'Albums' }));
  const grid = await screen.findByRole('list', { name: 'albums' });
  expect(
    within(grid)
      .getAllByRole('button')
      .map((b) => b.getAttribute('aria-label')),
  ).toEqual(['album Invented Songs', 'album Other Things']);
  await fireEvent.click(within(grid).getByRole('button', { name: 'album Invented Songs' }));
  const tracks = await screen.findByRole('list', { name: 'songs of Invented Songs' });
  // Track order, not title order.
  expect(
    within(tracks)
      .getAllByRole('listitem')
      .map((li) => li.querySelector('.title')?.textContent),
  ).toEqual(['Zebra', 'Apple']);
  await fireEvent.click(screen.getByRole('button', { name: 'Play' }));
  const deck = screen.getByRole('region', { name: 'music player' });
  await waitFor(() => expect(within(deck).getByText('now playing: Zebra')).toBeTruthy());
  expect(within(deck).getByText('TRK 01/02')).toBeTruthy();

  await fireEvent.click(screen.getByRole('tab', { name: 'Artists' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'artist The Example Band' }));
  expect(await screen.findByText('2 albums · 3 songs')).toBeTruthy();
});

test('a playlist is made from a song’s ⋯, plays in its order, and moves and drops songs', async () => {
  render(App);
  await openMusic();
  const list = await addSongs([
    { file: 'a.mp3', title: 'Arrival', artist: 'The Example Band' },
    { file: 'b.mp3', title: 'Bridges', artist: 'Another Example' },
  ]);
  // Bridges into a new playlist, then Arrival after it.
  await fireEvent.click(within(list).getByRole('button', { name: 'more for Bridges' }));
  let sheet = await screen.findByRole('dialog', { name: 'Bridges' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Add to playlist…' }));
  await fireEvent.input(within(sheet).getByLabelText('name of the new playlist'), {
    target: { value: 'Long drive' },
  });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Make it' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Bridges' })).toBeNull());
  await fireEvent.click(
    within(screen.getByRole('list', { name: 'songs' })).getByRole('button', {
      name: 'more for Arrival',
    }),
  );
  sheet = await screen.findByRole('dialog', { name: 'Arrival' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Add to playlist…' }));
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Long drive' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Arrival' })).toBeNull());

  await fireEvent.click(screen.getByRole('tab', { name: 'Playlists' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'playlist Long drive' }));
  const songsOf = () => screen.getByRole('list', { name: 'songs of Long drive' });
  const order = () =>
    within(songsOf())
      .getAllByRole('listitem')
      .map((li) => li.querySelector('.title')?.textContent);
  await waitFor(() => expect(order()).toEqual(['Bridges', 'Arrival']));

  // Move Arrival up, then play: the playlist's order is the queue's.
  await fireEvent.click(within(songsOf()).getByRole('button', { name: 'more for Arrival' }));
  sheet = await screen.findByRole('dialog', { name: 'Arrival' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Move up' }));
  await waitFor(() => expect(order()).toEqual(['Arrival', 'Bridges']));
  await fireEvent.click(screen.getByRole('button', { name: 'Play' }));
  expect(player.queue().map((t) => t.title)).toEqual(['Arrival', 'Bridges']);

  // Take Bridges out.
  await fireEvent.click(within(songsOf()).getByRole('button', { name: 'more for Bridges' }));
  sheet = await screen.findByRole('dialog', { name: 'Bridges' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Take out of this playlist' }));
  await waitFor(() => expect(order()).toEqual(['Arrival']));
});

test('play next and add to queue put songs where they say', async () => {
  render(App);
  await openMusic();
  const list = await addSongs([
    { file: 'a.mp3', title: 'Arrival', artist: 'The Example Band' },
    { file: 'b.mp3', title: 'Bridges', artist: 'Another Example' },
    { file: 'c.mp3', title: 'Coda', artist: 'The Example Band' },
  ]);
  await fireEvent.click(within(list).getByRole('button', { name: 'play Arrival' }));
  await fireEvent.click(within(list).getByRole('button', { name: 'more for Coda' }));
  const sheet = await screen.findByRole('dialog', { name: 'Coda' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Play next' }));
  expect(player.queue().map((t) => t.title)).toEqual(['Arrival', 'Coda', 'Bridges', 'Coda']);
  await fireEvent.click(within(list).getByRole('button', { name: 'more for Arrival' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog', { name: 'Arrival' })).getByRole('button', {
      name: 'Add to queue',
    }),
  );
  expect(player.queue().map((t) => t.title)).toEqual([
    'Arrival',
    'Coda',
    'Bridges',
    'Coda',
    'Arrival',
  ]);
});

test('the queue is kept on this device and comes back paused', async () => {
  render(App);
  await openMusic();
  const list = await addSongs([{ file: 'k.mp3', title: 'Kept', artist: 'The Example Band' }]);
  await fireEvent.click(within(list).getByRole('button', { name: 'play Kept' }));
  const deck = screen.getByRole('region', { name: 'music player' });
  await waitFor(() => expect(within(deck).getByText('now playing: Kept')).toBeTruthy());
  expect(JSON.parse(store.get('fm-queue') ?? '{}').queue[0].title).toBe('Kept');

  // As after a restart: the queue is read back, and nothing plays until asked.
  player.restore();
  await waitFor(() => {
    expect(within(deck).getByText('now playing: Kept')).toBeTruthy();
    expect(within(deck).getByRole('button', { name: 'play' })).toBeTruthy();
  });
});
