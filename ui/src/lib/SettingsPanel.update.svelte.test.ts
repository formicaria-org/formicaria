// **Getting a newer version has to be reachable, visible while it happens, and impossible to finish
// by accident.**
//
// The updater can check, download, verify and swap — but until this panel, nothing let a person start
// any of it, so none of it could reach anyone. What is pinned here is the shape of that path: the
// check is described beside its switch; a failed background check stays silent while an answer to
// *Check now* is shown; the download reports bytes and can be stopped; the desktop restart is two
// steps, like *Go back*; and on Android the last step is the system installer, reached through the
// shell's bridge rather than a restart the phone cannot do.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

const {
  updateStatus,
  updateCheckNow,
  setUpdateCheck,
  updateStart,
  updateCancel,
  updateApply,
  updateRollback,
  agentStatus,
} = vi.hoisted(() => ({
  updateStatus: vi.fn(),
  updateCheckNow: vi.fn(),
  setUpdateCheck: vi.fn(),
  updateStart: vi.fn(),
  updateCancel: vi.fn(),
  updateApply: vi.fn(),
  updateRollback: vi.fn(),
  agentStatus: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  updateStatus,
  updateCheckNow,
  setUpdateCheck,
  updateStart,
  updateCancel,
  updateApply,
  updateRollback,
  agentStatus,
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
  can_check: true,
  can_install: true,
  why: '',
  current: 'v0.6.0',
  available: 'v0.6.1',
  previous: null,
  can_go_back: false,
  checking: false,
  check: true,
  last_check: 0,
  error: null,
  progress: null,
  page: 'https://github.com/formicaria-org/formicaria/releases/latest',
  ...over,
});

const downloading = { stage: 'download', done: 1_048_576, total: 4_194_304, error: null };
const ready = { stage: 'ready', done: 4_194_304, total: 4_194_304, error: null };

beforeEach(() => {
  vi.clearAllMocks();
  updateStatus.mockResolvedValue(status());
  updateCheckNow.mockResolvedValue(status());
  setUpdateCheck.mockImplementation(async (check: boolean) => status({ check }));
  updateStart.mockResolvedValue(status({ progress: downloading }));
  updateCancel.mockResolvedValue(status({ progress: null }));
  updateApply.mockResolvedValue({ restarting: true });
  agentStatus.mockRejectedValue(new Error('no assistant here'));
});

afterEach(() => {
  delete (globalThis as { __fmUpdate?: unknown }).__fmUpdate;
});

describe('SettingsPanel — getting a newer version', () => {
  it('says what the check sends, beside the switch that controls it', async () => {
    panel();
    expect(
      await screen.findByText(/Nothing about you, this computer or your notes is sent/),
    ).toBeTruthy();
  });

  it('turns the check off', async () => {
    panel();
    await fireEvent.click(await screen.findByLabelText('look for newer versions'));
    expect(setUpdateCheck).toHaveBeenCalledWith(false);
  });

  it('offers the newer version by name, and starts getting it on a press', async () => {
    panel();
    await fireEvent.click(await screen.findByText(/Get v0\.6\.1/));
    expect(updateStart).toHaveBeenCalledOnce();
    expect(await screen.findByText(/downloading/)).toBeTruthy();
  });

  it('can stop a download that is under way', async () => {
    updateStatus.mockResolvedValue(status({ progress: downloading }));
    panel();
    await fireEvent.click(await screen.findByText(/^Stop$/));
    expect(updateCancel).toHaveBeenCalledOnce();
  });

  /// One press must not replace the running program.
  it('does not restart on the first press — it says what will happen', async () => {
    updateStatus.mockResolvedValue(status({ progress: ready }));
    panel();
    await fireEvent.click(await screen.findByText(/Restart into v0\.6\.1/));
    expect(updateApply).not.toHaveBeenCalled();
    expect(await screen.findByText(/your notes are not touched/)).toBeTruthy();
    await fireEvent.click(await screen.findByText(/Yes, restart/));
    expect(updateApply).toHaveBeenCalledOnce();
  });

  /// A phone cannot restart into a new version; Android's installer has to. The bridge is what the
  /// shell offers for that, and its presence is what decides which button appears.
  it('on Android, hands the checked download to the installer instead of restarting', async () => {
    const install = vi.fn(() => 'ok');
    (globalThis as { __fmUpdate?: unknown }).__fmUpdate = { install };
    updateStatus.mockResolvedValue(status({ progress: ready }));
    panel();
    await fireEvent.click(await screen.findByText(/Install v0\.6\.1/));
    expect(install).toHaveBeenCalledOnce();
    expect(screen.queryByText(/Restart into/)).toBeNull();
    expect(await screen.findByText(/The installer has opened/)).toBeTruthy();
  });

  it('on Android, explains the one-time permission rather than failing silently', async () => {
    (globalThis as { __fmUpdate?: unknown }).__fmUpdate = { install: () => 'permission' };
    updateStatus.mockResolvedValue(status({ progress: ready }));
    panel();
    await fireEvent.click(await screen.findByText(/Install v0\.6\.1/));
    expect(await screen.findByText(/allow installs from formicaria/)).toBeTruthy();
  });

  /// Being told a fix exists is worth having even where this copy cannot install it.
  it('names the version and the reason when this copy cannot install it', async () => {
    updateStatus.mockResolvedValue(status({ can_install: false, why: 'this folder is protected' }));
    panel();
    expect(await screen.findByText(/v0\.6\.1 is available/)).toBeTruthy();
    expect(screen.getByText(/this folder is protected/)).toBeTruthy();
    expect(screen.queryByText(/Get v0\.6\.1/)).toBeNull();
  });

  /// Silent in the background, answered when asked.
  it('keeps a background failure quiet but reports a failed Check now', async () => {
    updateStatus.mockResolvedValue(status({ available: null, error: 'offline' }));
    updateCheckNow.mockResolvedValue(status({ available: null, error: 'offline' }));
    panel();
    await screen.findByText(/Not looked yet/);
    expect(screen.queryByText(/Could not look just now/)).toBeNull();
    await fireEvent.click(await screen.findByText(/^Check now$/));
    expect(await screen.findByText(/Could not look just now: offline/)).toBeTruthy();
  });

  it('shows no update rows at all in a build without the updater', async () => {
    updateStatus.mockRejectedValue(new Error('unknown command: update_status'));
    panel();
    await screen.findByText(/This machine/);
    expect(screen.queryByLabelText('look for newer versions')).toBeNull();
  });
});
