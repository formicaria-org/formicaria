// **A checkbox you can see must be the checkbox you flip.**
//
// `toggleTask` finds the tapped box's ordinal among the rendered checkboxes and flips the byte of
// the marker at that ordinal in the *source*. Its own comment says it bails when "the counts ever
// disagree" — but the guard it shipped with only checked that the index was in range, which says
// nothing when the two lists are different lengths. Then the Nth box is not the Nth marker, and a
// tap writes the wrong byte and autosaves it: a silent edit to a line the user never touched.
//
// A conflicted note is exactly that shape, and it is not exotic. `=======` is a CommonMark setext
// underline, so it turns the line above it into a heading — swallowing that task line's `- [ ]` into
// heading text, where `marked` renders no checkbox at all. The source still has the marker.
// Sentence-granular conflict markers (`decisions.md`, 2026-09-08, *a conflict marks the sentence,
// not the paragraph*) put that shape on any task line long enough to hold two sentences, which is
// what turned a latent edge into one worth a test.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  getNote,
  updateBody,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
} = vi.hoisted(() => ({
  getNote: vi.fn(),
  updateBody: vi.fn(),
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
  updateBody,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
}));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000042';

// Two task markers in the source. The first is inside the conflict, so it does not render as a box.
const CONFLICTED = [
  '- [ ] Do this. ',
  '<<<<<<< ours',
  'And that, ours. ',
  '=======',
  'And that, theirs. ',
  '>>>>>>> theirs',
  '- [ ] Second task.',
].join('\n');

beforeEach(() => {
  vi.clearAllMocks();
  getNote.mockResolvedValue({
    id: ID,
    type: 'note',
    title: 'Conflicted list',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-09-08T10:00:00Z',
    updated: '2026-09-08T10:00:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: 'personal',
    body: CONFLICTED,
    version: 'mock-v1',
  });
  updateBody.mockResolvedValue('mock-v2');
  thread.mockResolvedValue({ messages: [], count: 0 });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  onlineAgents.mockResolvedValue([]);
  backlinks.mockResolvedValue([]);
  agentActivity.mockResolvedValue({ active: false });
});

describe('a task list inside a conflict', () => {
  it('refuses to toggle when the read view and the source disagree about how many tasks there are', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await waitFor(() => expect(getNote).toHaveBeenCalled());

    // The premise: the source has two `- [ ]` markers, and the read view shows fewer, because the
    // first one was swallowed by the setext heading that `=======` makes of the line above it.
    // If this ever stops being true the test below proves nothing, so it is asserted, not assumed.
    const boxes = await waitFor(() => {
      const found = screen.queryAllByRole('checkbox');
      expect(found.length).toBeGreaterThan(0);
      return found;
    });
    expect(boxes.length).toBeLessThan(2);

    await fireEvent.click(boxes[0]);
    // Nothing is written. The alternative is flipping the byte of a task the user cannot see.
    await new Promise((r) => setTimeout(r, 50));
    expect(updateBody).not.toHaveBeenCalled();
  });
});
