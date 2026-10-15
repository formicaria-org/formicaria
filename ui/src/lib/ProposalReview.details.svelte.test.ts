// A proposal that sets a note's details (a meeting's time and place) shows them as fields beside what
// the note has now, and lets them be corrected before accepting. The shared mock carries no details,
// so `ipc` is stubbed here with the shape the real backend returns.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { typeInto } from './editor/testing';
import type { ProposalContent } from './types';

const content: { value: ProposalContent } = {
  value: {
    host: 'N1',
    title: 'Task assignments for the project',
    body: 'the conversation',
    start: '2026-10-16T17:00',
    due: '2026-10-16T17:00',
    location: 'https://zoom.us/j/1',
    wasStart: null,
    wasDue: null,
    wasLocation: null,
    isNew: false,
  },
};
const createProposal = vi.fn(async (..._args: unknown[]) => ({}));

vi.mock('./ipc', () => ({
  proposalDiff: vi.fn(async () => ({ exists: true, declined: false, files: [], patch: '' })),
  proposalContent: vi.fn(async () => content.value),
  proposalShown: vi.fn(async () => {}),
  acceptProposal: vi.fn(async () => ({ outcome: 'merged' })),
  rejectProposal: vi.fn(async () => {}),
  createProposal: (...a: unknown[]) => createProposal(...a),
}));

const { default: ProposalReview } = await import('./ProposalReview.svelte');
const field = (name: string) => screen.getByLabelText(name) as HTMLInputElement;

describe('ProposalReview — the details a proposal sets', () => {
  beforeEach(() => createProposal.mockClear());

  it('shows the proposed time and place as fields, and what the note has now', async () => {
    render(ProposalReview, { id: 'P1' });
    await screen.findByTestId('pr-details');
    expect(field('proposed start').value).toBe('2026-10-16');
    expect(field('proposed start time').value).toBe('17:00');
    expect(field('proposed place').value).toBe('https://zoom.us/j/1');
    // The note has no date yet: each field says so, rather than leaving the change implied.
    expect(screen.getAllByText('now: not set').length).toBe(3);
  });

  it('sends a corrected time with the save, and nothing extra when only the text changed', async () => {
    render(ProposalReview, { id: 'P1' });
    await screen.findByTestId('pr-details');

    await fireEvent.input(field('proposed start time'), { target: { value: '18:30' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(createProposal).toHaveBeenCalledTimes(1);
    expect(createProposal.mock.calls[0]).toEqual([
      'N1',
      'the conversation',
      undefined,
      undefined,
      { start: '2026-10-16T18:30', due: '2026-10-16T17:00', location: 'https://zoom.us/j/1' },
    ]);
  });

  it('leaves the details out of a text-only save', async () => {
    render(ProposalReview, { id: 'P1' });
    const box = await screen.findByLabelText('proposed note body');
    typeInto(box, 'edited');
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    expect(createProposal.mock.calls[0]?.[4]).toBeUndefined();
  });

  it('names the current value a change would replace', async () => {
    content.value = {
      ...content.value,
      wasStart: '2026-10-16T12:00',
      wasDue: '2026-10-16T12:00',
      wasLocation: 'B2-14',
    };
    render(ProposalReview, { id: 'P1' });
    await screen.findByTestId('pr-details');
    expect(screen.getByText('now: B2-14')).toBeTruthy();
    expect(screen.getAllByText(/^now: 16 Oct 12:00$/).length).toBe(2);
  });

  it('has no details section for a proposal that changes only the text', async () => {
    content.value = { ...content.value, start: null, due: null, location: null };
    render(ProposalReview, { id: 'P1' });
    await screen.findByLabelText('proposed note body');
    expect(screen.queryByTestId('pr-details')).toBeNull();
  });

  it('shows a new title beside the current one, and sends a corrected one', async () => {
    content.value = {
      host: 'N2',
      title: 'Marta Bianchi — Master student, attention estimation',
      body: 'the conversation',
      wasTitle: 'Rescheduling Our Meeting',
      isNew: false,
    };
    render(ProposalReview, { id: 'P2' });
    await screen.findByTestId('pr-details');
    expect(field('proposed title').value).toBe(
      'Marta Bianchi — Master student, attention estimation',
    );
    expect(screen.getByText('now: Rescheduling Our Meeting')).toBeTruthy();
    expect(screen.queryByLabelText('proposed start')).toBeNull();

    await fireEvent.input(field('proposed title'), {
      target: { value: 'Marta Bianchi — Master student, gaze estimation' },
    });
    await fireEvent.click(screen.getByRole('button', { name: /Save changes/ }));
    // The title alone — never an empty date that would clear the meeting the note already has.
    expect(createProposal.mock.calls[0]?.[4]).toEqual({
      title: 'Marta Bianchi — Master student, gaze estimation',
    });
  });
});
