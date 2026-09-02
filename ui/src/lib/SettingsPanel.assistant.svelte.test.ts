// **The first enable spends gigabytes, so it asks first.**
//
// Turning the assistant on for the first time downloads a model — 2.5 GB for the desktop pick, plus
// another 836 MB if it is to read images. A checkbox that starts that silently is the failure the
// phone already has: the switch flips and nothing visibly happens for minutes, on a connection the
// user may be paying for.
//
// So what is pinned here is the shape of the question, not its wording: the sizes and the licence
// come from the catalogue and are on screen *before* anything is fetched, the total accounts for the
// optional projector, and a download in flight can be stopped. The progress line reports bytes
// rather than a percentage, because the server does not always send a length and an invented
// percentage is worse than an honest number.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

const { agentStatus, agentModels, setAgent } = vi.hoisted(() => ({
  agentStatus: vi.fn(),
  agentModels: vi.fn(),
  setAgent: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  agentStatus,
  agentModels,
  setAgent,
}));

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
  } as never);
}

const status = (over: Record<string, unknown> = {}) => ({
  enabled: false,
  transcribe: false,
  installed: true,
  why: '',
  transcribe_available: false,
  provisioned: false,
  provisioning: null,
  ...over,
});

const catalogue = [
  {
    name: 'qwen3-vl-4b',
    bytes: 2_497_281_664,
    license: 'Apache-2.0',
    vision: true,
    mmproj_bytes: 836_180_256,
    default: true,
  },
  {
    name: 'lfm2.5-1.2b',
    bytes: 1_400_000_000,
    license: 'LFM Open License v1.0',
    vision: false,
    mmproj_bytes: null,
    default: false,
  },
];

describe('turning the study assistant on for the first time', () => {
  it('asks before downloading anything, with the size and the licence on screen', async () => {
    agentStatus.mockResolvedValue(status());
    agentModels.mockResolvedValue(catalogue);
    setAgent.mockResolvedValue({ ok: true });
    panel();

    await fireEvent.click(await screen.findByRole('checkbox', { name: /study assistant/i }));

    // Nothing has been asked of the server yet — the question comes first.
    expect(setAgent).not.toHaveBeenCalled();
    expect(await screen.findByText(/needs a model on this computer/i)).toBeTruthy();
    // The numbers a person is being asked to accept.
    expect((await screen.findAllByText(/2\.5GB/)).length).toBeGreaterThan(0);
    expect((await screen.findAllByText(/Apache-2\.0/)).length).toBeGreaterThan(0);
  });

  it('counts the image reader into the total only when it is chosen', async () => {
    agentStatus.mockResolvedValue(status());
    agentModels.mockResolvedValue(catalogue);
    setAgent.mockResolvedValue({ ok: true });
    panel();
    await fireEvent.click(await screen.findByRole('checkbox', { name: /study assistant/i }));

    // The default pick can see, so the extra download is offered — and not assumed.
    const vision = await screen.findByRole('checkbox', { name: /Read images too/i });
    expect((vision as HTMLInputElement).checked).toBe(false);
    expect(await screen.findByText(/Total: 2\.5GB/)).toBeTruthy();

    await fireEvent.click(vision);
    expect(await screen.findByText(/Total: 3\.3GB/)).toBeTruthy();
  });

  it('sends the chosen model and only then starts the download', async () => {
    agentStatus.mockResolvedValue(status());
    agentModels.mockResolvedValue(catalogue);
    setAgent.mockResolvedValue({ ok: true });
    panel();
    await fireEvent.click(await screen.findByRole('checkbox', { name: /study assistant/i }));

    await fireEvent.click(await screen.findByRole('radio', { name: /lfm2\.5-1\.2b/i }));
    await fireEvent.click(await screen.findByRole('button', { name: /Download and turn on/i }));

    expect(setAgent).toHaveBeenCalledWith(true, 'lfm2.5-1.2b', false);
  });

  it('reports progress in bytes and offers to stop', async () => {
    agentStatus.mockResolvedValue(
      status({
        enabled: true,
        provisioning: { stage: 'model', done: 1_200_000_000, total: 2_497_281_664, error: null },
      }),
    );
    agentModels.mockResolvedValue(catalogue);
    setAgent.mockResolvedValue({ ok: true });
    panel();

    expect(await screen.findByText(/downloading the model/i)).toBeTruthy();
    expect(await screen.findByText(/1\.2GB of 2\.5GB/)).toBeTruthy();
    expect(await screen.findByRole('button', { name: /Stop/i })).toBeTruthy();
  });

  it('says why a download failed, in the server’s own words', async () => {
    agentStatus.mockResolvedValue(
      status({
        enabled: true,
        provisioning: { stage: 'failed', done: 0, total: null, error: 'no space left on device' },
      }),
    );
    agentModels.mockResolvedValue(catalogue);
    panel();

    expect(await screen.findByText(/download failed/i)).toBeTruthy();
    expect(await screen.findByText(/no space left on device/)).toBeTruthy();
  });

  it('is an ordinary switch once the model is already here', async () => {
    agentStatus.mockResolvedValue(status({ provisioned: true }));
    agentModels.mockResolvedValue(catalogue);
    setAgent.mockResolvedValue({ ok: true });
    panel();

    await fireEvent.click(await screen.findByRole('checkbox', { name: /study assistant/i }));

    // No question, no catalogue — it just turns on.
    expect(setAgent).toHaveBeenCalledWith(true);
    expect(screen.queryByText(/needs a model on this computer/i)).toBeNull();
  });
});
