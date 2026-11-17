// **Save media** (`SaveMedia.svelte`): a page's media listed; the one chosen looked at; the sound as
// MP3 into a new playlist; the job shown to its end, with the note it became; a protected one said so.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, it } from 'vitest';
import SaveMedia from './SaveMedia.svelte';
import * as mock from './mock';

beforeEach(() => mock.reset());

const candidates = [
  {
    url: 'https://www.youtube.com/watch?v=AbCdEf12345',
    kind: 'player' as const,
    label: 'YouTube video: Leafcutters',
  },
  {
    url: 'https://news.example/ants',
    kind: 'page' as const,
    label: 'The page itself: Ants march on',
  },
];

it('keeps the sound of an embedded video as MP3, in a new playlist, and opens the song', async () => {
  let opened = '';
  render(SaveMedia, {
    page: 'https://news.example/ants',
    candidates,
    onopen: (id: string) => (opened = id),
  });
  expect(await screen.findByText(/The first save fetches the media tools/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('radio', { name: 'YouTube video: Leafcutters' }));
  expect(await screen.findByText('Leafcutters at night')).toBeTruthy();
  await fireEvent.click(screen.getByRole('radio', { name: 'The sound as MP3' }));
  expect((screen.getByRole('radio', { name: 'Music' }) as HTMLInputElement).checked).toBe(true);
  await fireEvent.change(screen.getByRole('combobox', { name: 'MP3 quality' }), {
    target: { value: '320' },
  });
  await fireEvent.change(screen.getByRole('combobox', { name: 'playlist' }), {
    target: { value: '+' },
  });
  await fireEvent.input(screen.getByRole('textbox', { name: "the new list's name" }), {
    target: { value: 'Road' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(await screen.findByText('Saved in Music')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Open' }));
  const song = (await mock.handle('get', { id: opened })) as { props: Record<string, unknown> };
  expect(song.props.log).toBe('music');
});

it('a video goes to Videos at the height chosen', async () => {
  render(SaveMedia, { page: 'https://news.example/ants', candidates: [candidates[0]] });
  await screen.findByText('Leafcutters at night');
  await fireEvent.click(screen.getByRole('radio', { name: 'The video' }));
  expect((screen.getByRole('radio', { name: 'Videos' }) as HTMLInputElement).checked).toBe(true);
  expect(screen.getByRole('combobox', { name: 'video quality' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(await screen.findByText('Saved in Videos')).toBeTruthy();
});

it('says plainly when the media is protected', async () => {
  render(SaveMedia, {
    page: 'https://stream.example/',
    candidates: [{ url: 'https://stream.example/drm/1', kind: 'video', label: 'Video: 1' }],
  });
  expect(await screen.findByRole('alert')).toHaveProperty(
    'textContent',
    'This media is protected (DRM): it cannot be kept.',
  );
  expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
});

it('on the phone: the app’s own runner fetches, and the core files it as a song', async () => {
  const g = globalThis as Record<string, unknown>;
  const jobs: Record<string, Record<string, unknown>> = {};
  let started: string[] = [];
  g.__fmMedia = {
    workDir: () => '/data/cache/media',
    probe: (id: string) => (jobs[id] = { id, state: 'probed', probe: '{"title":"x"}', error: '' }),
    start: (id: string, _url: string, plan: string) => {
      started = (JSON.parse(plan) as { runs: string[][] }).runs.flat();
      jobs[id] = {
        id,
        state: 'fetched',
        progress: 1,
        title: 'Leafcutters at night',
        file: '/data/cache/media/1/media.m4a',
        info: '{"title":"Leafcutters at night"}',
        error: '',
      };
    },
    jobs: () => JSON.stringify(Object.values(jobs)),
    cancel: () => {},
    forget: (id: string) => delete jobs[id],
  };
  try {
    render(SaveMedia, { page: 'https://news.example/ants', candidates: [candidates[0]] });
    await screen.findByText('Leafcutters at night');
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('Saved in Music', {}, { timeout: 4000 })).toBeTruthy();
    expect(started.some((a) => a.startsWith('/data/cache/media/'))).toBe(true);
    const songs = (await mock.handle('music_library', {})) as { title: string }[];
    expect(songs.some((s) => s.title === 'Leafcutters at night')).toBe(true);
    expect(Object.keys(jobs)).toEqual([]);
  } finally {
    delete g.__fmMedia;
  }
});
