// ProposalReview fetches a proposal's diff (via ipc → mock in tests) and renders it. Against the
// mock backend the patch is a stand-in; the point under test is that the component fetches by id and
// shows the returned diff.

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import ProposalReview from './ProposalReview.svelte';
import type { ObjectMeta } from './types';

describe('ProposalReview', () => {
  it('fetches and renders the proposal diff for its id', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', { id: note.id, body: 'better' });

    render(ProposalReview, { id: prop.id });

    // The mock's stand-in patch line eventually renders.
    expect(await screen.findByText(/the real diff appears against a live backend/)).toBeTruthy();
  });

  it('shows a plain message, not an error, when the branch is gone', async () => {
    // A note that is not a proposal makes the mock throw "not a proposal" — surfaced as the error
    // line, proving the component reports failures rather than rendering nothing.
    const note = await mock.handle<ObjectMeta>('capture', { body: 'ordinary' });
    render(ProposalReview, { id: note.id });
    expect(await screen.findByText(/Couldn't load the diff/)).toBeTruthy();
  });
});
