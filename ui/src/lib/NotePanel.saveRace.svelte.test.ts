// **An autosave must never conflict with the autosave before it.**
//
// The owner's phone, 2026-09-11: typing in a note that existed only on that phone, with nobody else
// writing to it, repeatedly dropped `<<<<<<< your unsaved edit` markers into the text.
//
// The server's lost-update guard is right: it refuses a write whose `base` is not the body on disk.
// What was wrong is that the editor raced itself. Every write sent `base` as it stood when *that*
// write started, and nothing stopped a second write leaving while the first was still in flight. A
// phone write routinely outlasts the 500 ms debounce — slower storage, and the vault lock held by a
// background commit — so the next autosave went out carrying the version the first was replacing,
// was refused, and `onSaveRejected` handled it as someone else's edit: it reloaded the note and
// appended the user's own newer text under the markers.
//
// The mock below behaves like the real guard, and each write waits until the test lets it land, so
// the overlap is deterministic rather than a matter of machine speed. Real timers, for the reason
// `NotePanel.bigNote.svelte.test.ts` gives.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { getNote, updateBody, thread, discussions, proposalFor, agentActivity, backlinks } =
  vi.hoisted(() => ({
    getNote: vi.fn(),
    updateBody: vi.fn(),
    thread: vi.fn(),
    discussions: vi.fn(),
    proposalFor: vi.fn(),
    agentActivity: vi.fn(),
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
  backlinks,
}));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000042';

const detail = (body: string, version: string) => ({
  id: ID,
  type: 'note',
  title: 'Ideas',
  status: null,
  due: null,
  start: null,
  hard: false,
  created: '2026-09-11T07:00:00Z',
  updated: '2026-09-11T07:00:00Z',
  tags: [],
  assets: [],
  props: {},
  vault: 'personal',
  body,
  version,
});

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/// What is on disk, and the writes that have been sent but not yet allowed to land.
let disk = { body: 'first line', version: 'v1' };
let landings: Array<() => void> = [];

beforeEach(() => {
  vi.clearAllMocks();
  disk = { body: 'first line', version: 'v1' };
  landings = [];
  getNote.mockImplementation(async () => detail(disk.body, disk.version));
  // The real guard: a write whose base is not the body on disk is refused, with the server's words.
  updateBody.mockImplementation(
    (id: string, body: string, base: string) =>
      new Promise<string>((resolve, reject) => {
        landings.push(() => {
          if (base !== disk.version) {
            reject(new Error(`note ${id} changed on disk since it was opened`));
            return;
          }
          disk = { body, version: `v${Number(disk.version.slice(1)) + 1}` };
          resolve(disk.version);
        });
      }),
  );
  backlinks.mockResolvedValue([]);
  thread.mockResolvedValue({ messages: [] });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  agentActivity.mockResolvedValue({ active: false });
});

/// Open the body editor, the way `NotePanel.bigNote` does.
async function openEditor() {
  await fireEvent.click(await screen.findByLabelText('note options'));
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
  return (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
}

describe('typing while a save is still in flight', () => {
  it('waits for the slow save, sends the version it returned, and marks nothing as a conflict', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    const editor = await openEditor();

    // Type, pause past the debounce: the first write leaves, and hangs.
    await fireEvent.input(editor, { target: { value: 'first line\nsecond' } });
    await sleep(650);
    await waitFor(() => expect(updateBody).toHaveBeenCalledTimes(1));

    // Type again and pause again while it is still in flight.
    await fireEvent.input(editor, { target: { value: 'first line\nsecond\nthird' } });
    await sleep(650);

    // Unfixed, a second write has already left here, carrying `v1` — the version the first write
    // is in the middle of replacing.
    expect(updateBody).toHaveBeenCalledTimes(1);

    // The first write lands; only now may the second leave, and it must carry what the first
    // returned.
    landings.shift()!();
    await waitFor(() => expect(updateBody).toHaveBeenCalledTimes(2));
    expect(updateBody.mock.calls[1][2]).toBe('v2');

    landings.shift()!();
    await waitFor(() => expect(disk.body).toBe('first line\nsecond\nthird'));
    await sleep(50);
    expect(editor.value).not.toContain('<<<<<<<');
    expect(screen.queryByText(/Someone else changed this note/)).toBeNull();
  });
});
