// **An open discussion must not full-scan the vault every 1.5 seconds.**
//
// `pollAgent` ran unconditionally on every tick and called `proposal_for`, which on the server is
// `open_proposal_for` → `proposals()` → a filter with no `Text` predicate → `load_all()`: every
// note in every vault, hydrated, plus a libgit2 open per candidate. At a 1.5 s interval that is
// forty full-corpus scans a minute for as long as any discussion is open — and on Android each one
// parks the WebView's JS thread. This is the "lags generally" complaint, and it is the biggest of
// the three by a distance, because unlike the others it costs you the same whether you are typing
// or just looking at the note.
//
// A proposal can only appear or change on two edges: an agent turn starting, and a reply landing.
// So those are what the poll asks on. This budget pins that, and it fails on the unfixed code with
// roughly one call per tick.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { getNote, thread, discussions, proposalFor, agentActivity, onlineAgents, backlinks } =
  vi.hoisted(() => ({
    getNote: vi.fn(),
    thread: vi.fn(),
    discussions: vi.fn(),
    proposalFor: vi.fn(),
    agentActivity: vi.fn(),
    onlineAgents: vi.fn(),
    backlinks: vi.fn(),
  }));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  getNote,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
}));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000077';

const noteDetail = () => ({
  id: ID,
  type: 'note',
  title: 'Something to discuss',
  status: null,
  due: null,
  start: null,
  hard: false,
  created: '2026-08-01T10:00:00Z',
  updated: '2026-08-01T10:00:00Z',
  tags: [],
  assets: [],
  props: {},
  vault: 'personal',
  body: 'A short note with a comment thread.',
  version: 'mock-v1',
});

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

beforeEach(() => {
  getNote.mockResolvedValue(noteDetail());
  thread.mockResolvedValue({ messages: [], count: 0 });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  onlineAgents.mockResolvedValue([]);
  backlinks.mockResolvedValue([]);
  // The ordinary case, and the one that was expensive: nothing is happening.
  agentActivity.mockResolvedValue({ active: false });
});

describe('an open discussion, with an idle agent', () => {
  it('does not ask for the proposal on every poll tick', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await waitFor(() => expect(getNote).toHaveBeenCalled());

    const toggle = await screen.findByRole('button', { name: /Discussion/ });
    await fireEvent.click(toggle);
    await waitFor(() => expect(toggle.getAttribute('aria-expanded')).toBe('true'));

    // Opening the thread legitimately asks once — that is `toggleDiscussion`, so the PR shows
    // straight away rather than on the next tick.
    await waitFor(() => expect(proposalFor).toHaveBeenCalled());
    const onOpen = proposalFor.mock.calls.length;

    // Sit still for five seconds of wall time. That is one tick at the 4 s idle beat and three at
    // the old fixed 1.5 s one, so it discriminates without a long sleep.
    await sleep(5000);

    // The poll definitely ran — otherwise this test would pass by doing nothing, which is the
    // failure mode every budget has to rule out first.
    expect(agentActivity.mock.calls.length).toBeGreaterThan(0);
    // …and nothing happened, so nothing should have re-asked for the proposal. Unfixed this
    // climbs by one per tick, each a full-corpus scan plus a libgit2 open per candidate.
    expect(proposalFor.mock.calls.length).toBe(onOpen);
  });
});

// The *backoff policy* — how far apart the ticks are — is arithmetic, and is tested as arithmetic
// in `pollBeat.test.ts` rather than by sleeping through several beats here. The clock is needed
// for the claim above (that a tick does not drag a full-corpus scan behind it) and for nothing
// else.
