/// **"Nothing has been saved here in 39 days"** — the alert that reports an absence.
///
/// Two notes froze a vault for thirty-nine days (2026-07/08). The delete/modify cause is fixed
/// (`47750fb`) and the conflict *kinds* are surfaced now, but the plan's last item is the one that
/// does not depend on either: every other chip in this toolbar names something a detector found,
/// so every one of them is silent when the detector is what broke. This one asks git a single
/// question — when did this vault last save anything — and cannot be blocked by the answer.
///
/// The mock's fixture is the incident: `personal` saved minutes ago, `lab` thirty-nine days ago.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, setLastCommits, setUnrecorded } from './lib/mock';

beforeEach(() => {
  reset();
  clearFaults();
});
afterEach(() => {
  vi.useRealTimers();
  reset();
});

test('a vault that has not saved in weeks says so, by name and by count', async () => {
  render(App);
  const chip = await screen.findByRole('button', { name: /lab: 39 days since a save/i });
  // The per-vault detail the short label had to drop lives in the hover text.
  expect(chip.getAttribute('title')).toContain('lab: 39 days since a save');
});

test('a vault saved this morning is not mentioned, even beside one that has gone quiet', async () => {
  // **Anchored on the chip that must appear.** A bare "no chip is present" assertion would pass
  // before `loadLastSaves` had answered at all — a negative that is true for a moment in every
  // possible implementation, including a broken one. Waiting for `lab` proves the answer landed;
  // `personal` being absent from the label and the hover text is then a claim about the filter.
  // (That a fresh vault produces nothing at all is pinned where the rule lives, in
  // `quietVaults.test.ts`, which needs no clock and no render to say it.)
  const days = (n: number) => Math.floor((Date.now() - n * 86_400_000) / 1000);
  setLastCommits({ personal: days(0), lab: days(39) });
  render(App);
  const chip = await screen.findByRole('button', { name: /lab: 39 days since a save/i });
  expect(chip.getAttribute('title')).not.toContain('personal');
});

test('a vault that has never been committed is not accused of going quiet', async () => {
  // `null` read as `0` is 1970, which renders as twenty thousand days of silence — pointed at the
  // one person who has done nothing wrong yet. Anchored the same way, and that is what makes it
  // bite: with the bug `personal` joins `lab`, so the singular chip becomes "2 vaults: no save in
  // weeks" and this `findByRole` is the assertion that fails.
  setLastCommits({ personal: null, lab: Math.floor((Date.now() - 39 * 86_400_000) / 1000) });
  render(App);
  const chip = await screen.findByRole('button', { name: /lab: 39 days since a save/i });
  expect(chip.getAttribute('title')).not.toContain('personal');
});

test('backing up clears it, so the alert does not outlive the act that answered it', async () => {
  // The `2026-08-24` lesson, applied before it can be reported again: the "not in history" count
  // was read once per vault-list change, so backing up recorded the notes and left the chip
  // saying the old number — which reads, from the only screen the owner uses, as a backup that
  // did not work. A commit is exactly the event this chip measures.
  setUnrecorded([{ vault: 'lab', count: 1, new: 1, modified: 0, deleted: 0, notes: [] }]);
  render(App);
  await screen.findByRole('button', { name: /lab: 39 days since a save/i });

  await fireEvent.click(screen.getByRole('button', { name: /back up notes/i }));

  await vi.waitFor(() =>
    expect(screen.queryByRole('button', { name: /since a save/i })).toBeNull(),
  );
});

test('several quiet vaults are counted rather than listed', async () => {
  // No single number is true of all of them, and the toolbar has three other chips in it.
  const days = (n: number) => Math.floor((Date.now() - n * 86_400_000) / 1000);
  setLastCommits({ personal: days(20), lab: days(39) });
  render(App);
  const chip = await screen.findByRole('button', { name: /2 vaults: no save in weeks/i });
  const title = chip.getAttribute('title') ?? '';
  expect(title).toContain('lab: 39 days since a save');
  expect(title).toContain('personal: 20 days since a save');
});
