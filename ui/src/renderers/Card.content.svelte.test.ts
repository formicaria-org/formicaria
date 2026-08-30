// **What a card actually says, and what stops it saying too much.**
//
// A card rendered `title ?? preview`, so the same note carried strictly less on a board than in
// the Timeline or Search row beside it — and a note whose body is not prose put a fragment of that
// body on screen as its only line. Neither field is length-capped on the way in, so showing both
// only works with a clamp.
//
// The hue split is pinned here too because it is invisible and would regress silently: the
// component hands out the *raw hash* and the theme decides the colour from it. If the component
// ever sets `--hue` directly, the named exceptions in `app.css` can never fire — an inline style
// beats every selector — and the finished-status exception quietly stops applying.

import { describe, expect, it } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import Card from './Card.svelte';
import type { ObjectMeta } from '../lib/types';

const card = (over: Partial<ObjectMeta> = {}): ObjectMeta =>
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
    ...over,
  }) as ObjectMeta;

describe('what a card shows', () => {
  it('shows the title and the preview, not one or the other', () => {
    render(Card, { card: card(), onopen: () => {}, statuses: [] } as never);
    expect(screen.getByText('Battery chemistry')).toBeTruthy();
    expect(screen.getByText('rough notes on LiFePO4 cells')).toBeTruthy();
    cleanup();
  });

  it('falls back to the preview when there is no title', () => {
    render(Card, { card: card({ title: null }), onopen: () => {}, statuses: [] } as never);
    expect(screen.getByText('rough notes on LiFePO4 cells')).toBeTruthy();
    cleanup();
  });

  it('says "Untitled" rather than showing an id when a note has neither', () => {
    // The reachable case is a whiteboard: its body is Excalidraw JSON, so the old fallback put
    // `{"type":"excalidraw"…` on the card as its one line of text.
    render(Card, { card: card({ title: null, preview: undefined }), onopen: () => {}, statuses: [] } as never);
    expect(screen.getByText('Untitled')).toBeTruthy();
    expect(screen.queryByText('01AAA')).toBeNull();
    cleanup();
  });

  it('does not repeat the filename under the title of an ingested file', () => {
    const el = render(Card, {
      card: card({ type: 'asset', title: 'scan.pdf', preview: 'scan.pdf' }),
      onopen: () => {},
      statuses: [],
    } as never);
    expect(el.container.querySelectorAll('.preview').length).toBe(0);
    cleanup();
  });
});

describe('status colour', () => {
  it('hands the theme a raw hash, never a decided hue', () => {
    const el = render(Card, {
      card: card({ status: 'blocked' }),
      onopen: () => {},
      statuses: ['blocked'],
      onstatus: () => {},
    } as never);
    const chip = el.container.querySelector('.status-chip') as HTMLElement;
    const style = chip.getAttribute('style') ?? '';
    expect(style).toContain('--hash-hue');
    // The whole point: the theme's per-value exceptions must still be able to win.
    expect(style).not.toMatch(/--hue\s*:/);
    cleanup();
  });

  it('gives a status nobody hardcoded a colour anyway', () => {
    const el = render(Card, {
      card: card({ status: 'drafting' }),
      onopen: () => {},
      statuses: ['drafting'],
      onstatus: () => {},
    } as never);
    const chip = el.container.querySelector('.status-chip') as HTMLElement;
    expect(chip.getAttribute('style')).toMatch(/--hash-hue:\s*\d+/);
    expect(chip.getAttribute('data-value')).toBe('drafting');
    cleanup();
  });
});
