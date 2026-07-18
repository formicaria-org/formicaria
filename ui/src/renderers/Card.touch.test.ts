// The board's touch path, tested without a touchscreen.
//
// Dragging a card uses `@atlaskit/pragmatic-drag-and-drop`'s **element** adapter, which is
// HTML5 drag and does not fire on touch at all — and the package ships no pointer adapter
// to swap in. So on a phone the board would simply be inert. The tap→move menu is the stand-in.
//
// It is deliberately built from real `<button>`s rather than touch handlers, which is what
// makes it testable here: a click in jsdom exercises the same code path a tap does, so the
// thing CI genuinely cannot check is narrowed to "is the target big enough for a finger"
// rather than "does moving a card work at all".

import { describe, expect, it, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import Card from './Card.svelte';
import type { ObjectMeta } from '../lib/types';

const card = (over: Partial<ObjectMeta> = {}): ObjectMeta =>
  ({
    id: '01AAA',
    type: 'note',
    title: 'a note',
    preview: 'a note',
    status: 'left',
    due: null,
    start: null,
    hard: false,
    created: '2026-07-18T10:00:00Z',
    updated: '2026-07-18T10:00:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: '',
    ...over,
  }) as unknown as ObjectMeta;

// Two columns whose labels are arbitrary runtime values — the point being that this
// renderer never learns what they mean. If a status literal were needed here, the CI grep
// over `ui/src/renderers` would be the thing telling us the design had slipped.
const columns = [
  { value: 'left', label: 'Left' },
  { value: 'right', label: 'Right' },
];

describe('Card move-to-column menu', () => {
  it('moves a card to the column you pick', async () => {
    const onmoveto = vi.fn();
    const { getByLabelText } = render(Card, {
      props: { card: card(), onopen: vi.fn(), columns, column: 'left', onmoveto },
    });

    await getByLabelText('Move this card to another column').click();
    await screen.getByText('Right').click();

    expect(onmoveto).toHaveBeenCalledWith('01AAA', 'right');
    cleanup();
  });

  // The card itself is a button that opens the note. If the menu's clicks climbed to it,
  // every move would also open what it moved.
  it('does not open the note when you use the menu', async () => {
    const onopen = vi.fn();
    const { getByLabelText } = render(Card, {
      props: { card: card(), onopen, columns, column: 'left', onmoveto: vi.fn() },
    });

    await getByLabelText('Move this card to another column').click();
    await screen.getByText('Right').click();

    expect(onopen).not.toHaveBeenCalled();
    cleanup();
  });

  it('offers no move to the column the card is already in', async () => {
    const { getByLabelText } = render(Card, {
      props: { card: card(), onopen: vi.fn(), columns, column: 'left', onmoveto: vi.fn() },
    });

    await getByLabelText('Move this card to another column').click();

    // Plain DOM properties rather than jest-dom matchers — not worth a dependency for two
    // assertions, and this says the same thing.
    expect((screen.getByText('Left') as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByText('Right') as HTMLButtonElement).disabled).toBe(false);
    cleanup();
  });

  // A board grouped into one column has nowhere to move to, and an offer that does nothing
  // is worse than no offer.
  it('is absent when there is nowhere to move', () => {
    render(Card, {
      props: {
        card: card(),
        onopen: vi.fn(),
        columns: [{ value: 'only', label: 'Only' }],
        column: 'only',
        onmoveto: vi.fn(),
      },
    });

    expect(screen.queryByLabelText('Move this card to another column')).toBeNull();
    cleanup();
  });

  // A view that passes no `onmoveto` (anything that is not a board) gets no button.
  it('is absent when the view offers no move', () => {
    render(Card, { props: { card: card(), onopen: vi.fn() } });

    expect(screen.queryByLabelText('Move this card to another column')).toBeNull();
    cleanup();
  });
});
