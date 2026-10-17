// **The app backs up by itself — on the phone and on the computer.**
//
// `autoBackup.test.ts` pins *when* a vault is due; nothing ran the wiring that acts on it, because
// it sat behind `import.meta.env.PROD`. So the phone — which took itself for a paired tablet and
// returned on every tick — held 46 changes for five days (2026-10-17) with every test green.
// These run the whole app with a vault that has unsent changes, let the clock pass, and expect a
// send. Each fails if the wiring stops starting on that device, for whatever reason.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import { asDesktop, asPhone } from './lib/harness';
import { setGitRemote } from './lib/ipc';
import { RETURN_AFTER_MS, STUCK_AFTER_MS, TICK_MS, setRunsByItself } from './lib/autoBackup';

const { pushed } = vi.hoisted(() => ({ pushed: vi.fn() }));
vi.mock('./lib/ipc', async (importOriginal) => {
  const real = await importOriginal<typeof import('./lib/ipc')>();
  return {
    ...real,
    push: (message: string, vault = '') => {
      pushed(vault, message);
      return real.push(message, vault);
    },
  };
});
vi.mock('mermaid', () => ({ default: { initialize: vi.fn(), render: vi.fn() } }));
vi.mock('katex', () => ({ default: { renderToString: (t: string) => t } }));
vi.mock('katex/dist/katex.min.css', () => ({}));

const lsStore = new Map<string, string>();

beforeEach(() => {
  lsStore.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => lsStore.get(k) ?? null,
    setItem: (k: string, v: string) => void lsStore.set(k, String(v)),
    removeItem: (k: string) => void lsStore.delete(k),
    clear: () => lsStore.clear(),
    key: (i: number) => [...lsStore.keys()][i] ?? null,
    get length() {
      return lsStore.size;
    },
  });
  mock.reset();
  mock.clearFaults();
  pushed.mockClear();
  vi.useFakeTimers();
  // Saved ten minutes ago, sent an hour ago, three changes waiting: due on the next tick.
  const now = Math.floor(Date.now() / 1000);
  mock.setLastCommits({ personal: now - 600 });
  mock.setLastSent({ personal: now - 3600 }, { personal: 3 });
  setRunsByItself(true);
});

afterEach(() => {
  setRunsByItself(null);
  vi.useRealTimers();
  asDesktop();
  vi.unstubAllGlobals();
});

/// One tick of the backup loop, plus the boot and the backup's own round trips.
const PAST_A_TICK = TICK_MS + 5_000;

describe('backing up by itself', () => {
  it('sends unsent changes on the phone without a press', async () => {
    asPhone();
    render(App);
    await vi.advanceTimersByTimeAsync(PAST_A_TICK);
    expect(pushed).toHaveBeenCalledWith('personal', expect.stringMatching(/^backup: /));
  });

  it('sends unsent changes on the computer without a press', async () => {
    render(App);
    await vi.advanceTimersByTimeAsync(PAST_A_TICK);
    expect(pushed).toHaveBeenCalledWith('personal', expect.stringMatching(/^backup: /));
  });

  it('sends nothing when the switch on this device is off', async () => {
    lsStore.set('fm-auto-backup', 'off');
    asPhone();
    render(App);
    await vi.advanceTimersByTimeAsync(PAST_A_TICK);
    expect(pushed).not.toHaveBeenCalled();
  });
});

/// The page going out of sight and coming back, the way a phone or a browser tab does it.
function setHidden(hidden: boolean) {
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => hidden });
  document.dispatchEvent(new Event('visibilitychange'));
}

// **A send that has not happened is shown** (2026-10-17, `decisions.md#sync`). Two signals, and the
// second must not need the loop to have run — the five days were a loop that never started.
describe('saying that changes have not left this device', () => {
  const savedAgo = (s: number) => {
    const now = Math.floor(Date.now() / 1000);
    mock.setLastCommits({ personal: now - s });
    mock.setLastSent({ personal: now - 3600 }, { personal: 3 });
  };

  it('shows the chip when an automatic send fails, with the reason in the hover', async () => {
    savedAgo(60);
    // A vault that has sent before has somewhere it sends to; without one the try reads as
    // "nowhere to send", which is a different sentence.
    await setGitRemote(
      'https://example.invalid/notes.git',
      { name: 'Ana', email: 'ana@lab.invalid' },
      'personal',
    );
    mock.faults([
      { cmd: 'push', mode: 'reject', message: 'could not resolve host: example.invalid' },
    ]);
    render(App);
    await vi.advanceTimersByTimeAsync(2_000);
    expect(screen.queryByText(/3 not sent/)).toBeNull();
    await vi.advanceTimersByTimeAsync(PAST_A_TICK);
    const chip = (await screen.findByText(/3 not sent/)).closest('button')!;
    expect(chip.title).toMatch(/last try failed/);
  });

  it('shows the chip when changes have waited too long, before any send was even tried', async () => {
    savedAgo(STUCK_AFTER_MS / 1000 + 60);
    render(App);
    await vi.advanceTimersByTimeAsync(2_000);
    const chip = (await screen.findByText(/3 not sent/)).closest('button')!;
    expect(chip.title).toMatch(/should have sent them by now/);
  });

  it('leaves it to the three-day chip when backing up by itself is switched off', async () => {
    savedAgo(STUCK_AFTER_MS / 1000 + 60);
    lsStore.set('fm-auto-backup', 'off');
    render(App);
    await vi.advanceTimersByTimeAsync(2_000);
    expect(screen.queryByText(/not sent/)).toBeNull();
  });
});

describe('coming back to the app', () => {
  afterEach(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
  });

  it('backs up at once after a real absence, without waiting for the gap or the tick', async () => {
    render(App);
    await vi.advanceTimersByTimeAsync(PAST_A_TICK); // the tick's backup: the gap starts here
    const before = pushed.mock.calls.length;
    expect(before).toBeGreaterThan(0);

    setHidden(true); // leaving: the gap still holds, so this sends nothing new
    await vi.advanceTimersByTimeAsync(RETURN_AFTER_MS + 1_000);
    expect(pushed.mock.calls.length).toBe(before);

    setHidden(false);
    await vi.advanceTimersByTimeAsync(1_000); // well inside the two-minute gap, and before a tick
    expect(pushed.mock.calls.length).toBe(before + 1);
  });

  it('does not count a glance — a shade, a lock screen — as coming back', async () => {
    render(App);
    await vi.advanceTimersByTimeAsync(PAST_A_TICK);
    const before = pushed.mock.calls.length;
    setHidden(true);
    await vi.advanceTimersByTimeAsync(5_000);
    setHidden(false);
    await vi.advanceTimersByTimeAsync(1_000);
    expect(pushed.mock.calls.length).toBe(before);
  });
});
