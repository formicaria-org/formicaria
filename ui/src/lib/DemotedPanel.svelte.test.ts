// **The panel that makes the demotion honest**, and the two writes it must make.
//
// `decisions.md` (2026-09-07) keeps a losing value in a `conflict-<field>` key rather than
// discarding it, and argues that this is not last-write-wins *because* the loser is visible and one
// tap from winning. Both halves of that sentence are asserted here: that the panel shows both
// answers, and that each button does what it says through the ordinary property path.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { setProperty } = vi.hoisted(() => ({ setProperty: vi.fn() }));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  setProperty,
}));

import DemotedPanel from './DemotedPanel.svelte';

const row = (over: Record<string, unknown> = {}) => ({
  id: '01JQ0000000000000000000000',
  vault: 'home',
  title: 'Ship the thing',
  field: 'status',
  kept: 'done',
  other: ['doing'],
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  setProperty.mockResolvedValue(undefined);
});

describe('both devices had an answer', () => {
  it('shows what the note holds and what the other device said', async () => {
    render(DemotedPanel, { rows: [row()], onclose: () => {}, onchanged: () => {} });
    const body = document.body.textContent ?? '';
    expect(body).toContain('Ship the thing');
    expect(body).toContain('status');
    // The whole claim of the ruling, on screen: neither value is missing.
    expect(body).toContain('done');
    expect(body).toContain('doing');
  });

  // "One tap from winning" is the sentence the ruling rests on. Two writes: the field takes the
  // other device's value, and the record of the disagreement goes.
  it('promoting the other answer sets the field and clears the record', async () => {
    const onchanged = vi.fn();
    render(DemotedPanel, { rows: [row()], onclose: () => {}, onchanged });

    await fireEvent.click(await screen.findByRole('button', { name: /Keep .doing./ }));

    await waitFor(() => expect(setProperty).toHaveBeenCalledTimes(2));
    expect(setProperty).toHaveBeenNthCalledWith(1, '01JQ0000000000000000000000', 'status', 'doing');
    expect(setProperty).toHaveBeenNthCalledWith(
      2,
      '01JQ0000000000000000000000',
      'conflict-status',
      '',
    );
    expect(onchanged).toHaveBeenCalled();
  });

  // Dismissing keeps what the note already shows. It must **not** write the field — doing so would
  // bump `updated` and re-stamp a value nobody chose to change.
  it('dismissing clears only the record', async () => {
    render(DemotedPanel, { rows: [row()], onclose: () => {}, onchanged: () => {} });

    await fireEvent.click(await screen.findByRole('button', { name: /Dismiss/ }));

    await waitFor(() => expect(setProperty).toHaveBeenCalledTimes(1));
    expect(setProperty).toHaveBeenCalledWith('01JQ0000000000000000000000', 'conflict-status', '');
  });

  // A second disagreement on one field offers both, because dropping one on the way to the screen
  // would be the silent loss the whole mechanism exists to prevent.
  it('offers every value the other side had', async () => {
    render(DemotedPanel, {
      rows: [row({ other: ['doing', 'review'] })],
      onclose: () => {},
      onchanged: () => {},
    });
    expect(await screen.findByRole('button', { name: /Keep .doing./ })).toBeTruthy();
    expect(await screen.findByRole('button', { name: /Keep .review./ })).toBeTruthy();
  });
});
