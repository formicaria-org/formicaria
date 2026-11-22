// **The computer's pages back up by itself** (found 2026-11-22: only the older frame ran the loop,
// so formicaria's browser never sent notes by itself). A vault with unsent changes, the clock
// passed: a feature page sends, without a press, exactly as `App.autoBackup.test.ts` expects of the
// older frame; and one that cannot send says so.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';
import { TICK_MS, setAutoBackup, setRunsByItself } from '../lib/autoBackup';
import { setGitRemote } from '../lib/ipc';

const { pushed } = vi.hoisted(() => ({ pushed: vi.fn() }));
vi.mock('../lib/ipc', async (importOriginal) => {
  const real = await importOriginal<typeof import('../lib/ipc')>();
  return {
    ...real,
    push: (message: string, vault = '') => {
      pushed(vault, message);
      return real.push(message, vault);
    },
  };
});

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
  const now = Math.floor(Date.now() / 1000);
  mock.setLastCommits({ personal: now - 600 });
  mock.setLastSent({ personal: now - 3600 }, { personal: 3 });
  setRunsByItself(true);
  history.replaceState(null, '', '/f/group/Library');
});
afterEach(() => {
  setRunsByItself(null);
  vi.useRealTimers();
  vi.unstubAllGlobals();
  history.replaceState(null, '', '/');
});

const PAST_A_TICK = TICK_MS + 5_000;

it('a feature page sends unsent changes without a press', async () => {
  render(Feature);
  await vi.advanceTimersByTimeAsync(PAST_A_TICK);
  expect(pushed).toHaveBeenCalledWith('personal', expect.stringMatching(/^backup: /));
});

it('sends nothing when backing up by itself is off on this device', async () => {
  setAutoBackup(false);
  render(Feature);
  await vi.advanceTimersByTimeAsync(PAST_A_TICK * 2);
  expect(pushed).not.toHaveBeenCalled();
});

it('a vault that cannot send is said on the page', async () => {
  await setGitRemote(
    'https://example.invalid/notes.git',
    { name: 'Ana', email: 'ana@lab.invalid' },
    'personal',
  );
  mock.faults([
    { cmd: 'push', mode: 'reject', message: 'could not resolve host: example.invalid' },
  ]);
  render(Feature);
  await vi.advanceTimersByTimeAsync(PAST_A_TICK);
  const warn = await vi.waitFor(() => {
    const w = screen.getAllByRole('status').find((e) => /not sent/i.test(e.textContent ?? ''));
    if (!w) throw new Error('no warning');
    return w;
  });
  expect(warn.textContent).toMatch(/Back up in Settings/);
});
