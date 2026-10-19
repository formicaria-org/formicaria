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

const {
  agentStatus,
  agentModels,
  chooseAgentModel,
  setAgent,
  removeAgentModel,
  updateAgentModel,
  setListening,
  device,
} = vi.hoisted(() => ({
  agentStatus: vi.fn(),
  agentModels: vi.fn(),
  chooseAgentModel: vi.fn(),
  setAgent: vi.fn(),
  removeAgentModel: vi.fn(),
  updateAgentModel: vi.fn(),
  setListening: vi.fn(),
  // The panel asks `isPhone()`; a test flips this to see the phone's rows. Off = a computer.
  device: { phone: false },
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  agentStatus,
  agentModels,
  chooseAgentModel,
  setAgent,
  removeAgentModel,
  updateAgentModel,
  setListening,
}));
vi.mock('./platform', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./platform')>()),
  isPhone: () => device.phone,
}));
// On a "phone" every other call the panel makes goes through the shell's transport, Tauri's
// `window.__TAURI_INTERNALS__.invoke`. Answer those from the in-memory mock backend, as the dev build
// does, so the rest of the panel loads; the assistant's own calls are the mocks above.
import * as mockBackend from './mock';
(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  invoke: async (_cmd: string, a: { cmd: string; args?: Record<string, unknown> }) =>
    JSON.stringify((await mockBackend.handle(a.cmd, a.args ?? {})) ?? null),
};

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onnewvault: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
    category: 'assistant',
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

// **Better listening** (2026-10-18): every device hears any language with the everyday speech
// model; a larger one is a choice, offered with what it downloads, only once transcription is on.
describe('better listening', () => {
  const on = (over: Record<string, unknown> = {}) =>
    status({
      enabled: true,
      provisioned: true,
      transcribe: true,
      transcribe_available: true,
      listening_better: false,
      listening_better_bytes: 487_601_967,
      ...over,
    });

  it('is offered with its download once transcription is on, and sends the choice', async () => {
    agentStatus.mockResolvedValue(on());
    setListening.mockResolvedValue({ ok: true });
    panel();

    const box = await screen.findByRole('checkbox', { name: /better listening/i });
    expect(screen.getByText(/about 488 MB/)).toBeTruthy();
    await fireEvent.click(box);
    expect(setListening).toHaveBeenCalledWith(true);
  });

  it('is not offered while transcription is off', async () => {
    agentStatus.mockResolvedValue(on({ transcribe: false }));
    panel();

    await screen.findByRole('checkbox', { name: /audio transcription/i });
    expect(screen.queryByRole('checkbox', { name: /better listening/i })).toBeNull();
  });
});

