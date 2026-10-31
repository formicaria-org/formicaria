// **A newer version is offered where people are, and taken in one press** (2026-10-29).
//
// The daily check found new versions, and the only place it said so was Settings → About &
// updates, which nobody opens; taking one was three presses there. The main window now says so,
// *Update* does everything, and *Later* is a few days' quiet for that version — never for ever.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import UpdateBanner from './UpdateBanner.svelte';
import { refresh, resetUpdater } from './updater.svelte';

const { updateStatus, updateStart, updateCancel, updateApply, reloadPage } = vi.hoisted(() => ({
  updateStatus: vi.fn(),
  updateStart: vi.fn(),
  updateCancel: vi.fn(),
  updateApply: vi.fn(),
  reloadPage: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  updateStatus,
  updateStart,
  updateCancel,
  updateApply,
  reloadPage,
}));

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
  ...over,
});
const downloading = { stage: 'download', done: 1_048_576, total: 4_194_304, error: null };
const ready = { stage: 'ready', done: 4_194_304, total: 4_194_304, error: null };

let kept: Map<string, string>;
beforeEach(() => {
  vi.clearAllMocks();
  kept = new Map();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => kept.get(k) ?? null,
    setItem: (k: string, v: string) => void kept.set(k, v),
    removeItem: (k: string) => void kept.delete(k),
  });
  resetUpdater();
  updateStatus.mockResolvedValue(status());
  updateStart.mockResolvedValue(status({ progress: downloading }));
  updateCancel.mockResolvedValue(status());
  updateApply.mockResolvedValue({ restarting: true });
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  delete (globalThis as { __fmUpdate?: unknown }).__fmUpdate;
});

async function shown() {
  await refresh();
  return render(UpdateBanner);
}

describe('UpdateBanner — a newer version, offered in the main window', () => {
  it('names the new version and the one in use, with Update and Later', async () => {
    await shown();
    expect(screen.getByText(/formicaria v0\.6\.1 is out/)).toBeTruthy();
    expect(screen.getByText(/you have v0\.6\.0/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Update' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Later' })).toBeTruthy();
  });

  it('says nothing when there is nothing newer', async () => {
    updateStatus.mockResolvedValue(status({ available: null }));
    const { container } = await shown();
    expect(container.textContent?.trim()).toBe('');
  });

  /// A copy that can only be told (a protected folder, a run from source) keeps that in Settings:
  /// a line every day offering something that cannot be done here would teach people to ignore it.
  it('says nothing where this copy cannot install it', async () => {
    updateStatus.mockResolvedValue(status({ can_install: false, why: 'protected folder' }));
    const { container } = await shown();
    expect(container.textContent?.trim()).toBe('');
  });

  it('says nothing in a build without the updater, or on a paired device', async () => {
    updateStatus.mockRejectedValue(new Error('unknown command: update_status'));
    const { container } = await shown();
    expect(container.textContent?.trim()).toBe('');
  });

  it('one press: downloads with progress, then installs and restarts', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    expect(updateStart).toHaveBeenCalledOnce();
    expect(await screen.findByText(/Updating to v0\.6\.1/)).toBeTruthy();
    expect(screen.getByRole('status').textContent).toMatch(/downloading…\s+1MB of 4\.2MB/);
    expect(updateApply).not.toHaveBeenCalled();

    updateStatus.mockResolvedValue(status({ progress: ready }));
    await vi.advanceTimersByTimeAsync(1600);
    expect(updateApply).toHaveBeenCalledOnce();
    expect(await screen.findByText(/Restarting as v0\.6\.1/)).toBeTruthy();

    updateStatus.mockResolvedValue(status({ current: 'v0.6.1', available: null }));
    await vi.advanceTimersByTimeAsync(1600);
    expect(reloadPage).toHaveBeenCalledOnce();
  });

  it('can be stopped while downloading, and offers the update again', async () => {
    await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Stop' }));
    expect(updateCancel).toHaveBeenCalledOnce();
    expect(await screen.findByRole('button', { name: 'Update' })).toBeTruthy();
  });

  it('says why a download failed, and can try again', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    updateStatus.mockResolvedValue(
      status({ progress: { stage: 'failed', done: 0, total: null, error: 'checksum mismatch' } }),
    );
    await vi.advanceTimersByTimeAsync(1600);
    expect(await screen.findByText(/checksum mismatch/)).toBeTruthy();
    expect(updateApply).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(updateStart).toHaveBeenCalledTimes(2);
  });

  /// *Later* is quiet for that version for a few days — not for ever, which is how a copy ends up
  /// three versions behind without anyone having decided it should — and a newer version is news.
  it('Later puts it away for days, not for ever, and a newer version is offered at once', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { unmount } = await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(screen.queryByRole('button', { name: 'Update' })).toBeNull();
    unmount();

    // The next start, the same day: still put away.
    resetUpdater();
    const again = await shown();
    expect(screen.queryByRole('button', { name: 'Update' })).toBeNull();
    again.unmount();

    // A newer version than the one put away.
    resetUpdater();
    updateStatus.mockResolvedValue(status({ available: 'v0.6.2' }));
    const newer = await shown();
    expect(screen.getByText(/v0\.6\.2 is out/)).toBeTruthy();
    newer.unmount();

    // The same version, four days on.
    resetUpdater();
    updateStatus.mockResolvedValue(status());
    vi.setSystemTime(Date.now() + 4 * 24 * 60 * 60 * 1000);
    await shown();
    expect(screen.getByRole('button', { name: 'Update' })).toBeTruthy();
  });

  /// A phone downloads nothing itself (2026-10-31): Update opens the newest APK in the browser,
  /// and Android's installer, with its own confirmation, does the rest.
  it('on Android, opens the download in the browser instead of fetching it', async () => {
    const download = vi.fn(() => 'ok');
    (globalThis as { __fmUpdate?: unknown }).__fmUpdate = { download };
    await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Update' }));
    expect(download).toHaveBeenCalledOnce();
    expect(updateStart).not.toHaveBeenCalled();
    expect(updateApply).not.toHaveBeenCalled();
    expect(await screen.findByText(/Your browser is downloading/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Download again' }));
    expect(download).toHaveBeenCalledTimes(2);
  });
});
