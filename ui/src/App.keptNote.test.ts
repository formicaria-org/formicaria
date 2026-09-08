/// **The chip that covers everything the merge settled for you** — and the pull that must refresh it.
///
/// Two shapes share one chip: a field the two devices set differently (both answers kept) and a
/// note one device deleted while the other was editing it (the note kept). `outstanding.md` §2.12
/// owed the second one a *persistent* surface — it had a step line and a dismissible banner, both
/// gone by the next screen, for a decision the app made on the user's behalf.
///
/// **Why they share.** A sixth chip is what a separate surface would have been, and `App.svelte`'s
/// own media query records the measured cost: a toolbar wrapped to four rows on a real phone with
/// the board starting past halfway (2026-07-19).
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, setKept } from './lib/mock';

beforeEach(() => {
  reset();
  clearFaults();
});
afterEach(() => {
  vi.useRealTimers();
  reset();
});

// Real fixture ids. The mock drops a kept row whose note is gone from the store — which is the
// backend's own terminator ("delete it again" needs no record), so a made-up id silently reports
// nothing and a test built on one passes for the wrong reason.
const N1 = 'M0CK0000000000000000000000';
const N2 = 'M0CK0000000000000000000001';

const kept = (over: Record<string, unknown> = {}) => ({
  path: `notes/${N1}.md`,
  id: N1,
  vault: 'personal',
  title: 'the note they deleted',
  ...over,
});

test('a note that came back is counted by the chip and named in the panel', async () => {
  setKept([kept()]);
  render(App);

  // The mock fixture also carries one demoted field, on a different note — so the count is the
  // union, which is the claim: one chip, both shapes, counted by note.
  const chip = await screen.findByRole('button', { name: /2 decided for you/i });
  await fireEvent.click(chip);

  expect(await screen.findByText(/the note they deleted/)).toBeTruthy();
  // Both sections are present, because both have rows.
  expect(document.body.textContent).toContain('Notes that came back');
  expect(document.body.textContent).toContain('Both answers kept');
});

test('the chip appears after the pull that caused it, not after a restart', async () => {
  // **The staleness this closes.** `loadDemoted` ran at boot and from its own panel and nowhere
  // else, so a chip for a disagreement the current pull produced did not show until the app was
  // restarted — a "persistent" surface less prompt than the banner it replaced. Same 2026-08-24
  // lesson as the "not in history" count.
  setKept([]);
  render(App);
  await screen.findByRole('button', { name: /1 decided for you/i });

  // The pull happens, and only now does the backend have something to report.
  setKept([kept(), kept({ path: `notes/${N2}.md`, id: N2, title: 'and another' })]);
  // Through **Back up notes**, which is `commit → pull → push`: the "get their changes" chip only
  // exists behind the production-only remote poll, and the debt is about the surface being right
  // after whichever door the user came through.
  await fireEvent.click(await screen.findByRole('button', { name: /back up notes/i }));

  await vi.waitFor(() =>
    expect(screen.getByRole('button', { name: /3 decided for you/i })).toBeTruthy(),
  );
});

test('acknowledging clears the resurrections and leaves the field disagreement alone', async () => {
  // The two halves terminate differently and must not terminate each other: "I have seen these"
  // is a watermark over resurrections, and a demoted field is cleared by answering it.
  setKept([kept()]);
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /2 decided for you/i }));

  await fireEvent.click(await screen.findByRole('button', { name: /I have seen these/i }));

  await vi.waitFor(() =>
    expect(screen.getByRole('button', { name: /1 decided for you/i })).toBeTruthy(),
  );
});
