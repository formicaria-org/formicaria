// **Getting a newer version has to be reachable, visible while it happens, and impossible to finish
// by accident.**
//
// The updater can check, download, verify and swap — but until this panel, nothing let a person start
// any of it, so none of it could reach anyone. What is pinned here is the shape of that path: the
// check is described beside its switch; a failed background check stays silent while an answer to
// *Check now* is shown; the download reports bytes and can be stopped; **one press** gets, checks
// and installs (2026-10-29 — it was three); and on Android the last step is the system installer,
// reached through the shell's bridge rather than a restart the phone cannot do.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';
import { resetUpdater } from './updater.svelte';

const {
  updateStatus,
  updateCheckNow,
  setUpdateCheck,
  updateStart,
  updateCancel,
  updateApply,
  updateRollback,
  reloadPage,
  agentStatus,
} = vi.hoisted(() => ({
  updateStatus: vi.fn(),
  updateCheckNow: vi.fn(),
  setUpdateCheck: vi.fn(),
  updateStart: vi.fn(),
  updateCancel: vi.fn(),
  updateApply: vi.fn(),
  updateRollback: vi.fn(),
  reloadPage: vi.fn(),
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
  reloadPage,
  agentStatus,
}));

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
    category: 'about',
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
  updated_by: null,
  ...over,
});

const downloading = { stage: 'download', done: 1_048_576, total: 4_194_304, error: null };
const ready = { stage: 'ready', done: 4_194_304, total: 4_194_304, error: null };

