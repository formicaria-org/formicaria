// **The panel that makes the demotion honest**, and the two writes it must make — plus the second
// shape that shares it: a note the merge brought back.
//
// `decisions.md` (2026-09-07) keeps a losing value in a `conflict-<field>` key rather than
// discarding it, and argues that this is not last-write-wins *because* the loser is visible and one
// tap from winning. Both halves of that sentence are asserted here: that the panel shows both
// answers, and that each button does what it says through the ordinary property path.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { setProperty, deleteNote, keptSeen } = vi.hoisted(() => ({
  setProperty: vi.fn(),
  deleteNote: vi.fn(),
  keptSeen: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  setProperty,
  deleteNote,
  keptSeen,
}));

import DemotedPanel from './KeptPanel.svelte';

const keptRow = (over: Record<string, unknown> = {}) => ({
  path: '01JQ0000000000000000000000',
  id: '01JQ0000000000000000000000',
  vault: 'home',
  title: 'the note they deleted',
  ...over,
});

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
  deleteNote.mockResolvedValue(undefined);
  keptSeen.mockResolvedValue({ seen: true });
});

describe('both devices had an answer', () => {
  it('shows what the note holds and what the other device said', async () => {
    render(DemotedPanel, { kept: [], rows: [row()], onclose: () => {}, onchanged: () => {} });
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
    render(DemotedPanel, { kept: [], rows: [row()], onclose: () => {}, onchanged });

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
    render(DemotedPanel, { kept: [], rows: [row()], onclose: () => {}, onchanged: () => {} });

    await fireEvent.click(await screen.findByRole('button', { name: /Dismiss/ }));

    await waitFor(() => expect(setProperty).toHaveBeenCalledTimes(1));
    expect(setProperty).toHaveBeenCalledWith('01JQ0000000000000000000000', 'conflict-status', '');
  });

  // A second disagreement on one field offers both, because dropping one on the way to the screen
  // would be the silent loss the whole mechanism exists to prevent.
  it('offers every value the other side had', async () => {
    render(DemotedPanel, {
      kept: [],
      rows: [row({ other: ['doing', 'review'] })],
      onclose: () => {},
      onchanged: () => {},
    });
    expect(await screen.findByRole('button', { name: /Keep .doing./ })).toBeTruthy();
    expect(await screen.findByRole('button', { name: /Keep .review./ })).toBeTruthy();
  });
});

// **The second shape in the same panel**: a note the merge brought back because the other device
// had deleted it while this one was editing it (`decisions.md`, 2026-09-07). The keep discards a
// deletion — deliberately, and on the argument that a resurrected note is recoverable while a note
// deleted by fiat on a phone with no shell is not — so the surface owes the user the undo.
describe('notes that came back', () => {
  it('names the note and says what happened to it', async () => {
    render(DemotedPanel, { kept: [keptRow()], rows: [], onclose: () => {}, onchanged: () => {} });
    const body = document.body.textContent ?? '';
    expect(body).toContain('the note they deleted');
    expect(body).toContain('came back');
  });

  it('arms the delete before it fires, like every other destructive control here', async () => {
    // The house stance, not a preference: `NotePanel` arms a confirm strip, `BackupPanel` asks
    // Remove/Cancel, `prune_duplicates` refuses outright rather than delete something unrecoverable.
    // A one-tap delete in a list somebody opened to *read* is the outlier a stray thumb finds.
    render(DemotedPanel, { kept: [keptRow()], rows: [], onclose: () => {}, onchanged: () => {} });

    await fireEvent.click(await screen.findByRole('button', { name: /Delete it again/i }));
    expect(deleteNote).not.toHaveBeenCalled();

    await fireEvent.click(await screen.findByRole('button', { name: /Yes, delete it/i }));
    await waitFor(() => expect(deleteNote).toHaveBeenCalledWith('01JQ0000000000000000000000'));
  });

  it('cancels back to unarmed without deleting anything', async () => {
    render(DemotedPanel, { kept: [keptRow()], rows: [], onclose: () => {}, onchanged: () => {} });
    await fireEvent.click(await screen.findByRole('button', { name: /Delete it again/i }));
    await fireEvent.click(await screen.findByRole('button', { name: /Cancel/i }));
    expect(deleteNote).not.toHaveBeenCalled();
    expect(await screen.findByRole('button', { name: /Delete it again/i })).toBeTruthy();
  });

  it('offers no delete for a kept path that is not a note', async () => {
    // The keep branch settles every delete/modify path in the repo, so a saved view or the
    // attachment manifest can come back too. Saying so is honest; offering `delete` on it would
    // point a note command at something that is not a note.
    render(DemotedPanel, {
      kept: [keptRow({ path: 'manifest.json', id: null, title: null })],
      rows: [],
      onclose: () => {},
      onchanged: () => {},
    });
    expect(document.body.textContent).toContain('manifest.json');
    expect(screen.queryByRole('button', { name: /Delete it again/i })).toBeNull();
  });

  it('acknowledges the whole list in one write, because looking is not per-row', async () => {
    const onchanged = vi.fn();
    render(DemotedPanel, { kept: [keptRow()], rows: [], onclose: () => {}, onchanged });
    await fireEvent.click(await screen.findByRole('button', { name: /I have seen these/i }));
    await waitFor(() => expect(keptSeen).toHaveBeenCalledTimes(1));
    expect(onchanged).toHaveBeenCalled();
  });

  it('says nothing about resurrections when there are none', async () => {
    render(DemotedPanel, { kept: [], rows: [row()], onclose: () => {}, onchanged: () => {} });
    expect(document.body.textContent).not.toContain('came back');
  });
});
