// **The app backs up by itself — on the phone and on the computer.**
//
// `autoBackup.test.ts` pins *when* a vault is due; nothing ran the wiring that acts on it, because
// it sat behind `import.meta.env.PROD`. So the phone — which took itself for a paired tablet and
// returned on every tick — held 46 changes for five days (2026-10-17) with every test green.
// These run the whole app with a vault that has unsent changes, let the clock pass, and expect a
// send. Each fails if the wiring stops starting on that device, for whatever reason.
import { render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import { asDesktop, asPhone } from './lib/harness';
import { TICK_MS, setRunsByItself } from './lib/autoBackup';

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
