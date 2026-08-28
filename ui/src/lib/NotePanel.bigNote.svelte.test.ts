// **Typing in a note must not re-ask the server about the note.**
//
// The owner's report: the Android app "freezes with big notes". The size turns out to be a red
// herring — what actually happens is that `save()` reassigns the `note` signal every 500 ms, and
// two `$effect`s read `note?.id`, which in Svelte 5 subscribes them to the *whole* signal rather
// than to the id. So every autosave re-fired both, and one of them (`backlinks`) has no `Text`
// predicate on the server, so it falls back to `load_all()`: every note in every vault, hydrated
// and body-scanned, under the vault lock. On a phone, where every command parks the JS thread,
// that reads as a freeze on exactly the notes you type in most.
//
// **These are work budgets, not timings.** Wall-clock in jsdom measures the CI machine, not the
// phone; the number of commands a keystroke causes is the same everywhere and is the thing that
// actually hurts. Both tests below fail on the unfixed code — the first with 4 `backlinks` calls
// instead of 1, the second because the comment thread closes itself mid-typing.
//
// Real timers, deliberately: Testing Library's `findBy*` polls on real time, so `vi.useFakeTimers`
// deadlocks against it. The debounce is 500 ms and there are only a handful of waits.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backlinks, getNote, updateBody, thread, discussions, proposalFor, agentActivity } =
  vi.hoisted(() => ({
    backlinks: vi.fn(),
    getNote: vi.fn(),
    updateBody: vi.fn(),
    thread: vi.fn(),
    discussions: vi.fn(),
    proposalFor: vi.fn(),
    agentActivity: vi.fn(),
  }));
// Spread the real module and override only what this test observes — a hand-listed factory drops
// whatever else the import graph needs and fails as "no tests collected".
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backlinks,
  getNote,
  updateBody,
  thread,
  discussions,
  proposalFor,
  agentActivity,
}));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000042';

/// A note big enough to be the one the owner means. ~200 KB is a long research note, not a
/// pathological one — and the point is that size should be irrelevant to the call count, so a big
/// body here proves the budget does not scale with it.
const BIG_BODY = Array.from(
  { length: 3000 },
  (_, i) => `${i}. The advantage estimate leaks across the meta-update boundary.`,
).join('\n');

const noteDetail = (body: string) => ({
  id: ID,
  type: 'note',
  title: 'A long note',
  status: 'doing',
  due: null,
  start: null,
  hard: false,
  created: '2026-08-01T10:00:00Z',
  updated: '2026-08-01T10:00:00Z',
  tags: [],
  assets: [],
  props: {},
  vault: 'personal',
  body,
  version: 'mock-v1',
});

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

beforeEach(() => {
  getNote.mockResolvedValue(noteDetail(BIG_BODY));
  backlinks.mockResolvedValue([]);
  updateBody.mockResolvedValue('mock-v2');
  thread.mockResolvedValue({ messages: [] });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  agentActivity.mockResolvedValue({ active: false });
});

/// Open the body editor. There is no Edit button any more — it lives in the options popover.
async function openEditor() {
  await fireEvent.click(await screen.findByLabelText('note options'));
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
  return screen.findByLabelText('note body (Markdown)');
}

/// Type, then wait out the 500 ms debounce, `times` times over.
async function typeAndSave(el: HTMLElement, times: number) {
  for (let i = 0; i < times; i += 1) {
    await fireEvent.input(el, { target: { value: `${BIG_BODY}\nedit ${i}` } });
    await sleep(650);
  }
}

describe('editing a large note', () => {
  it('asks for backlinks once per note, not once per autosave', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });

    await waitFor(() => expect(backlinks).toHaveBeenCalledTimes(1));
    const body = await openEditor();
    await typeAndSave(body, 3);

    // Three saves happened…
    await waitFor(() => expect(updateBody).toHaveBeenCalledTimes(3));
    // …and the note never changed, so nothing should have re-asked who links to it.
    //
    // Unfixed this is 4 — one extra per save, each a full-corpus scan on the server, under the
    // vault lock. Asserted as exactly `1` rather than `<= 2`: a budget with slack in it is a
    // budget that drifts back.
    expect(backlinks).toHaveBeenCalledTimes(1);
  });

  it('keeps an open comment thread open while the body is edited', async () => {
    // **A correctness bug, not a cost one.** The discussion effect also read `note?.id`, and on an
    // ordinary note its `else` branch runs `discOpen = false`. So it did not merely re-run on
    // every autosave — it *closed the comment thread the user had just opened*, mid-typing, and
    // tore down the agent poll with it. Nothing in the suite could see this, because no test had
    // ever typed in a note with its thread open.
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await waitFor(() => expect(getNote).toHaveBeenCalled());

    const toggle = await screen.findByRole('button', { name: /Discussion/ });
    await fireEvent.click(toggle);
    await waitFor(() => expect(toggle.getAttribute('aria-expanded')).toBe('true'));

    const body = await openEditor();
    await typeAndSave(body, 2);
    await waitFor(() => expect(updateBody).toHaveBeenCalledTimes(2));

    // Still open. Unfixed, the first autosave flips this back to "false" under the user.
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
  });
});
