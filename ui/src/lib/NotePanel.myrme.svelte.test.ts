// **Myrme: one name for the assistant on every device** (owner, 2026-10-18). On a phone the owner
// wrote `@qwen3-vl-4b /transcribe` — the computer's model, named in the help — while the phone's own
// assistant was running under another name; nothing answered, and the message blamed memory.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const { onlineAgents, agentStatus, reply } = vi.hoisted(() => ({
  onlineAgents: vi.fn(),
  agentStatus: vi.fn(),
  reply: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => {
  const real = await importOriginal<typeof import('./ipc')>();
  return {
    ...real,
    onlineAgents,
    agentStatus,
    reply: (id: string, body: string) => {
      reply(id, body, real.thisDevice());
      return real.reply(id, body);
    },
  };
});

import NotePanel from './NotePanel.svelte';
import { handle, reset } from './mock';
import { asDesktop, asPhone } from './harness';
import type { ObjectMeta } from './types';

beforeEach(async () => {
  vi.clearAllMocks();
  (await import('./ipc')).forgetDevice();
  onlineAgents.mockResolvedValue(['Myrme']);
  agentStatus.mockResolvedValue({ enabled: true, model: 'lfm2.5-1.2b' });
});
afterEach(() => {
  asDesktop();
  reset();
});

async function openDiscussion() {
  const [n] = await handle<ObjectMeta[]>('create_notes', {
    notes: [{ title: 'ricetta tiramisu', body: 'A recording.', tags: [], props: {} }],
  });
  render(NotePanel, { id: n.id, onclose: vi.fn() });
  await fireEvent.click(await screen.findByRole('button', { name: /discussion/i }));
  return screen.findByPlaceholderText(/Add to the discussion/);
}

test('a model named from another device is pointed at Myrme, in one press', async () => {
  asPhone();
  const box = await openDiscussion();
  await fireEvent.input(box, { target: { value: '@qwen3-vl-4b /transcribe' } });
  await fireEvent.click(document.querySelector<HTMLButtonElement>('button.disc-send')!);

  expect(await screen.findByText(/isn’t on this device/)).toBeTruthy();
  expect(screen.queryByText(/not enough free memory/)).toBeNull();
  const instead = screen.getByRole('button', { name: 'Ask @Myrme instead' }) as HTMLButtonElement;
  await waitFor(() => expect(instead.disabled).toBe(false)); // the first send has finished
  await fireEvent.click(instead);
  await waitFor(() =>
    expect(reply).toHaveBeenLastCalledWith(expect.any(String), '@Myrme /transcribe', 'phone'),
  );
});

test('@Myrme and the model running here are both answered here, with no warning', async () => {
  const box = await openDiscussion();
  for (const said of ['@Myrme summarise this', '@lfm2.5-1.2b summarise this']) {
    await fireEvent.input(box, { target: { value: said } });
    await fireEvent.click(document.querySelector<HTMLButtonElement>('button.disc-send')!);
    // Sent, and the box cleared for the next one.
    await waitFor(() => expect((box as HTMLTextAreaElement).value).toBe(''));
    await waitFor(() => expect(reply).toHaveBeenLastCalledWith(expect.any(String), said, 'laptop'));
    expect(screen.queryByText(/isn’t on this device|did not start/)).toBeNull();
  }
});
