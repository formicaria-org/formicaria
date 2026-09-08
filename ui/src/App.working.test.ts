/// **"Some icon rotating like a wheel should be visually present."**
///
/// Reported from the phone, 2026-09-08, after pressing *Get their changes* and seeing the laptop's
/// newer notes fail to appear: *"I tried to back up on the laptop and press again get their
/// changes but nothing… Ok, I see them only now (time issue with pull I guess)."*
///
/// Nothing was broken. The pull was still running — over a phone's network, which is the slowest
/// thing this app does — and the app said so nowhere. Worse, `getTheirChanges` **clears the "get
/// changes" chip as its first act**, so pressing the button deleted the only evidence that anything
/// had started. A slow operation with no sign of life is indistinguishable from a dead button, and
/// the honest response to a dead button is to press it again, which is what happened.
///
/// The state had been there all along: `sync.svelte.ts` tracks `committing` / `pulling` /
/// `pushing`, and its `syncing()` helper says in its own docstring that it is *"what a global
/// 'syncing…' indicator reads"*. A grep for its consumers returned only the definition.
///
/// The wording is unit-tested in `sync.svelte.test.ts`. This is the half that rots: whether the
/// indicator ever reaches the screen, and — just as important — whether it *leaves*.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, faults, reset } from './lib/mock';

beforeEach(() => {
  reset();
  clearFaults();
});
afterEach(() => {
  vi.useRealTimers();
  clearFaults();
  reset();
});

test('the toolbar says it is working while a slow backup runs, and goes quiet when it ends', async () => {
  // The reported shape: the network step takes long enough for a person to wonder.
  faults([{ cmd: 'push', mode: 'delay', ms: 300 }]);
  render(App);

  await fireEvent.click(await screen.findByRole('button', { name: /back up notes/i }));

  // Announced, not merely drawn — the same information for someone who cannot see it spin.
  const working = await screen.findByRole('status');
  expect(working.textContent).toMatch(/Sending…/);

  // **And it must not outlive the work.** A spinner that never stops is worse than none: it
  // teaches the user that the indicator means nothing.
  await vi.waitFor(() => expect(screen.queryByRole('status')).toBeNull(), { timeout: 4000 });
});

test('nothing is spinning when nothing is happening', async () => {
  render(App);
  // Anchored on a control that must render, so this cannot pass by the app failing to boot.
  await screen.findByRole('button', { name: /back up notes/i });
  expect(screen.queryByRole('status')).toBeNull();
});

test('a backup that fails still stops the spinner', async () => {
  // The path that would strand it: an error unwinds the sequence, and an indicator keyed on "did
  // we ever start" rather than "are we still going" would spin forever on a failed send.
  faults([{ cmd: 'push', mode: 'reject', message: 'nope' }]);
  render(App);

  await fireEvent.click(await screen.findByRole('button', { name: /back up notes/i }));
  await vi.waitFor(() => expect(screen.queryByRole('status')).toBeNull(), { timeout: 4000 });
});
