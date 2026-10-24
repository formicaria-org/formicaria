// **Closed proposals, like closed pull requests** (`decisions.md#git`, 2026-10-24): Collaboration
// lists the open ones first and the closed ones below, folded, newest first, each saying how it
// ended; opened, a closed one shows what it did, read-only.

import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { ObjectMeta, ProposalDiff } from './types';

vi.mock('./appTime', async (orig) => {
  const m = await orig<typeof import('./appTime')>();
  m.setAppZone('UTC');
  return m;
});
vi.mock('./ipc', async (orig) => ({
  ...(await orig<typeof import('./ipc')>()),
  proposalDiff: vi.fn(async (id: string): Promise<ProposalDiff> =>
    id === 'P2'
      ? {
          exists: false,
          declined: false,
          files: ['notes/N2.md'],
          patch: '@@ -1 +1 @@\n-old line\n+the accepted line\n',
          state: 'merged',
          closed_at: '2026-10-20T08:00:00Z',
          why: null,
          shown_from_history: true,
        }
      : {
          exists: false,
          declined: true,
          files: [],
          patch: '',
          state: 'declined',
          closed_at: '2026-10-22T09:00:00Z',
          why: 'too long',
          shown_from_history: false,
        },
  ),
  proposalShown: vi.fn(async () => {}),
}));

const { default: Pane } = await import('./Pane.svelte');

const card = (id: string, title: string, props: Record<string, unknown>, created: string) =>
  ({ id, vault: 'v', title, tags: [], props, created }) as unknown as ObjectMeta;
const noop = () => {};

function show(cards: ObjectMeta[]) {
  return render(Pane, {
    pane: { id: 1, kind: 'collaboration', colSpan: 1, rowSpan: 1 },
    feed: { cards, conflicts: [] },
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
  } as never);
}

describe('closed proposals', () => {
  const cards = [
    card('P1', 'Proposal: still open', {}, '2026-10-23T10:00:00Z'),
    card(
      'P2',
      'Proposal: merged one',
      { accepted: '2026-10-20T08:00:00Z' },
      '2026-10-19T10:00:00Z',
    ),
    card(
      'P3',
      'Proposal: declined one',
      { declined: 'true', declined_at: '2026-10-22T09:00:00Z', declined_why: 'too long' },
      '2026-10-18T10:00:00Z',
    ),
  ];

  it('lists the open ones first and the closed ones apart, newest first, with how they ended', () => {
    show(cards);
    const open = screen.getByText('Open proposals').closest('section')!;
    expect(within(open).getByText('Proposal: still open')).toBeTruthy();
    expect(within(open).queryByText('Proposal: merged one')).toBeNull();
    const closed = screen.getByText('Closed · 2').closest('details')!;
    const titles = within(closed)
      .getAllByText(/^Proposal: /)
      .map((e) => e.textContent);
    expect(titles).toEqual(['Proposal: declined one', 'Proposal: merged one']);
    expect(within(closed).getByText('Declined')).toBeTruthy();
    expect(within(closed).getByText('Merged')).toBeTruthy();
    expect(within(closed).getByText('22 Oct 2026')).toBeTruthy();
  });

  it('shows what a merged one changed, and a declined one its reason', async () => {
    show(cards);
    await fireEvent.click(screen.getByText('Proposal: merged one'));
    expect(await screen.findByText(/accepted into the note/)).toBeTruthy();
    expect(screen.getByText(/What it changed/)).toBeTruthy();
    expect(screen.getByText('+the accepted line')).toBeTruthy();
    await fireEvent.click(screen.getByText('Proposal: declined one'));
    expect(await screen.findByText('Why: “too long”')).toBeTruthy();
    expect(screen.getByText(/not kept on this device/)).toBeTruthy();
  });

  it('says there are no open proposals when every one is closed', () => {
    show(cards.slice(1));
    expect(screen.getByText('No open proposals.')).toBeTruthy();
    expect(screen.getByText('Closed · 2')).toBeTruthy();
  });
});
