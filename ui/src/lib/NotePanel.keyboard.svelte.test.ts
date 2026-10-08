// **The line being written stays above the keyboard.**
//
// The Android shell draws edge-to-edge at SDK 36, where the system no longer resizes the window for the
// keyboard; it reports the overlap as `--kb` and fires `fm-keyboard` instead (`lib/keyboard.ts`). The
// editor gets shorter, but a textarea scrolls to its caret only on typing, never on a resize — so the
// line you tapped stayed put, behind the keyboard, and you wrote what you could not see (the owner,
// 2026-10-08). jsdom has no layout, so the caret's position and the box's height are stood in.
import { render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { getNote, thread, discussions, proposalFor, agentActivity, onlineAgents, backlinks, caret } =
  vi.hoisted(() => ({
    getNote: vi.fn(),
    thread: vi.fn(),
    discussions: vi.fn(),
    proposalFor: vi.fn(),
    agentActivity: vi.fn(),
    onlineAgents: vi.fn(),
    backlinks: vi.fn(),
    caret: { top: 0 },
  }));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  getNote,
  thread,
  discussions,
  proposalFor,
  agentActivity,
  onlineAgents,
  backlinks,
}));
vi.mock('./caret', () => ({ caretXY: () => ({ top: caret.top, left: 0, lineHeight: 20 }) }));

import NotePanel from './NotePanel.svelte';

const ID = 'M0CK0000000000000000000043';

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal('requestAnimationFrame', (f: FrameRequestCallback) => (f(0), 1));
  getNote.mockResolvedValue({
    id: ID,
    type: 'note',
    title: 'A long note',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-10-08T10:00:00Z',
    updated: '2026-10-08T10:00:00Z',
    tags: [],
    assets: [],
    props: {},
    vault: 'personal',
    body: 'line\n'.repeat(200),
    version: 'mock-v1',
  });
  thread.mockResolvedValue({ messages: [], count: 0 });
  discussions.mockResolvedValue([]);
  proposalFor.mockResolvedValue(null);
  onlineAgents.mockResolvedValue([]);
  backlinks.mockResolvedValue([]);
  agentActivity.mockResolvedValue({ active: false });
});

async function editorOf(): Promise<HTMLTextAreaElement> {
  render(NotePanel, { props: { id: ID, onclose: () => {}, startEditing: true } });
  const el = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
  // What the keyboard left: 300px of editor.
  Object.defineProperty(el, 'clientHeight', { configurable: true, get: () => 300 });
  el.focus();
  el.scrollTop = 1000;
  return el;
}

describe('the editor and the keyboard', () => {
  it('opens on the text alone — the properties are folded away', async () => {
    await editorOf();
    expect(screen.queryByLabelText('status')).toBeNull();
  });

  it('brings a caret the keyboard covered up into the top third of what is left', async () => {
    const el = await editorOf();
    caret.top = 450; // 150px below the bottom of the shortened editor
    window.dispatchEvent(new Event('fm-keyboard'));
    await waitFor(() => expect(el.scrollTop).toBe(1000 + 450 - 100));
  });

  it('leaves a caret that is still in sight where it is', async () => {
    const el = await editorOf();
    caret.top = 120;
    window.dispatchEvent(new Event('fm-keyboard'));
    await new Promise((r) => setTimeout(r, 0));
    expect(el.scrollTop).toBe(1000);
  });
});
