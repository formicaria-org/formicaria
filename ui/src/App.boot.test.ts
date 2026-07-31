// **The tests that would have caught the gray screen.**
//
// The bug: on the owner's phone the app opened to a blank screen on the first launch and worked on
// the second. The Android shell answers commands only once it has opened the vaults, and Tauri
// builds the webview *before* that — so `list_vaults` can answer late, or (when startup failed and
// panicked the hook) never. `App.svelte`'s render gate rendered NOTHING while `vaults === null`, so
// "still opening" and "backend dead" were the same screen: nothing to read, nothing to tap, nothing
// to report. And the failure path rendered the first-run "create your first vault" form, because
// `.catch(() => [])` conflated a refusal with an empty vault list.
//
// Nothing in the suite could see any of it, for one reason: `mock.ts` always succeeded, immediately.
// So these tests drive the app against a backend that refuses, stalls, and goes silent — the three
// things a real one does and a mock never did.
//
// The invariant worth keeping past this bug is the first test's: **no state of the gate may render
// an empty document.** That is the property whose absence made this unreportable, and it holds for
// states nobody has invented yet.

import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults, faults } from './lib/mock';

/// Long enough to clear `Starting`'s deliberate 700 ms silence (which exists so a normal launch
/// never flashes it) plus the app's own first round of effects.
const SETTLED = 1200;

beforeEach(() => {
  clearFaults();
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
  clearFaults();
});

const text = () => document.body.textContent?.trim() ?? '';
/// The app chrome — present only in the fourth gate state.
const appPainted = () => document.body.querySelector('header.topbar') !== null;
/// The first-run form's own words. If this shows up while vaults exist, the app is offering to
/// create a first vault to someone who has several — the specific lie this bug told.
const firstRunOffered = () => /first vault|create a vault|Start empty/i.test(text());

describe('the boot gate never leaves the user with a blank screen', () => {
  it('paints the app when the backend answers, as before', async () => {
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    expect(appPainted()).toBe(true);
    expect(text()).not.toBe('');
  });

  it('shows the startup screen while a slow backend is still opening the vaults', async () => {
    faults([{ cmd: 'list_vaults', mode: 'delay', ms: 30_000 }]);
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    // The point: words, not an empty document — and specifically NOT the app and NOT the
    // first-run form, neither of which we are entitled to claim yet.
    expect(text()).toContain('Opening your vaults');
    expect(appPainted()).toBe(false);
    expect(firstRunOffered()).toBe(false);
  });

  it('escalates and keeps asking when the backend never answers at all', async () => {
    // `hang` is the phone's actual symptom: the shell's event loop had not started, so the invoke
    // was neither resolved nor rejected. It is also the case a `.catch`-driven retry cannot see —
    // a promise that never settles takes neither branch — so the first version of this fix showed
    // a bare "Opening your vaults…" forever, with no reason, no attempt count and no re-ask. What
    // is asserted here is that waiting is *visible* and *bounded*: the wording changes, and a
    // recovery is offered without the user having to guess that the app is stuck.
    faults([{ cmd: 'list_vaults', mode: 'hang' }]);
    render(App);
    await vi.advanceTimersByTimeAsync(1200);
    expect(text()).toContain('Opening your vaults');
    // No button while a slow-but-plausible launch is still in its window: a "Try again" with
    // nothing wrong to fix invites a tap that starts a second boot.
    expect(screen.queryByRole('button', { name: 'Try again' })).toBeNull();

    await vi.advanceTimersByTimeAsync(20_000);
    expect(text()).toMatch(/taking longer than it should/);
    expect(screen.getByRole('button', { name: 'Try again' })).toBeTruthy();
    expect(text()).toMatch(/Tried \d+ times/);
    expect(appPainted()).toBe(false);
  });

  it('recovers by itself from a hang, with no tap, once the backend answers', async () => {
    // The same silent backend, but it comes up: the elapsed-time watchdog must re-ask. Without it
    // the app stays on the startup screen behind a backend that has been ready for minutes.
    faults([{ cmd: 'list_vaults', mode: 'hang', times: 3 }]);
    render(App);
    await vi.advanceTimersByTimeAsync(1200);
    expect(appPainted()).toBe(false);
    await vi.advanceTimersByTimeAsync(8000);
    expect(appPainted()).toBe(true);
  });

  it('keeps the saved views after an early refusal, instead of losing them for the session', async () => {
    // `list_views` fires at t≈0, concurrently with `list_vaults`, and on the phone that is before
    // the shell can answer anything. It used to `.catch(() => [])` once and never ask again, so a
    // badly-timed launch left the sidebar with none of the user's `.view` files all session.
    faults([{ cmd: 'list_views', mode: 'reject', message: 'still opening your vaults', times: 1 }]);
    render(App);
    await vi.advanceTimersByTimeAsync(3000);
    expect(appPainted()).toBe(true);
    // The mock ships saved views; if the retry never happened, none of them are here.
    expect(document.body.textContent).toMatch(/Overdue|view/i);
  });

  it("shows a refusal's own words and does not mistake it for a first run", async () => {
    faults([
      {
        cmd: 'list_vaults',
        mode: 'reject',
        message: 'could not open vaults: open /data/user/0/dev.formicaria.notes/vaults: no such file',
      },
    ]);
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    // Verbatim, because on a phone the screen is the only diagnostic channel there is: Rust's
    // stdout is not routed to logcat, the WebView forwards no console output, and MIUI suppresses
    // our own tag (known-issues.md).
    expect(screen.getByText(/no such file/)).toBeTruthy();
    expect(firstRunOffered()).toBe(false);
    expect(appPainted()).toBe(false);
  });

  it('recovers on its own once the backend comes up', async () => {
    // Two refusals then success: the automatic retry (1.2 s) must get there without the user
    // doing anything, which is what makes "the vaults took a while to open" a non-event.
    faults([{ cmd: 'list_vaults', mode: 'reject', message: 'still opening your vaults', times: 2 }]);
    render(App);
    await vi.advanceTimersByTimeAsync(SETTLED);
    expect(appPainted()).toBe(false);
    await vi.advanceTimersByTimeAsync(4000);
    expect(appPainted()).toBe(true);
  });
});
