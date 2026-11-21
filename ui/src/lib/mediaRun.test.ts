// **Save media's queue survives a paused or closed app** (2026-11-21: an album saved on a phone that
// went to the background kept its first track only). The tracks not yet filed are kept on the
// device; the next start carries on, and a track already in the library is not saved twice.
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import * as mock from './mock';
import type { MediaAsk } from './ipc';
import * as run from './mediaRun';

const ask = (n: number): MediaAsk => ({
  url: `https://t4.bcbits.com/stream/x/mp3-128/${n}`,
  page: 'https://ant-band.bandcamp.com/album/night-walk',
  what: 'mp3',
  into: 'music',
  list: '',
  meta: { title: `Track ${n}`, source: `https://ant-band.bandcamp.com/track/track-${n}` },
});

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
afterEach(() => vi.unstubAllGlobals());

it('carries on at the next start with the tracks a closed app had not finished', async () => {
  localStorage.setItem('fm-media-queue', JSON.stringify([ask(2), ask(3)]));
  expect(run.resumeQueue()).toBe(2);
  await vi.waitFor(() => expect(mock.mockMediaAsks).toHaveLength(2), { timeout: 8000 });
  expect(mock.mockMediaAsks.map((a) => (a.meta as { title: string }).title)).toEqual([
    'Track 2',
    'Track 3',
  ]);
  await vi.waitFor(() => expect(localStorage.getItem('fm-media-queue')).toBeNull(), {
    timeout: 8000,
  });
  // Nothing left: a second start has nothing to carry on with.
  expect(run.resumeQueue()).toBe(0);
});

it('does not save again a track whose song is already in the library', async () => {
  await mock.handle('create_notes', {
    notes: [
      {
        title: 'Track 2',
        props: { log: 'music', audio: 'sha256:00', source: ask(2).meta!.source! },
      },
    ],
  });
  localStorage.setItem('fm-media-queue', JSON.stringify([ask(2), ask(3)]));
  run.resumeQueue();
  await vi.waitFor(() => expect(localStorage.getItem('fm-media-queue')).toBeNull(), {
    timeout: 8000,
  });
  expect(mock.mockMediaAsks.map((a) => (a.meta as { title: string }).title)).toEqual(['Track 3']);
});

it('leaves a queue another open tab is carrying on alone', async () => {
  localStorage.setItem('fm-media-queue', JSON.stringify([ask(2)]));
  localStorage.setItem(
    'fm-media-queue-owner',
    JSON.stringify({ id: 'another-tab', at: Date.now() }),
  );
  expect(run.resumeQueue()).toBe(0);
  expect(mock.mockMediaAsks).toHaveLength(0);
});

it('takes a queue over once the mark of a stopped app has gone stale', async () => {
  localStorage.setItem('fm-media-queue', JSON.stringify([ask(5)]));
  localStorage.setItem(
    'fm-media-queue-owner',
    JSON.stringify({ id: 'an-app-stopped', at: Date.now() - 20000 }),
  );
  expect(run.resumeQueue()).toBe(1);
  await vi.waitFor(() => expect(mock.mockMediaAsks).toHaveLength(1), { timeout: 8000 });
});
