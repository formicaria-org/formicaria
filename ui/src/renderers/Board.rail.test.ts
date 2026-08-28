// **The column you cannot see, and could not know was there.**
//
// The board is one horizontally scrolling strip. In a narrow pane a single column fills the width
// and the strip snaps between them, so the second, third and fourth columns are off-screen by
// construction — with nothing to say they exist. The owner reported a column "not showing up"
// (2026-08-24); a filtered saved view turned out to be the cause that day, but this is the other
// way a board can lose a column in plain sight, and it needed answering too.
//
// The rail is the map: every column by name and card count, the one you are on marked, a click to
// jump. What CI can check is exactly that — the entries, their names, where a click goes.
// **Whether the rail is visible is a layout question, and layout is what jsdom cannot answer**
// (it applies no CSS and measures nothing: `scrollWidth`/`getBoundingClientRect` are all zero, and
// the container query never runs). So the rail is always rendered and CSS decides when it shows;
// that split is what leaves anything here testable at all.
//
// Column labels are arbitrary runtime strings on purpose — this renderer never learns what one
// means, and the CI grep over `ui/src/renderers` would fail the build if a status literal appeared.

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import Board from './Board.svelte';
import type { ObjectMeta } from '../lib/types';

const card = (id: string): ObjectMeta =>
  ({
    id,
    type: 'note',
    title: `note ${id}`,
    preview: 'x',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-08-24T10:00:00Z',
    updated: '2026-08-24T10:00:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: '',
  }) as unknown as ObjectMeta;

const board = {
  group_by: 'stage',
  columns: [
    { value: 'a', label: 'Backlog', cards: [card('01A')] },
    { value: 'b', label: 'Review', cards: [card('01B'), card('01C')] },
    { value: 'c', label: 'Shipped', cards: [] },
    { value: 'd', label: 'Archive', cards: [card('01D')] },
  ],
};

const props = {
  board,
  onmove: () => {},
  onreorder: () => {},
  onopen: () => {},
  statuses: [],
  onstatus: () => {},
};

// jsdom implements no scrolling at all, so `scrollIntoView` is simply absent. The component calls
// it optionally for that reason; here it is a spy, which is the only part of "jump to a column"
// that can be observed without a layout engine.
const scrollIntoView = vi.fn();
beforeEach(() => {
  cleanup();
  scrollIntoView.mockClear();
  Element.prototype.scrollIntoView = scrollIntoView;
});

describe('the board rail', () => {
  it('names every column, with its card count, so an off-screen one is still findable', () => {
    render(Board, { props });
    const rail = screen.getByRole('navigation', { name: /columns on this board/i });
    const stops = rail.querySelectorAll('button');
    expect(stops.length).toBe(4);
    // Name then count. The separator and the spacing between them are CSS's business (the pill is
    // a flex row with a gap), so they are normalised away rather than asserted on.
    const read = (b: Element) => b.textContent?.replace(/[\s·]+/g, ' ').trim();
    expect([...stops].map(read)).toEqual(['Backlog 1', 'Review 2', 'Shipped 0', 'Archive 1']);
  });

  it('jumps to the column you pick, and marks it as where you are', async () => {
    render(Board, { props });
    const rail = screen.getByRole('navigation', { name: /columns on this board/i });
    const stops = [...rail.querySelectorAll('button')];

    // The first is where you start — the strip is at its left edge.
    expect(stops[0].getAttribute('aria-current')).toBe('true');

    await fireEvent.click(stops[2]);

    expect(scrollIntoView).toHaveBeenCalledTimes(1);
    // Scrolled the *column*, not the rail entry — the element whose `.column-label` is that name.
    const target = scrollIntoView.mock.instances[0] as HTMLElement;
    expect(target.querySelector('.column-label')?.textContent).toBe('Shipped');
    expect(target.className).toContain('column');

    expect(stops[2].getAttribute('aria-current')).toBe('true');
    expect(stops[0].getAttribute('aria-current')).toBeNull();
  });

  it('has no rail when there is only one column — a map of one place is noise', () => {
    render(Board, { props: { ...props, board: { group_by: 'stage', columns: [board.columns[0]] } } });
    expect(screen.queryByRole('navigation', { name: /columns on this board/i })).toBeNull();
  });
});
