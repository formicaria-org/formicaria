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
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'the proposed body',
    });

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
  it('saves the reviewer edits before merging, instead of silently dropping them', async () => {
    // The regression this pins: `acceptProposal` sends only the proposal id, never the draft, so
    // clicking Accept with unsaved edits used to merge the model's ORIGINAL text and throw the
    // reviewer's correction away with nothing on screen saying so.
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'the model text',
    });

    render(ProposalReview, { id: prop.id });
    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    expect(box.value).toContain('the model text');

    // Edit, then Accept WITHOUT pressing Save — the exact path that lost the correction.
    await fireEvent.input(box, { target: { value: 'my correction' } });
    await fireEvent.click(screen.getByRole('button', { name: /Accept & merge/ }));
    expect(await screen.findByText(/Merged into main/)).toBeTruthy();

    // What reached the backend is the reviewer's text, not the model's.
    const after = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(after.props.proposedBody).toBe('my correction');
  });

  it('sends the reviewer reason with a save, and treats a blank one as a genuine skip', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });

    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    await fireEvent.input(box, { target: { value: 'corrected' } });

    // One tap fills the common case; the field stays free prose.
    await fireEvent.click(screen.getByRole('button', { name: '@wrong' }));
    const whyBox = screen.getByLabelText(/What did you change/) as HTMLInputElement;
    expect(whyBox.value).toBe('@wrong');
    await fireEvent.input(whyBox, { target: { value: 'misheard the name @wrong' } });

    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(await screen.findByText(/Saved/)).toBeTruthy();
    const saved = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(saved.props.why).toBe('misheard the name @wrong');

    // ...and it is cleared, so the next edit does not silently inherit the last reason.
    expect((screen.getByLabelText(/What did you change/) as HTMLInputElement).value).toBe('');
  });

  it('lets a reason be skipped entirely — the field is optional, not a gate', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });

    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    await fireEvent.input(box, { target: { value: 'a one-word typo fix' } });
    // Straight to Save with the reason box untouched: no extra click, no dismissal, no failure.
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(await screen.findByText(/Saved/)).toBeTruthy();
    const saved = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(saved.props.proposedBody).toBe('a one-word typo fix');
    expect(saved.props.why).toBeUndefined();
  });

  it('asks a different question when a reject is armed, and keeps the answer', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });
    await screen.findByText(/the real diff appears against a live backend/);

    const reject = () => screen.getByRole('button', { name: /reject/i });
    await fireEvent.click(reject()); // arms it — and the question changes with it
    const whyBox = await screen.findByLabelText(/Why are you turning this down/);
    await fireEvent.input(whyBox, { target: { value: 'invented a source' } });
    await fireEvent.click(reject()); // confirms

    expect(await screen.findByText(/kept as a declined record/)).toBeTruthy();
    // A rejection writes no commit and its text never reaches main, so this note is the only
    // place the reason can survive.
    const after = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(after.props.declined_why).toBe('invented a source');
  });

  it('uses no git vocabulary in the strings it added', async () => {
    // The owner's constraint: this has to work without the user knowing what a commit is. The
    // component still says "Accept & merge" and "main" from before — deliberately not touched here,
    // but not added to either.
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });
    const label = (await screen.findByText(/What did you change/)).textContent ?? '';
    const placeholder =
      (screen.getByLabelText(/What did you change/) as HTMLInputElement).placeholder ?? '';
    for (const word of ['commit', 'branch', 'merge', 'trailer', 'HEAD', 'repository']) {
      expect(`${label} ${placeholder}`.toLowerCase()).not.toContain(word.toLowerCase());
    }
  });

  it('records what kind of correction it was, single-select and optional', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });
    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    await fireEvent.input(box, { target: { value: 'corrected' } });

    // Single-select: picking a second kind replaces the first rather than adding to it.
    await fireEvent.click(screen.getByRole('button', { name: 'style' }));
    await fireEvent.click(screen.getByRole('button', { name: 'factual' }));
    expect(screen.getByRole('button', { name: 'factual' }).getAttribute('aria-pressed')).toBe(
      'true',
    );
    expect(screen.getByRole('button', { name: 'style' }).getAttribute('aria-pressed')).toBe(
      'false',
    );

    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(await screen.findByText(/Saved/)).toBeTruthy();
    const saved = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(saved.props.kind).toBe('factual');
  });

  it('leaves the kind unset when nobody picks one', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });
    render(ProposalReview, { id: prop.id });
    const box = (await screen.findByLabelText('proposed note body')) as HTMLTextAreaElement;
    await fireEvent.input(box, { target: { value: 'a typo fix' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(await screen.findByText(/Saved/)).toBeTruthy();
    const saved = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(saved.props.kind).toBeUndefined();
  });

  it('records that the proposal was actually put in front of someone', async () => {
    const note = await mock.handle<ObjectMeta>('capture', { body: 'a note' });
    const prop = await mock.handle<ObjectMeta>('create_proposal', {
      id: note.id,
      body: 'model text',
    });

    const before = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
    expect(before.props.shown).toBeUndefined();

    render(ProposalReview, { id: prop.id });
    await screen.findByLabelText('proposed note body');

    // Stamped once it is genuinely on screen — this is what separates "read it and left it" from
    // "never opened", which are the same absence in the data and opposite facts about the model.
    await vi.waitFor(async () => {
      const after = await mock.handle<{ props: Record<string, string> }>('get', { id: prop.id });
      expect(after.props.shown).toBeTruthy();
    });
  });
});
