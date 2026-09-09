/// **The app can say it has stopped, which is what issue #2 needed and did not have.**
///
/// The project's only outside bug report ([#2], macOS, v0.2.1, 2026-08-30): a note pane whose whole
/// body read `TypeError: Load failed`, then *"Reloading the page didn't help. I had to rerun the
/// start file. In general, I can't reload the page, I need to restart it to reload it."*
///
/// The cause was never established and is recorded as open. What *is* established from the v0.2.1
/// source is the shape: the failing request got no HTTP response at all, and every background
/// failure in this app is swallowed, so a raw `String(e)` in one pane was the only witness a user
/// got that the server was gone. Reloading could not work — there was nothing to answer it — and
/// nothing said so.
///
/// This is the surface that says so. It does not fix whatever stopped the server; it stops the
/// stopped server from presenting as a wedged interface.
import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { recent } from './lib/ipc';
import { clearFaults, faults, reset } from './lib/mock';
import { resetReachable } from './lib/reachable.svelte';

beforeEach(() => {
  reset();
  clearFaults();
  resetReachable();
});
afterEach(() => {
  vi.useRealTimers();
  clearFaults();
  reset();
  resetReachable();
});

test('an app that has stopped answering says so, and says reloading will not help', async () => {
  // **Mid-session, which is issue #2's situation.** The app booted fine, the reporter was taking a
  // note, and *then* nothing answered. A server already down at boot is a different screen
  // (`Starting`), reached by the startup contract before this banner exists.
  render(App);
  await screen.findByRole('button', { name: /settings/i });

  // `offline`, not `reject`: a browser rejects with a TypeError when nothing is listening and
  // resolves non-ok when the server refuses. Only the first means "it has stopped". `'*'` because
  // a stopped process does not fail one route and answer another.
  faults([{ cmd: '*', mode: 'offline' }]);
  // Driven by real commands rather than by winding the clock: the app's beats were registered
  // during boot, so fake timers installed afterwards never own them. Any two commands will do —
  // the point is that ordinary traffic is what notices, which is the whole design.
  await recent().catch(() => null);
  await recent().catch(() => null);

  const banner = await screen.findByRole('alert', {}, { timeout: 5000 });
  expect(banner.textContent).toMatch(/formicaria has stopped/i);
  // The two things the reporter needed and could not get: that their notes are fine, and why the
  // obvious move did not work.
  expect(banner.textContent).toMatch(/notes are files on your disk and are safe/i);
  expect(banner.textContent).toMatch(/reloading first cannot work/i);
});

test('a refusal is not a stopped app — an error status is proof of life', async () => {
  // The distinction the whole surface turns on. `reject` is the server answering "no"; announcing
  // a dead app for that would fire the loudest sentence in the program on an ordinary failure.
  faults([
    { cmd: 'recent', mode: 'reject', message: 'nope' },
    { cmd: 'board', mode: 'reject', message: 'nope' },
  ]);
  render(App);

  await screen.findByRole('button', { name: /settings/i });
  expect(screen.queryByText(/formicaria has stopped/i)).toBeNull();
});

test('nothing is announced when the app is answering normally', async () => {
  render(App);
  await screen.findByRole('button', { name: /settings/i });
  expect(screen.queryByText(/formicaria has stopped/i)).toBeNull();
});
