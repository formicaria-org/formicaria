// ProposalReview fetches a proposal's diff (via ipc → mock in tests) and renders it. Against the
// mock backend the patch is a stand-in; the point under test is that the component fetches by id and
// shows the returned diff.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
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

  it('accepts (merges) a proposal from the button and tells the parent', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', { id: note.id, body: 'better' });
    const onaccepted = vi.fn();

    render(ProposalReview, { id: prop.id, onaccepted });
    // Wait for the diff (and thus the Accept button) to render.
    await screen.findByText(/the real diff appears against a live backend/);

    await fireEvent.click(screen.getByRole('button', { name: /Accept & merge/ }));

    // Merged: the confirmation shows, the parent is notified, and the diff re-fetches to "gone".
    expect(await screen.findByText(/Merged into main/)).toBeTruthy();
    expect(onaccepted).toHaveBeenCalled();
    expect(await screen.findByText(/branch is gone — merged or deleted/)).toBeTruthy();
  });
});
