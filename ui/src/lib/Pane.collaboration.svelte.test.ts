// The Collaboration view refreshes whenever the vault moves — every ~15 s while anything changes. An
// opened proposal must ride that out: not blank itself to "Loading…" (the flicker the owner saw every
// 15–20 s), and not re-record that it was shown, which the server counts as a change and which so fed
// the next refresh, forever.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { ObjectMeta, ProposalContent } from './types';

const calls = { content: 0, shown: 0 };
vi.mock('./ipc', async (orig) => ({
  ...(await orig<typeof import('./ipc')>()),
  proposalDiff: vi.fn(async () => ({ exists: true, declined: false, files: [], patch: '' })),
  proposalContent: vi.fn(async (): Promise<ProposalContent> => {
    calls.content++;
    return { host: 'N1', title: 'A meeting', body: 'the proposed text' };
  }),
  proposalShown: vi.fn(async () => {
    calls.shown++;
  }),
}));

const { default: Pane } = await import('./Pane.svelte');

const card = (): ObjectMeta =>
  ({
    id: 'P1',
    vault: 'v',
    title: 'Proposal: A meeting',
    tags: [],
    props: {},
  }) as unknown as ObjectMeta;
const noop = () => {};

describe('the Collaboration view across a refresh', () => {
  it('keeps an opened proposal on screen and records its showing once', async () => {
    const props = {
      pane: { id: 1, kind: 'collaboration', colSpan: 1, rowSpan: 1 },
      feed: { cards: [card()], conflicts: [] },
      index: 0,
      cols: 1,
      statuses: [],
      shown: () => true,
      focused: true,
      startEditing: false,
      vaults: ['v'],
      onopen: noop,
      onresolve: async () => {},
      onmove: noop,
      onstatus: noop,
      onnavigate: noop,
      onsaved: noop,
      onchange: noop,
      onreorder: noop,
      onresize: noop,
      onclose: noop,
      onfocus: noop,
      headed: false,
    };
    const { rerender } = render(Pane, props as never);
    await fireEvent.click(screen.getByText('Proposal: A meeting'));
    await screen.findByLabelText('proposed note body');
    expect(calls).toEqual({ content: 1, shown: 1 });

    // A refresh: the same proposal, as a new object — exactly what `refresh()` hands down.
    for (let i = 0; i < 3; i++) {
      await rerender({ ...props, feed: { cards: [card()], conflicts: [] } } as never);
      expect(screen.queryByText(/Loading the proposed change/)).toBeNull();
      expect(screen.getByLabelText('proposed note body')).toBeTruthy();
    }
    expect(calls).toEqual({ content: 1, shown: 1 });
  });
});
