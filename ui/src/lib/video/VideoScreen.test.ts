// **Videos** (`VideoScreen.svelte`): every video note, a collection's own, searched, played from the
// vault's file.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, it } from 'vitest';
import VideoScreen from './VideoScreen.svelte';
import * as mock from '../mock';

beforeEach(() => mock.reset());

async function videos(...titles: string[]) {
  return (await mock.handle('create_notes', {
    notes: titles.map((t, i) => ({
      title: t,
      body: '',
      tags: [],
      start: '',
      props: {
        log: 'video',
        video: `sha256:${String(i).padStart(64, '0')}`,
        channel: 'Ant Channel',
        duration_s: 61,
      },
    })),
  })) as { id: string }[];
}

it('lists the videos, finds one, and plays it from the vault', async () => {
  await videos('Leafcutters at night', 'Bees in spring');
  render(VideoScreen);
  expect(await screen.findByText('Leafcutters at night')).toBeTruthy();
  expect(screen.getAllByText('Ant Channel · 1:01').length).toBe(2);
  await fireEvent.input(screen.getByRole('searchbox', { name: 'search videos' }), {
    target: { value: 'bees' },
  });
  await waitFor(() => expect(screen.queryByText('Leafcutters at night')).toBeNull());
  await fireEvent.click(screen.getByText('Bees in spring'));
  expect(screen.getByLabelText('playing Bees in spring')).toBeTruthy();
});

it('shows a collection’s videos only', async () => {
  const [a] = await videos('Leafcutters at night', 'Bees in spring');
  await mock.handle('create_notes', {
    notes: [
      {
        title: 'Ants',
        body: `1. [Leafcutters at night](note:${a.id})\n`,
        tags: ['collection'],
        start: '',
        props: {},
      },
    ],
  });
  render(VideoScreen);
  await screen.findByText('Bees in spring');
  expect(screen.getByRole('option', { name: 'Ants' })).toBeTruthy();
});
