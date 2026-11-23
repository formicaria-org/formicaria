// **The red dot on Collaboration** (2026-11-23): there while a proposal is open and never looked at,
// gone once it is looked at, accepted, declined or closed.
import { render, screen } from '@testing-library/svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import * as mock from './mock';
import { resetWaiting, waitingProposal } from './waiting.svelte';
import WaitingDot from './WaitingDot.svelte';
import type { ObjectMeta } from './types';

beforeEach(() => {
  mock.reset();
  resetWaiting();
});

const p = (props: Record<string, unknown>) => ({ props }) as unknown as ObjectMeta;

it('a proposal waits while it is open and was never shown', () => {
  expect(waitingProposal(p({ proposes: 'branch:proposal/a' }))).toBe(true);
  expect(waitingProposal(p({ proposes: 'branch:proposal/a', shown: '2026-11-23T10:00Z' }))).toBe(
    false,
  );
  expect(waitingProposal(p({ proposes: 'branch:proposal/a', declined: true }))).toBe(false);
  expect(waitingProposal(p({ proposes: 'branch:proposal/a', accepted: '2026-11-23' }))).toBe(false);
  expect(waitingProposal(p({ proposes: 'branch:proposal/a', closed: 'true' }))).toBe(false);
});

it('the dot says how many wait, and goes when they are looked at', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Proposal: Lab meeting', body: '', props: { proposes: 'branch:proposal/x' } }],
  })) as { id: string }[];
  const first = render(WaitingDot);
  expect(await screen.findByRole('img', { name: '1 proposal to look at' })).toBeTruthy();
  first.unmount();
  await mock.handle('proposal_shown', { id: made.id });
  resetWaiting();
  render(WaitingDot);
  // Read again: nothing waits, so no dot.
  await new Promise((r) => setTimeout(r, 50));
  expect(screen.queryAllByRole('img', { name: /to look at/ })).toHaveLength(0);
});
