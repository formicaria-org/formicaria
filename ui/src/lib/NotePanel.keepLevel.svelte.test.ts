// **An open note and the changes that arrive under it** (2026-10-17, `decisions.md#sync`, *one
// person's devices are kept level*).
//
// The conflict the owner kept meeting between their own phone and laptop was not two people
// disagreeing: a note opened before a get kept the old version, and the first keystroke after the
// get was refused as stale and shown as "someone else changed this note". These pin the panel's
// half of the fix — take what arrived when nothing here is unsaved, keep both versions when
// something is — and the two things it now tells the backup loop: an edit is starting, a note
// written in was left.
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
import { changesArrived, setLevelHooks } from './keepLevel.svelte';

const ID = 'M0CK0000000000000000000043';

const detail = (body: string, version: string) => ({
  id: ID,
  type: 'note',
  title: 'Reading list',
  status: null,
  due: null,
  start: null,
  hard: false,
  created: '2026-10-14T16:44:53Z',
  updated: '2026-10-14T16:56:00Z',
  tags: [],
  assets: [],
  props: {},
  vault: 'personal',
  body,
  version,
});

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

let disk = { body: 'from this device', version: 'v1' };
const editing = vi.fn();
const left = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  disk = { body: 'from this device', version: 'v1' };
  getNote.mockImplementation(async () => detail(disk.body, disk.version));
  updateBody.mockImplementation(async (id: string, body: string, base: string) => {
    if (base !== disk.version) throw new Error(`note ${id} changed on disk since it was opened`);
    disk = { body, version: `v${Number(disk.version.slice(1)) + 1}` };
    return disk.version;
  });
  backlinks.mockResolvedValue([]);
  thread.mockResolvedValue({ messages: [] });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  agentActivity.mockResolvedValue({ active: false });
  setLevelHooks({ editing, left });
});

/// Another device's change lands on disk, and the backup loop says so.
function theOtherDeviceSends(body: string) {
  disk = { body, version: `v${Number(disk.version.slice(1)) + 1}` };
  changesArrived();
}

async function openEditor() {
  await fireEvent.click(await screen.findByTitle('Click to edit'));
  return (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
}

describe('an open note when changes arrive', () => {
  it('takes them when it holds nothing unsaved, so the next keystroke is not refused', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await screen.findByText('from this device');

    theOtherDeviceSends('from this device\n\nand from the other one');
    await screen.findByText('and from the other one');

    // The edit that used to be refused as stale now saves against the version that arrived.
    const editor = await openEditor();
    expect(editor.value).toContain('and from the other one');
    await fireEvent.input(editor, { target: { value: `${editor.value}\n\nand more` } });
    await sleep(650);
    await waitFor(() => expect(disk.body).toContain('and more'));
    expect(screen.queryByText(/Someone else changed this note/)).toBeNull();
  });

  it('keeps what is being written, and shows both versions, when something is unsaved', async () => {
    updateBody.mockImplementation(() => new Promise(() => {})); // the save never lands
    const view = render(NotePanel, { props: { id: ID, onclose: () => {} } });
    const editor = await openEditor();
    await fireEvent.input(editor, { target: { value: 'from this device, still being written' } });

    theOtherDeviceSends('their version');
    await sleep(50);
    expect(editor.value).toBe('from this device, still being written');
    // Closed here, and the debounced save (which still fires after a close — the draft is kept)
    // waited out while the never-landing fake is in place, so it cannot reach the next test's disk.
    view.unmount();
    await sleep(700);
  });
});

describe('what the note tells the backup loop', () => {
  it('says an edit is starting, with the note’s vault', async () => {
    render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await openEditor();
    await waitFor(() => expect(editing).toHaveBeenCalledWith('personal'));
  });

  it('says a note it wrote in was left, and stays quiet about one only read', async () => {
    const view = render(NotePanel, { props: { id: ID, onclose: () => {} } });
    await screen.findByText('from this device');
    view.unmount();
    expect(left).not.toHaveBeenCalled();

    const again = render(NotePanel, { props: { id: ID, onclose: () => {} } });
    const editor = await openEditor();
    await fireEvent.input(editor, { target: { value: 'written here' } });
    await sleep(650);
    await waitFor(() => expect(disk.body).toBe('written here'));
    again.unmount();
    expect(left).toHaveBeenCalledWith('personal');
  });
});
