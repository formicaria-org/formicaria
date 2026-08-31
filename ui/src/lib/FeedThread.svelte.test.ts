// **The discussion inside a feed post.**
//
// The backend half of this shipped long ago — a message is a note carrying `thread_of`, `reply`
// writes one file, `thread` reads them back. What is new is the surface, and what these pin is that
// it is the *small* surface: the tail of the conversation and one box, with the expensive parts
// (the mention picker, the agent poll, proposals, per-message actions) deliberately left in the
// note pane. See `decisions.md`, 2026-08-31.

import { describe, expect, it, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';
import FeedThread from './FeedThread.svelte';
import * as mock from './mock';
import type { ObjectMeta } from './types';

let note: string;
beforeEach(async () => {
  note = (await mock.handle<ObjectMeta>('capture', { body: 'the note under discussion' })).id;
});

describe('a discussion in a post', () => {
  it('shows the messages already on the note', async () => {
    await mock.handle('reply', { id: note, body: 'a first thought' });
    render(FeedThread, { props: { noteId: note } });
    expect(await screen.findByText('a first thought')).toBeTruthy();
    cleanup();
  });

  it('posts a message and reports the new count, without asking the feed to reload', async () => {
    const onposted = vi.fn();
    const onsaved = vi.fn();
    render(FeedThread, { props: { noteId: note, onposted, onsaved } });
    await screen.findByLabelText('write a message');

    await fireEvent.input(screen.getByLabelText('write a message'), {
      target: { value: 'posted from the feed' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Send' }));

    expect(await screen.findByText('posted from the feed')).toBeTruthy();
    // The count goes back to the feed so one badge can move. Re-reading the feed instead would
    // re-run `recent()` *and* `activity()`'s git revwalk under the vault lock for a 200-byte write
    // — and would collapse the reader's own window on the next beat.
    await waitFor(() => expect(onposted).toHaveBeenCalledWith(1));
    // A message is a file write like any other, so it rides the editor's commit debounce.
    expect(onsaved).toHaveBeenCalled();
    cleanup();
  });

  it('shows only the tail, and points at the note for the rest', async () => {
    for (let i = 0; i < 5; i++) {
      await mock.handle('reply', { id: note, body: `message ${i}` });
    }
    render(FeedThread, { props: { noteId: note } });

    // The last three, not all five — a feed row is not where a long thread is read.
    expect(await screen.findByText('message 4')).toBeTruthy();
    expect(screen.getByText('message 2')).toBeTruthy();
    expect(screen.queryByText('message 1')).toBeNull();
    expect(screen.getByRole('button', { name: /2 earlier messages/ })).toBeTruthy();
    cleanup();
  });

  it('will not send an empty message', async () => {
    render(FeedThread, { props: { noteId: note } });
    const send = await screen.findByRole('button', { name: 'Send' });
    expect((send as HTMLButtonElement).disabled).toBe(true);

    await fireEvent.input(screen.getByLabelText('write a message'), { target: { value: '   ' } });
    expect((send as HTMLButtonElement).disabled, 'whitespace is not a message').toBe(true);
    cleanup();
  });

  // The expensive surface stays in the note pane: a mention picker needs a full-corpus
  // `discussions()` scan to build its list, and the agent poll is two commands every four seconds
  // *per open thread*. One tap away, where they already work.
  it('carries none of the note panel’s heavy discussion chrome', async () => {
    await mock.handle('reply', { id: note, body: 'a thought' });
    render(FeedThread, { props: { noteId: note } });
    await screen.findByText('a thought');

    expect(screen.queryByRole('button', { name: /delete/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /reply/i })).toBeNull();
    cleanup();
  });
});
