// **When the welcome screen appears, and — more importantly — when it does not.**
//
// It sits in the boot gate, above the app and below pairing, so getting its condition wrong shows a
// first-run form to someone who has been using formicaria for months. `outstanding.md` §2.6 named
// the three things that decide whether it is right, and two of them are conditions on this gate:
// the trigger must be cheap (identity comes from `list_vaults`, never from `backup_status`'s
// network `ls-remote` per vault), and it must be hidden entirely where git is absent — a machine
// with no git has no committer to name, so every field on it would be a control that does nothing.

import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults, faults, setMockIdentity } from './lib/mock';

/// jsdom here does not always provide one, and the dismissal is the one thing that cannot be tested
/// without it. A Map-backed stand-in, installed only when it is missing, so this file asserts the
/// same code path a browser takes rather than the app's storage-unavailable fallback.
const store = new Map<string, string>();
if (typeof globalThis.localStorage === 'undefined') {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, String(v)),
      removeItem: (k: string) => void store.delete(k),
      clear: () => store.clear(),
    },
  });
}

/// Long enough to clear `Starting`'s deliberate 700 ms silence plus the app's first effects.
const SETTLED = 1200;
const welcomeShown = () => screen.queryByRole('button', { name: /Start writing/i }) !== null;
const appPainted = () => document.body.querySelector('header.topbar') !== null;

beforeEach(() => {
  clearFaults();
  localStorage.removeItem('fm-welcome-done');
  setMockIdentity(null); // nobody has said who they are — the state the screen exists for
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
  clearFaults();
  setMockIdentity({ name: 'Ada Lovelace', email: 'ada@example.org' }); // the fixtures' own premise
});

describe('the welcome gate', () => {
  it('asks once, when git is here and nobody has said who they are', async () => {
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    expect(welcomeShown()).toBe(true);
    expect(appPainted()).toBe(false);
  });

  it('stays out of the way of someone who already has a committer', async () => {
    setMockIdentity({ name: 'Ada Lovelace', email: 'ada@example.org' });
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    expect(welcomeShown()).toBe(false);
    expect(appPainted()).toBe(true);
  });

  it('never appears where there is no git, which is where none of it would work', async () => {
    // `ping` is what answers "does this machine have git"; a refusal leaves it unknown, and
    // unknown is not "yes" — the gate must not fire on a maybe either.
    faults([{ cmd: 'ping', mode: 'reject', message: 'no git here' }]);
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    expect(welcomeShown()).toBe(false);
    expect(appPainted()).toBe(true);
  });

  it('does not come back once it has been dismissed', async () => {
    localStorage.setItem('fm-welcome-done', '1');
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    // Skipping that did not stick would mean "skip" was really "ask me again next launch".
    expect(welcomeShown()).toBe(false);
    expect(appPainted()).toBe(true);
  });
});
