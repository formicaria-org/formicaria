// **Music on in the background, on the phone** (`player.svelte.ts` with `PlaybackService`): the native
// player is handed the whole queue with each song's file, takes the keys, and the app's player
// mirrors what it says.
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import * as mock from '../mock';

const calls: [string, unknown[]][] = [];
const g = globalThis as Record<string, unknown>;
beforeEach(() => {
  mock.reset();
  calls.length = 0;
  const rec =
    (name: string) =>
    (...a: unknown[]) =>
      calls.push([name, a]);
  g.__fmPlayer = Object.fromEntries(
    [
      'setQueue',
      'play',
      'pause',
      'next',
      'previous',
      'jump',
      'seek',
      'volume',
      'modes',
      'stop',
    ].map((n) => [n, rec(n)]),
  );
  vi.resetModules();
});
afterEach(() => {
  delete g.__fmPlayer;
});

const track = (id: string, title: string) => ({
  id,
  title,
  artist: 'Band',
  album: 'Road',
  audio: `sha256:${id.repeat(8)}`,
  cover: null,
  duration: 200,
});

it('hands the queue to the native player and mirrors what it says', async () => {
  const p = await import('./player.svelte');
  p.playList([track('aaaaaaaa', 'One'), track('bbbbbbbb', 'Two')], 1);
  await vi.waitFor(() => expect(calls.some(([n]) => n === 'setQueue')).toBe(true));
  const [json, index, , play] = calls.find(([n]) => n === 'setQueue')![1] as [
    string,
    number,
    number,
    boolean,
  ];
  const items = JSON.parse(json) as { path: string; title: string }[];
  expect(items.map((i) => i.title)).toEqual(['One', 'Two']);
  expect(items[0].path).toBe(`/data/files/blobs/${'aaaaaaaa'.repeat(8)}`);
  expect([index, play]).toEqual([1, true]);
  // The keys go to it.
  p.toggle();
  p.next();
  expect(calls.map(([n]) => n)).toEqual(expect.arrayContaining(['pause', 'next']));
  // What it says is what the app shows.
  window.dispatchEvent(
    new CustomEvent('fm-player', {
      detail: { index: 0, playing: true, time: 12, length: 200, error: '' },
    }),
  );
  expect(p.at()).toBe(0);
  expect(p.playing()).toBe(true);
  expect(p.time()).toBe(12);
  p.stop();
  expect(calls.at(-1)?.[0]).toBe('stop');
});

// **A player that lost its songs is given them again** (2026-11-21, found in the emulator): when
// Android stops the service, or the player is cleared behind the page, it says it holds none; the
// next play hands the queue over again instead of asking for a song that is not there.
it('hands the queue over again when the native player says it holds none', async () => {
  const p = await import('./player.svelte');
  const list = [track('aaaaaaaa', 'One'), track('bbbbbbbb', 'Two')];
  p.playList(list, 0);
  await vi.waitFor(() => expect(calls.filter(([n]) => n === 'setQueue').length).toBe(1));
  window.dispatchEvent(
    new CustomEvent('fm-player', {
      detail: { index: 0, playing: false, time: 0, length: 0, error: '', count: 0 },
    }),
  );
  p.playList(list, 1);
  await vi.waitFor(() => expect(calls.filter(([n]) => n === 'setQueue').length).toBe(2));
  expect(calls.filter(([n]) => n === 'jump')).toEqual([]);
  // While it holds them, a press is a jump, as before.
  window.dispatchEvent(
    new CustomEvent('fm-player', {
      detail: { index: 1, playing: true, time: 1, length: 200, error: '', count: 2 },
    }),
  );
  p.playList(list, 0);
  expect(calls.at(-1)?.[0]).toBe('jump');
});