// **The phone's model choice** (Myrme 0.6.0): two models, in a few words — "Faster" and "More
// accurate, slower" — each with what it downloads. A phone always fetches the image reader with the
// model, so the size includes it and there is no "Read images too" box.
describe('the phone chooses between a faster and a more accurate model', () => {
  const phoneModels = [
    {
      name: 'lfm2.5-vl-450m-q4',
      label: 'Faster',
      bytes: 229_313_568,
      license: 'LFM Open License v1.0',
      vision: true,
      mmproj_bytes: 189_126_080,
      default: true,
      running: true,
    },
    {
      name: 'qwen3.5-0.8b',
      label: 'More accurate, slower',
      bytes: 507_154_688,
      license: 'Apache-2.0',
      vision: true,
      mmproj_bytes: 204_987_232,
      default: false,
      running: false,
    },
  ];
  const onPhone =
    <T>(run: () => Promise<T>) =>
    async () => {
      device.phone = true;
      try {
        await run();
      } finally {
        device.phone = false;
      }
    };

  it(
    'asks in a few words on a first enable, and downloads the one picked',
    onPhone(async () => {
      agentStatus.mockResolvedValue(status({ provisioned: false }));
      agentModels.mockResolvedValue(phoneModels.map((m) => ({ ...m, running: false })));
      setAgent.mockReset().mockResolvedValue({ ok: true });
      panel();

      await fireEvent.click(await screen.findByLabelText('study assistant'));
      expect(await screen.findByText('Faster')).toBeTruthy();
      expect(screen.getByText('More accurate, slower')).toBeTruthy();
      // Weights and image reader together: 229.3 + 189.1 MB, 507.2 + 205.0 MB.
      // Beside each model; the default's figure is also the "Total" until another is picked.
      expect(screen.getAllByText(/418\.4MB/).length).toBeGreaterThan(0);
      expect(screen.getAllByText(/712\.1MB/).length).toBeGreaterThan(0);
      expect(screen.queryByText('Read images too')).toBeNull();

      await fireEvent.click(screen.getByDisplayValue('qwen3.5-0.8b'));
      expect(screen.getByText(/Total: 712\.1MB/)).toBeTruthy();
      await fireEvent.click(screen.getByRole('button', { name: /Download and turn on/ }));
      expect(setAgent).toHaveBeenCalledWith(true, 'qwen3.5-0.8b', false);
    }),
  );

  it(
    'switches only when asked, with the size on the button',
    onPhone(async () => {
      agentStatus.mockResolvedValue(status({ enabled: true, provisioned: true }));
      agentModels.mockResolvedValue(phoneModels);
      chooseAgentModel.mockReset().mockResolvedValue({ ok: true });
      panel();

      expect(await screen.findByText('in use')).toBeTruthy();
      expect(screen.queryByRole('button', { name: /and switch/ })).toBeNull();

      await fireEvent.click(await screen.findByDisplayValue('qwen3.5-0.8b'));
      expect(chooseAgentModel).not.toHaveBeenCalled();
      await fireEvent.click(screen.getByRole('button', { name: 'Download 712.1MB and switch' }));
      expect(chooseAgentModel).toHaveBeenCalledWith('qwen3.5-0.8b');
      expect(await screen.findByText(/downloading the model/i)).toBeTruthy();
    }),
  );

  it(
    'after an update, asks once: the faster one picked, the other beside it, no separate offer',
    onPhone(async () => {
      // The phone runs the old 8-bit model — neither choice — and the update offers the new default.
      const offer = {
        name: 'lfm2.5-vl-450m-q4',
        bytes: 418_439_648,
        license: 'LFM Open License v1.0',
        replaces: 'lfm2.5-vl-450m',
      };
      agentStatus.mockResolvedValue(
        status({ enabled: false, provisioned: true, model: 'lfm2.5-vl-450m', update: offer }),
      );
      agentModels.mockResolvedValue(phoneModels.map((m) => ({ ...m, running: false })));
      chooseAgentModel.mockReset().mockResolvedValue({ ok: true });
      updateAgentModel.mockClear();
      panel();

      // One place to choose, even with the assistant off.
      expect(await screen.findByText('More accurate, slower')).toBeTruthy();
      expect(screen.queryByText(/comes with a newer model/i)).toBeNull();
      expect((screen.getByDisplayValue('lfm2.5-vl-450m-q4') as HTMLInputElement).checked).toBe(
        true,
      );

      await fireEvent.click(screen.getByRole('button', { name: 'Download 418.4MB and switch' }));
      expect(chooseAgentModel).toHaveBeenCalledWith('lfm2.5-vl-450m-q4');
      expect(updateAgentModel).not.toHaveBeenCalled();
    }),
  );

  it(
    'still offers a way back when the model on the phone no longer works',
    onPhone(async () => {
      const offer = {
        name: 'lfm2.5-vl-450m-q4',
        bytes: 418_439_648,
        license: 'LFM Open License v1.0',
        replaces: null,
      };
      agentStatus.mockResolvedValue(
        status({
          enabled: true,
          provisioned: false,
          model: null,
          unsupported: true,
          update: offer,
        }),
      );
      agentModels.mockResolvedValue(phoneModels.map((m) => ({ ...m, running: false })));
      panel();
      expect(await screen.findByText(/no longer works with this version/i)).toBeTruthy();
      expect(screen.getByRole('button', { name: 'Download' })).toBeTruthy();
    }),
  );

  it('is not shown on a computer, whose chooser is the whole catalogue', async () => {
    agentStatus.mockResolvedValue(status({ enabled: true, provisioned: true }));
    agentModels.mockResolvedValue(phoneModels);
    panel();
    await screen.findByLabelText('study assistant');
    expect(screen.queryByText('in use')).toBeNull();
  });
});
