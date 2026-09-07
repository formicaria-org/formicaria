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

  // **One control opens the note, and it is a real button.** The post used to be a `div` with
  // `role="button"`, which made it a leaf in the accessibility tree — everything inside flattened
  // into one name and the status chip within was not separately reachable. A post that holds a
  // discussion and a composer would have been a form inside a button.
  it('opens the note from the title, which is a real button', async () => {
    const onopen = vi.fn();
    const el = render(Timeline, { cards: [note()], onopen } as never);
    const open = el.container.querySelector('.post .open') as HTMLElement;
    expect(open.tagName, 'a real button, not a div wearing a role').toBe('BUTTON');
    await fireEvent.click(open);
    expect(onopen).toHaveBeenCalledWith('01AAA');
    cleanup();
  });

  it('is not itself a button, so what it contains stays reachable', () => {
    const el = render(Timeline, { cards: [note()], onopen: () => {} } as never);
    const post = el.container.querySelector('.post') as HTMLElement;
    expect(post.tagName).toBe('ARTICLE');
    expect(post.getAttribute('role')).toBeNull();
    expect(post.getAttribute('tabindex')).toBeNull();
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

describe('how much of the note a post shows', () => {
  it('prefers the feed excerpt over the one-line preview', () => {
    // `excerpt` is the feed's own field — several lines, char-capped server-side. Only `recent()`
    // sends it, which is why a post has to fall back rather than assume it.
    const el = render(Timeline, {
      props: {
        cards: [note({ excerpt: 'first line\nsecond line\nthird line' })],
        onopen: () => {},
        mode: 'feed',
      },
    });
    const shown = el.container.querySelector('.post .preview')?.textContent ?? '';
    expect(shown).toContain('third line');
    expect(shown, 'the one-line preview is not what a post reads').not.toContain(
      'rough notes on LiFePO4',
    );
    cleanup();
  });

  it('falls back to the preview when no excerpt was sent', () => {
    const el = render(Timeline, {
      props: { cards: [note()], onopen: () => {}, mode: 'feed' },
    });
    expect(el.container.querySelector('.post .preview')?.textContent).toContain(
      'rough notes on LiFePO4',
    );
    cleanup();
  });

  // jsdom applies no CSS, so this cannot say the fade is *visible* — only that the excerpt reached
  // the DOM to be faded. The clamp and the gradient are eyes-on-a-device or they are unverified.
});

describe('the window', () => {
  it('survives the feed being refetched, so a poll beat does not collapse it', async () => {
    // `refresh()` replaces the whole `cards` array on every `changed` beat, and this effect used to
    // key on `cards.length` — so any vault change reset the window to 30. Once you can post a reply
    // from the feed, your own reply would collapse your own window on the next beat.
    const el = render(Timeline, { props: { cards: many(75), onopen: () => {}, mode: 'feed' } });
    // One click opens one more page: 30 → 60.
    await fireEvent.click(screen.getByRole('button', { name: /45 older/ }));
    expect(el.container.querySelectorAll('.post').length).toBe(60);

    // Same notes, a new array — exactly what a refresh hands over.
    await el.rerender({ cards: many(75).map((c) => ({ ...c })), onopen: () => {}, mode: 'feed' });
    expect(
      el.container.querySelectorAll('.post').length,
      'the reader opened this window; a refetch must not close it',
    ).toBe(60);
    cleanup();
  });

  it('still resets when the underlying set changes, so switching vault lands you at the top', async () => {
    const el = render(Timeline, { props: { cards: many(75), onopen: () => {}, mode: 'feed' } });
    await fireEvent.click(screen.getByRole('button', { name: /45 older/ }));
    expect(el.container.querySelectorAll('.post').length).toBe(60);

    const other = many(75).map((c, i) => ({ ...c, id: `other-${i}` }));
    await el.rerender({ cards: other, onopen: () => {}, mode: 'feed' });
    expect(el.container.querySelectorAll('.post').length).toBe(30);
    cleanup();
  });
});

describe('the reply count', () => {
  // The count comes from one `thread_roots` call for the whole feed. `thread()` — the call that
  // reads an actual conversation — is a whole-corpus read, so one per row would be thirty corpus
  // scans behind a single mutex. Knowing a conversation exists is most of the value; opening it is
  // the expensive half, and happens on demand.
  it('shows how many messages a post has, and nothing when it has none', () => {
    const el = render(Timeline, {
      props: {
        cards: [note({ id: 'a' }), note({ id: 'b' })],
        onopen: () => {},
        counts: { a: 3 },
        ontoggle: () => {},
        mode: 'feed',
      },
    });
    const badges = el.container.querySelectorAll('.post .comments');
    expect(badges.length, 'every post can start one').toBe(2);
    expect(badges[0].textContent?.trim()).toBe('3');
    expect(badges[1].textContent?.trim(), 'no conversation, no number').toBe('');
    cleanup();
  });

  it('asks to open one thread, and says which is open', async () => {
    const ontoggle = vi.fn();
    const el = render(Timeline, {
      props: {
        cards: [note({ id: 'a' })],
        onopen: () => {},
        counts: { a: 2 },
        expandedId: 'a',
        ontoggle,
        mode: 'feed',
      },
    });
    const badge = el.container.querySelector('.post .comments') as HTMLElement;
    expect(badge.getAttribute('aria-expanded')).toBe('true');
    await fireEvent.click(badge);
    expect(ontoggle).toHaveBeenCalledWith('a');
    cleanup();
  });

  it('offers no discussion at all in the compact list', () => {
    const el = render(Timeline, {
      props: {
        cards: [note({ id: 'a' })],
        onopen: () => {},
        counts: { a: 3 },
        ontoggle: () => {},
        mode: 'compact',
      },
    });
    expect(el.container.querySelector('.comments')).toBeNull();
    cleanup();
  });
});