beforeEach(() => {
  vi.clearAllMocks();
  resetUpdater();
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

  /// **A store install says who updates it** (2026-10-30), and the switch still works.
  it('names the app store that keeps a store install up to date', async () => {
    updateStatus.mockResolvedValue(status({ updated_by: 'F-Droid', check: false }));
    panel();
    expect(
      await screen.findByText(/Updates come from F-Droid, which installed formicaria/),
    ).toBeTruthy();
    await fireEvent.click(await screen.findByLabelText('look for newer versions'));
    expect(setUpdateCheck).toHaveBeenCalledWith(true);
  });

  it('names no store when none installed it', async () => {
    panel();
    await screen.findByLabelText('look for newer versions');
    expect(screen.queryByText(/Updates come from/)).toBeNull();
  });

  it('turns the check off', async () => {
    panel();
    await fireEvent.click(await screen.findByLabelText('look for newer versions'));
    expect(setUpdateCheck).toHaveBeenCalledWith(false);
  });

  it('offers the newer version by name, and starts getting it on a press', async () => {
    panel();
    await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
    expect(updateStart).toHaveBeenCalledOnce();
    expect(await screen.findByText(/downloading/)).toBeTruthy();
  });

  it('can stop a download that is under way', async () => {
    updateStatus.mockResolvedValue(status({ progress: downloading }));
    panel();
    await fireEvent.click(await screen.findByText(/^Stop$/));
    expect(updateCancel).toHaveBeenCalledOnce();
  });

  /// **One press, not three.** It was *Get*, then *Restart into…*, then *Yes, restart*; people
  /// pressed the first and thought they were done. Update now carries on by itself: when the
  /// download is checked and ready it saves what is being typed and restarts into it.
  it('gets, checks and installs on one press', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      panel();
      await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
      expect(updateStart).toHaveBeenCalledOnce();
      expect(updateApply).not.toHaveBeenCalled();
      updateStatus.mockResolvedValue(status({ progress: ready }));
      await vi.advanceTimersByTimeAsync(1600);
      expect(updateApply).toHaveBeenCalledOnce();
      expect(await screen.findByText(/restarting as v0\.6\.1/)).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });

  /// What was typed in the last half second is still in the editor's debounce, which only the page
  /// can flush — the server's `settle` commits what is already on disk.
  it('waits for every open editor to write before restarting', async () => {
    let landed = false;
    const onSaveNow = (e: Event) =>
      (e as CustomEvent<Promise<unknown>[]>).detail.push(
        new Promise((r) => setTimeout(() => r((landed = true)), 50)),
      );
    window.addEventListener('fm-save-now', onSaveNow);
    updateApply.mockImplementation(async () => {
      expect(landed).toBe(true);
      return { restarting: true };
    });
    try {
      updateStatus.mockResolvedValue(status({ progress: ready }));
      panel();
      await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
      await vi.waitFor(() => expect(updateApply).toHaveBeenCalledOnce());
    } finally {
      window.removeEventListener('fm-save-now', onSaveNow);
    }
  });

  /// A refusal (a sync still running) changes nothing, so it is said and Update can be pressed again.
  it('says why an install was refused, and lets it be pressed again', async () => {
    updateStatus.mockResolvedValue(status({ progress: ready }));
    updateApply.mockRejectedValueOnce(new Error('a sync is still running'));
    panel();
    await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
    expect(await screen.findByText('a sync is still running')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Update to v0.6.1' }));
    await vi.waitFor(() => expect(updateApply).toHaveBeenCalledTimes(2));
  });

  /// "This page reconnects by itself" was a sentence with nothing behind it: the message stayed
  /// over an app that had long since restarted. It now asks until the new version answers, through
  /// the silence of the restart, and then loads the page from it.
  it('after restarting, loads the page again once the new version answers', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      updateStatus.mockResolvedValue(status({ progress: ready }));
      panel();
      await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
      expect(await screen.findByText(/restarting as v0\.6\.1/)).toBeTruthy();

      updateStatus.mockRejectedValue(new Error('connection refused'));
      await vi.advanceTimersByTimeAsync(1600);
      expect(reloadPage).not.toHaveBeenCalled();

      updateStatus.mockResolvedValue(
        status({ current: 'v0.6.1', available: null, progress: null }),
      );
      await vi.advanceTimersByTimeAsync(1600);
      expect(reloadPage).toHaveBeenCalledOnce();
    } finally {
      vi.useRealTimers();
    }
  });

  /// A phone cannot restart into a new version; Android's installer has to. Since 2026-10-31 the app
  /// downloads nothing itself: the bridge opens the newest APK in the browser.
  it('on Android, opens the download in the browser instead of restarting', async () => {
    const download = vi.fn(() => 'ok');
    (globalThis as { __fmUpdate?: unknown }).__fmUpdate = { download };
    panel();
    expect(await screen.findByText(/Your browser downloads it/)).toBeTruthy();
    await fireEvent.click(await screen.findByRole('button', { name: 'Update to v0.6.1' }));
    expect(download).toHaveBeenCalledOnce();
    expect(updateStart).not.toHaveBeenCalled();
    expect(screen.queryByText(/Restart into/)).toBeNull();
    expect(await screen.findByText(/Your browser is downloading/)).toBeTruthy();
  });

  /// Being told a fix exists is worth having even where this copy cannot install it.
  it('names the version and the reason when this copy cannot install it', async () => {
    updateStatus.mockResolvedValue(status({ can_install: false, why: 'this folder is protected' }));
    panel();
    expect(await screen.findByText(/v0\.6\.1 is available/)).toBeTruthy();
    expect(screen.getByText(/this folder is protected/)).toBeTruthy();
    expect(screen.queryByRole('button', { name: /Update to/ })).toBeNull();
  });

  /// *Check now* used to exist only in the "up to date" state. A copy that had remembered an old
  /// release then offered it for ever, with nothing to press but *Get* — on a user's phone that
  /// was v0.6.3 with v0.6.5 already out. An offer is not the end of the road.
  it('can look again while a version is on offer, and replaces a stale offer', async () => {
    const stale = new Date(2026, 9, 9, 14, 15).getTime() / 1000;
    updateStatus.mockResolvedValue(status({ available: 'v0.6.3', last_check: stale }));
    updateCheckNow
      .mockReset()
      .mockResolvedValue(status({ available: 'v0.6.5', last_check: stale + 9 }));
    panel();
    expect(await screen.findByText(/^v0\.6\.3 is available$/)).toBeTruthy();
    // When it was found is on screen, which is what makes a stale offer recognisable.
    expect(screen.getByText(/A newer one may\s+be out since/)).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Check now' }));
    expect(updateCheckNow).toHaveBeenCalled();
    expect(await screen.findByText(/^v0\.6\.5 is available$/)).toBeTruthy();
    expect(screen.queryByText(/v0\.6\.3/)).toBeNull();
  });

  /// Not while a download is under way or waiting to be installed: a new name on an old file.
  it('does not offer to look again over a download in flight or one that is ready', async () => {
    for (const stage of ['download', 'ready']) {
      updateStatus.mockResolvedValue(
        status({ progress: { stage, done: 1, total: 2, error: null } }),
      );
      const { unmount } = panel();
      await screen.findAllByText(/v0\.6\.1/);
      expect(screen.queryByRole('button', { name: 'Check now' })).toBeNull();
      unmount();
    }
  });

  /// Silent in the background, answered when asked.
  it('keeps a background failure quiet but reports a failed Check now', async () => {
    updateStatus.mockResolvedValue(status({ available: null, error: 'offline' }));
    updateCheckNow.mockResolvedValue(status({ available: null, error: 'offline' }));
    panel();
    await screen.findByText(/^not looked yet$/);
    expect(screen.queryByText(/Could not look just now/)).toBeNull();
    await fireEvent.click(await screen.findByText(/^Check now$/));
    expect(await screen.findByText(/Could not look just now: offline/)).toBeTruthy();
  });

  /// It read "newest version · You have the newest version — last looked 11/09/2026": the same words
  /// twice, and a date that stayed the same when somebody looked again that day.
  it('says it is up to date once, and when it looked, to the minute', async () => {
    const earlier = new Date(2026, 0, 2, 9, 30).getTime() / 1000;
    updateStatus.mockResolvedValue(status({ available: null, last_check: earlier }));
    panel();
    expect(await screen.findByText(/^up to date$/)).toBeTruthy();
    expect(screen.getByText(/^Last looked .+ at .+\.$/)).toBeTruthy();
    expect(screen.queryByText(/newest version/)).toBeNull();
  });

  /// Both shells answer *Check now* only once the check is done, so the status can never say it is
  /// checking — which on a phone left a tap with no visible effect at all.
  it('shows that Check now is looking while it waits, then the time it looked', async () => {
    let answer: (s: ReturnType<typeof status>) => void = () => {};
    updateStatus.mockResolvedValue(status({ available: null }));
    updateCheckNow.mockImplementation(
      () =>
        new Promise((resolve) => {
          answer = resolve;
        }),
    );
    panel();
    await fireEvent.click(await screen.findByText(/^Check now$/));
    const waiting = await screen.findByRole('button', { name: 'Looking…' });
    expect((waiting as HTMLButtonElement).disabled).toBe(true);
    answer(status({ available: null, last_check: Math.floor(Date.now() / 1000) }));
    expect(await screen.findByText(/^Last looked today at .+\.$/)).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Check now' })).toBeTruthy();
  });

  /// One category at a time: the list of them is how you move, and it says what each is set to.
  it("shows one category at a time, and says each one's state beside its name", async () => {
    updateStatus.mockResolvedValue(status());
    const { container } = panel();
    await screen.findByText(/This machine/);
    const nav = container.querySelector('nav.jump')!;
    const names = [...nav.querySelectorAll('.cat-name')].map((b) => b.textContent);
    expect(names).toEqual(
      expect.arrayContaining(['Look', 'Keyboard', 'Vaults & backup', 'About & updates']),
    );
    expect(nav.textContent).toMatch(/System · One view/);
    const section = (title: RegExp) =>
      [...container.querySelectorAll<HTMLElement>('section')].find((s) =>
        title.test(s.querySelector('h3')?.textContent ?? ''),
      )!;
    // Opened on About: its sections are on screen, the others are not.
    expect(section(/^This machine/).hidden).toBe(false);
    expect(section(/^Theme/).hidden).toBe(true);
    await fireEvent.click(
      [...nav.querySelectorAll('button')].find((b) => /^Look/.test(b.textContent!))!,
    );
    expect(section(/^Theme/).hidden).toBe(false);
    expect(section(/^This machine/).hidden).toBe(true);
    // The actions are not part of Settings at all.
    expect(container.querySelector<HTMLElement>('section[data-cat="actions"]')!.hidden).toBe(true);
    expect(screen.getByLabelText('close')).toBeTruthy();
  });

  it('searches every category at once and says where each match lives', async () => {
    updateStatus.mockResolvedValue(status());
    const { container } = panel();
    await screen.findByText(/This machine/);
    await fireEvent.input(screen.getByLabelText('search settings'), { target: { value: 'theme' } });
    const hits = [...container.querySelectorAll<HTMLElement>('section')].filter((s) => !s.hidden);
    expect(hits.length).toBeGreaterThan(0);
    expect(hits.every((s) => /theme/i.test(s.textContent!))).toBe(true);
    expect(hits.some((s) => s.dataset.found === 'Look')).toBe(true);
    await fireEvent.input(screen.getByLabelText('search settings'), { target: { value: 'zzzz' } });
    expect(await screen.findByText(/Nothing in Settings matches/)).toBeTruthy();
  });

  /// The ＋ opens the actions, and only them.
  it('is a window of actions and nothing else when the ＋ opened it', async () => {
    updateStatus.mockResolvedValue(status());
    const { container } = render(SettingsPanel, {
      onclose: () => {},
      onnewvault: () => {},
      layout: 'auto',
      onlayout: () => {},
      columns: 2,
      oncolumns: () => {},
      theme: 'dark',
      ontheme: () => {},
      commands: [{ group: 'Create', label: 'New note', run: () => {} }],
      section: 'commands',
    } as never);
    expect(await screen.findByRole('button', { name: /New note/ })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Do something' })).toBeTruthy();
    expect(container.querySelector('nav.jump')).toBeNull();
    expect(screen.queryByLabelText('search settings')).toBeNull();
    await screen.findByText(/This machine/);
    const visible = [...container.querySelectorAll<HTMLElement>('section')].filter(
      (s) => !s.hidden,
    );
    expect(visible.map((s) => s.dataset.cat)).toEqual(['actions']);
  });

  /// Backing up by itself is on unless turned off, and the switch is the only place it is said.
  it('offers backing up by itself, on, and remembers when it is turned off', async () => {
    const kept = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => kept.get(k) ?? null,
      setItem: (k: string, v: string) => void kept.set(k, v),
      removeItem: (k: string) => void kept.delete(k),
    });
    try {
      updateStatus.mockResolvedValue(status());
      panel();
      const box = (await screen.findByLabelText('back up by itself')) as HTMLInputElement;
      expect(box.checked).toBe(true);
      expect(screen.getByText(/Never while\s+you are typing/)).toBeTruthy();
      await fireEvent.click(box);
      expect(kept.get('fm-auto-backup')).toBe('off');
    } finally {
      vi.unstubAllGlobals();
    }
  });

  /// It used to show nothing: no rows and no reason, so "there is no Check now" had no answer.
  it('says why, and where to download, when this copy cannot look at all', async () => {
    updateStatus.mockResolvedValue(
      status({
        can_check: false,
        can_install: false,
        available: null,
        why: 'nothing is published for this computer',
      }),
    );
    panel();
    expect(await screen.findByText(/nothing is published for this computer/)).toBeTruthy();
    expect(screen.getByText('Open the download page')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Check now' })).toBeNull();
  });

  /// The other half of the same promise: every copy that *can* look shows the button.
  it('always has Check now when this copy can look, whether or not it can install', async () => {
    for (const can_install of [true, false]) {
      updateStatus.mockResolvedValue(status({ can_install, available: null, last_check: 0 }));
      const { unmount } = panel();
      expect(await screen.findByRole('button', { name: 'Check now' })).toBeTruthy();
      unmount();
    }
  });

  it('shows no update rows at all in a build without the updater', async () => {
    updateStatus.mockRejectedValue(new Error('unknown command: update_status'));
    panel();
    await screen.findByText(/This machine/);
    expect(screen.queryByLabelText('look for newer versions')).toBeNull();
  });
});
