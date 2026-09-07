// **Frontmatter a user can see and change without opening a text editor.**
//
// Until 2026-08-29 the note's property editor offered six fixed fields — Status, Start, Due, Hard,
// Title, Tags — and nothing else. `ObjectMeta.props` (every other frontmatter key) was read in
// exactly two places in the whole UI, neither of them a display. So a key added by hand, by an
// importer, or by any future feature was written to the file and then **invisible**: correctable
// only in an editor, which `outstanding.md` §2.6b explicitly rules out for this app.
//
// Structural keys are the exception and are shown read-only. They are how a note is hidden from
// every view (`fm_app::thread`); `apply_property` already refuses two of them outright, because a
// board grouped by one plus a single drag would strand the note out of every surface at once.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  getNote,
  setProperty,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
} = vi.hoisted(() => ({
  getNote: vi.fn(),
  setProperty: vi.fn(),
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
  setProperty,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
}));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000042';

const noteDetail = (props: Record<string, unknown>) => ({
  id: ID,
  type: 'note',
  title: 'Attention Is All You Need',
  status: null,
  due: null,
  start: null,
  hard: false,
  created: '2026-08-01T10:00:00Z',
  updated: '2026-08-01T10:00:00Z',
  tags: ['paper'],
  assets: [],
  props,
  vault: 'personal',
  body: 'A paper worth re-reading.',
  version: 'mock-v1',
});

async function openDetails(props: Record<string, unknown>) {
  getNote.mockResolvedValue(noteDetail(props));
  render(NotePanel, { props: { id: ID, onclose: () => {} } });
  await waitFor(() => expect(getNote).toHaveBeenCalled());
  // The property form lives behind the note's Options popover — open it, then Edit.
  await fireEvent.click(await screen.findByRole('button', { name: 'note options' }));
  await fireEvent.click(await screen.findByRole('button', { name: /^(Edit|Details)$/ }));
}

beforeEach(() => {
  vi.clearAllMocks();
  setProperty.mockResolvedValue({});
  thread.mockResolvedValue({ messages: [], count: 0 });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  onlineAgents.mockResolvedValue([]);
  backlinks.mockResolvedValue([]);
  agentActivity.mockResolvedValue({ active: false });
});

describe('the note details panel', () => {
  it('shows every frontmatter key, not just the six it knows about', async () => {
    await openDetails({ year: 2017, venue: 'NeurIPS', doi: '10.48550/arXiv.1706.03762' });

    // Each is a real, labelled, editable field — the thing that did not exist before.
    expect(((await screen.findByLabelText('year')) as HTMLInputElement).value).toBe('2017');
    expect(((await screen.findByLabelText('venue')) as HTMLInputElement).value).toBe('NeurIPS');
    expect(((await screen.findByLabelText('doi')) as HTMLInputElement).value).toBe(
      '10.48550/arXiv.1706.03762',
    );
  });

  it('writes an edited value back through set_property', async () => {
    await openDetails({ venue: 'NeurIPS' });
    const field = (await screen.findByLabelText('venue')) as HTMLInputElement;
    await fireEvent.input(field, { target: { value: 'ICML' } });
    // Text fields debounce, so this also proves the write is not one call per keystroke.
    await waitFor(() => expect(setProperty).toHaveBeenCalledWith(ID, 'venue', 'ICML'), {
      timeout: 2000,
    });
  });

  it('adds a property that did not exist', async () => {
    await openDetails({});
    await fireEvent.input(await screen.findByLabelText('new property name'), {
      target: { value: 'authors' },
    });
    await fireEvent.input(await screen.findByLabelText('new property value'), {
      target: { value: 'Vaswani, A.' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Add' }));
    await waitFor(() => expect(setProperty).toHaveBeenCalledWith(ID, 'authors', 'Vaswani, A.'));
  });

  it('shows a list property as one editable line', async () => {
    await openDetails({ keywords: ['attention', 'transformers'] });
    expect(((await screen.findByLabelText('keywords')) as HTMLInputElement).value).toBe(
      'attention, transformers',
    );
  });

  it('shows a structural key but refuses to let it be edited', async () => {
    // `thread_of` hides a note from every planning view. Offering it as a text box is how a note
    // gets stranded — `apply_property` refuses it server-side for the same reason.
    // `thread_of` pointing at *this* note would make it a discussion, which has no property form
    // at all — so use the other shape the key takes: a reply, pointing at some other note.
    await openDetails({ thread_of: 'note:M0CK0000000000000000000099', mime: 'application/pdf' });
    expect((await screen.findByLabelText('thread_of')).hasAttribute('readonly')).toBe(true);
    expect((await screen.findByLabelText('mime')).hasAttribute('readonly')).toBe(true);
  });
});
