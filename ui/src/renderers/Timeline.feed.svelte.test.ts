// **The timeline as a feed.**
//
// A timeline of titles answers "what did I write". A feed answers "what has been happening",
// which is the question a shared vault raises: every collaborator's notes in one stream, each post
// saying which vault it came from and who last touched it.
//
// Two things here are load-bearing beyond looks. The post asks for a **thumbnail** — until this
// existed nothing in the app ever did, and `render.ts` puts the cost of the alternative at "~50 MB
// of decoded pixels" for a single 12 MP photo, which a feed would multiply by the screen. And the
// list is **windowed**, because `recent` is unbounded and there is no virtualisation anywhere.

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import Timeline from './Timeline.svelte';
import type { ObjectMeta } from '../lib/types';

const note = (over: Partial<ObjectMeta> = {}): ObjectMeta =>
  ({
    id: '01AAA',
    type: 'note',
    title: 'Battery chemistry',
    preview: 'rough notes on LiFePO4 cells',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-07-18T10:00:00Z',
    updated: '2026-07-18T10:00:00Z',
    tags: [],
    assets: [],
    vault: 'lab',
    ...over,
  }) as ObjectMeta;

const many = (n: number) =>
  Array.from({ length: n }, (_, i) => note({ id: `note-${i}`, title: `Note ${i}` }));

describe('a post', () => {
  it('shows the note picture as a thumbnail, never the full blob', () => {
    const el = render(Timeline, {
      cards: [note({ assets: ['sha256:abc'] })],
      onopen: () => {},
    } as never);
    const img = el.container.querySelector('img') as HTMLImageElement;
    expect(img).toBeTruthy();
    // The point of the whole first half of this work.
    expect(img.getAttribute('src')).toContain('kind=thumb');
    // Native, no library: the guard against a screen full of decoding at once.
    expect(img.getAttribute('loading')).toBe('lazy');
    expect(img.getAttribute('decoding')).toBe('async');
    cleanup();
  });

  it('describes the picture instead of leaving it unlabelled', () => {
    const el = render(Timeline, {
      cards: [note({ assets: ['sha256:abc'], title: 'Whiteboard' })],
      onopen: () => {},
    } as never);
    expect((el.container.querySelector('img') as HTMLImageElement).alt).toMatch(/Whiteboard/);
    cleanup();
  });

  it('has no picture frame at all when the note has no picture', () => {
    // A text note must not be padded out with an empty box to look like the others.
    const el = render(Timeline, { cards: [note()], onopen: () => {} } as never);
    expect(el.container.querySelector('.shot')).toBeNull();
    expect(el.container.querySelector('img')).toBeNull();
    cleanup();
  });

  it('names the vault and the note, which is what makes a shared stream readable', () => {
    render(Timeline, { cards: [note({ vault: 'lab' })], onopen: () => {} } as never);
    expect(screen.getByText('Battery chemistry')).toBeTruthy();
    expect(screen.getByText('rough notes on LiFePO4 cells')).toBeTruthy();
    expect(screen.getByText('lab')).toBeTruthy();
    cleanup();
  });

  it('opens the note when the post is clicked', async () => {
    const onopen = vi.fn();
    const el = render(Timeline, { cards: [note()], onopen } as never);
    await fireEvent.click(el.container.querySelector('.post') as HTMLElement);
    expect(onopen).toHaveBeenCalledWith('01AAA');
    cleanup();
  });
});

describe('the window', () => {
  it('caps what it mounts, and says how much is left', () => {
    const el = render(Timeline, { cards: many(75), onopen: () => {} } as never);
    expect(el.container.querySelectorAll('.post').length).toBe(30);
    expect(screen.getByRole('button', { name: /45 older/ })).toBeTruthy();
    cleanup();
  });

  it('grows when asked', async () => {
    const el = render(Timeline, { cards: many(75), onopen: () => {} } as never);
    await fireEvent.click(screen.getByRole('button', { name: /Show more/ }));
    expect(el.container.querySelectorAll('.post').length).toBe(60);
    cleanup();
  });

  it('offers nothing to expand when everything already fits', () => {
    render(Timeline, { cards: many(5), onopen: () => {} } as never);
    expect(screen.queryByRole('button', { name: /Show more/ })).toBeNull();
    cleanup();
  });
});

describe('the compact list is untouched', () => {
  it('renders rows, no posts and no pictures', () => {
    const el = render(Timeline, {
      cards: [note({ assets: ['sha256:abc'] })],
      onopen: () => {},
      mode: 'compact',
    } as never);
    expect(el.container.querySelectorAll('.post').length).toBe(0);
    expect(el.container.querySelectorAll('.row').length).toBe(1);
    expect(el.container.querySelector('img')).toBeNull();
    cleanup();
  });

  it('is not windowed — it does what it always did', () => {
    const el = render(Timeline, { cards: many(75), onopen: () => {}, mode: 'compact' } as never);
    expect(el.container.querySelectorAll('.row').length).toBe(75);
    cleanup();
  });
});
