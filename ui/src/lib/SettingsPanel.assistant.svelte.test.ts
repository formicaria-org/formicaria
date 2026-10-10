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

const { agentStatus, agentModels, setAgent, removeAgentModel, updateAgentModel } = vi.hoisted(
  () => ({
    agentStatus: vi.fn(),
    agentModels: vi.fn(),
    setAgent: vi.fn(),
    removeAgentModel: vi.fn(),
    updateAgentModel: vi.fn(),
  }),
);
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  agentStatus,
  agentModels,
  setAgent,
  removeAgentModel,
  updateAgentModel,
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
  provisioned_bytes: 0,
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

  it('offers to reclaim the disk, names the figure, and asks first', async () => {
    agentStatus.mockResolvedValue(
      status({ enabled: true, provisioned: true, provisioned_bytes: 2_497_281_664 }),
    );
    agentModels.mockResolvedValue(catalogue);
    removeAgentModel.mockResolvedValue({ freed: 2_497_281_664 });
    panel();

    // The size is on screen before anything is clicked — "delete 2.5GB" is a decision someone can
    // make; "delete the model" is a leap of faith.
    expect(await screen.findByText(/2\.5GB/)).toBeTruthy();
    await fireEvent.click(await screen.findByRole('button', { name: /Remove the model/i }));

    // Armed, not done: one click must not delete gigabytes.
    expect(removeAgentModel).not.toHaveBeenCalled();
    expect(await screen.findByText(/Your notes are untouched/i)).toBeTruthy();

    await fireEvent.click(await screen.findByRole('button', { name: /Yes, remove it/i }));
    expect(removeAgentModel).toHaveBeenCalled();
    expect(await screen.findByText(/2\.5GB is free again/i)).toBeTruthy();
  });

  it('can be backed out of without deleting anything', async () => {
    agentStatus.mockResolvedValue(
      status({ enabled: true, provisioned: true, provisioned_bytes: 2_497_281_664 }),
    );
    agentModels.mockResolvedValue(catalogue);
    panel();

    await fireEvent.click(await screen.findByRole('button', { name: /Remove the model/i }));
    await fireEvent.click(await screen.findByRole('button', { name: /Cancel/i }));
    expect(removeAgentModel).not.toHaveBeenCalled();
  });

  it('offers nothing to remove when there is nothing downloaded', async () => {
    agentStatus.mockResolvedValue(status({ provisioned: false, provisioned_bytes: 0 }));
    agentModels.mockResolvedValue(catalogue);
    panel();
    await screen.findByRole('checkbox', { name: /study assistant/i });
    expect(screen.queryByRole('button', { name: /Remove the model/i })).toBeNull();
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

// **An app update never changes the model behind someone's back.** When a new version names a
// different model, the one already downloaded keeps working and the new one is *offered*: its name
// and size on screen, one button to get it, one to decline. Pinned here: nothing is asked of the
// server until a button is pressed, declining is remembered by the server (not just hidden),
// stopping the download does not turn the assistant off, and a model that no longer works at all
// cannot be declined.
describe('a newer assistant model that came with an update', () => {
  const offer = {
    name: 'lfm2.5-vl-450m',
    bytes: 568_345_184,
    license: 'LFM Open License v1.0',
    replaces: 'lfm2.5-1.2b',
  };
  const on = (over: Record<string, unknown> = {}) =>
    status({ enabled: true, provisioned: true, model: 'lfm2.5-1.2b', update: offer, ...over });

  it('is offered with its size, and says the one in use keeps working', async () => {
    agentStatus.mockResolvedValue(on());
    updateAgentModel.mockClear();
    panel();

    expect(await screen.findByText(/comes with a newer model/i)).toBeTruthy();
    // The figure is the whole download: the model and its image reader together.
    expect(await screen.findByText(/, a 568\.3MB download/)).toBeTruthy();
    expect(
      await screen.findByText(/lfm2\.5-1\.2b, which keeps working until you switch/),
    ).toBeTruthy();
    expect(updateAgentModel).not.toHaveBeenCalled();
  });

  it('downloads only when asked, and Stop then leaves the assistant on', async () => {
    agentStatus.mockResolvedValue(on());
    updateAgentModel.mockReset().mockResolvedValue({ ok: true });
    setAgent.mockClear();
    panel();

    await fireEvent.click(await screen.findByRole('button', { name: 'Download' }));
    expect(updateAgentModel).toHaveBeenCalledWith();
    expect(await screen.findByText(/downloading the model/i)).toBeTruthy();

    await fireEvent.click(await screen.findByRole('button', { name: /Stop/i }));
    expect(updateAgentModel).toHaveBeenLastCalledWith({ cancel: true });
    expect(setAgent).not.toHaveBeenCalled();
  });

  it('can be declined, and tells the server so the offer is not repeated', async () => {
    agentStatus.mockResolvedValue(on());
    updateAgentModel.mockReset().mockResolvedValue({ ok: true });
    panel();

    await fireEvent.click(await screen.findByRole('button', { name: 'Not now' }));
    expect(updateAgentModel).toHaveBeenCalledWith({ dismiss: true });
    expect(screen.queryByText(/comes with a newer model/i)).toBeNull();
  });

  it('cannot be declined when the model on the device no longer works', async () => {
    agentStatus.mockResolvedValue(
      on({
        provisioned: false,
        model: null,
        unsupported: true,
        update: { ...offer, replaces: null },
      }),
    );
    panel();

    expect(await screen.findByText(/no longer works with this version/i)).toBeTruthy();
    expect(await screen.findByRole('button', { name: 'Download' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Not now' })).toBeNull();
  });

  it('says a downloaded model takes over at the next launch', async () => {
    agentStatus.mockResolvedValue(
      on({ update: null, provisioning: { stage: 'restart', done: 0, total: null, error: null } }),
    );
    panel();

    expect(await screen.findByText(/takes over the next time you open formicaria/i)).toBeTruthy();
    expect(screen.queryByText(/downloading the model/i)).toBeNull();
  });
});
