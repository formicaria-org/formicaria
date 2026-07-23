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
    expect(await screen.findByText(/merged into the note/)).toBeTruthy();
  });

  it('shows the proposed note and lets the reviewer edit and save it', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', { id: note.id, body: 'the proposed body' });

    render(ProposalReview, { id: prop.id });
    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    expect(box.value).toContain('the proposed body');

    // Editing enables Save; saving round-trips and reflects the edit (the real backend revises the branch).
    await fireEvent.input(box, { target: { value: 'my edited body' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(await screen.findByText(/Saved/)).toBeTruthy();
    const box2 = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    expect(box2.value).toContain('my edited body');
  });

  it('rejects a proposal (two-click) and shows it kept as a declined record', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', { id: note.id, body: 'better' });
    const onrejected = vi.fn();

    render(ProposalReview, { id: prop.id, onrejected });
    await screen.findByText(/the real diff appears against a live backend/);

    // Two-click: first click arms, second confirms.
    const reject = () => screen.getByRole('button', { name: /reject/i });
    await fireEvent.click(reject());
    await fireEvent.click(reject());

    expect(await screen.findByText(/kept as a declined record/)).toBeTruthy();
    expect(onrejected).toHaveBeenCalled();
    // The proposal is not deleted — it re-fetches to a declined state, not an error.
    expect(await screen.findByText(/This proposal was declined/)).toBeTruthy();
  });
});
