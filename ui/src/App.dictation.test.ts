// **Dictation** (`decisions.md` 2026-10-18; the owner: *"writing on phone is hard"*). On a phone, the
// quick row's microphone listens, and what was said is written at the caret — no proposal, since
// they are the person's own words. Pinned here, on the phone's own transport (`fm_dictate`): the
// words land where the caret was, with a space before them; and when they cannot be written down
// — transcription off, or nothing heard — **the recording is kept in the note**, never lost.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { stop } = vi.hoisted(() => ({ stop: vi.fn() }));
vi.mock('./lib/record', () => ({
  startRecording: async () => ({ stop, cancel: () => {} }),
}));
vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: vi.fn() } }));

import App from './App.svelte';
import * as mock from './lib/mock';
import { asPhone, asDesktop, type CountingBridge } from './lib/harness';

let bridge: CountingBridge;
beforeEach(() => {
  mock.reset();
  bridge = asPhone();
  stop.mockResolvedValue(new File([new Uint8Array(3200)], 'dictation.wav', { type: 'audio/wav' }));
});
afterEach(() => {
  asDesktop();
  mock.setMockDictation('Cinque uova, 500 grammi di mascarpone.', false);
});

async function dictateInto(body: string) {
  const ids = mock.seed({ notes: 3 });
  await mock.handle('update_body', { id: ids[0], body, base: '' });
  render(App);
  await fireEvent.click(await screen.findByText(/Seeded 0/));
  await fireEvent.click(await screen.findByTitle('Click to edit'));
  const el = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
  el.focus();
  el.selectionStart = el.selectionEnd = body.length;
  const row = await screen.findByRole('toolbar', { name: 'quick actions' });
  await fireEvent.click(within(row).getByRole('button', { name: 'dictate' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Stop & write' }));
  return el;
}

describe('dictation', () => {
  it('writes what was said at the caret, on the phone’s own transport', async () => {
    mock.setMockDictation('Cinque uova, 500 grammi di mascarpone.');
    const el = await dictateInto('Tiramisù:');
    await waitFor(() => expect(el.value).toBe('Tiramisù: Cinque uova, 500 grammi di mascarpone.'));
    expect(bridge.count('dictate')).toBe(1);
    expect(bridge.count('ingest')).toBe(0); // nothing kept: the words are the note
  });

  it('keeps the recording in the note when transcription is off, and says how to turn it on', async () => {
    mock.setMockDictation('', false);
    const el = await dictateInto('Tiramisù:');
    await waitFor(() => expect(el.value).toMatch(/!\[.*\]\(asset:sha256-[0-9a-f]+\)/));
    expect(await screen.findByText(/Turn it on in Settings.*kept in the note/)).toBeTruthy();
  });

  it('keeps the recording when nothing could be written down', async () => {
    mock.setMockDictation('');
    const el = await dictateInto('');
    await waitFor(() => expect(el.value).toMatch(/asset:sha256-/));
    expect(await screen.findByText(/Nothing could be written down.*kept in the note/)).toBeTruthy();
  });
});
