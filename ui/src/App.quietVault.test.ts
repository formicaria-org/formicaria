/// **"Nothing has been saved here in 39 days"** — the alert that reports an absence.
///
/// Two notes froze a vault for thirty-nine days (2026-07/08). The delete/modify cause is fixed
/// (`47750fb`) and the conflict *kinds* are surfaced now, but the plan's last item is the one that
/// does not depend on either: every other chip in this toolbar names something a detector found,
/// so every one of them is silent when the detector is what broke. This one asks git a single
/// question — when did this vault last save anything — and cannot be blocked by the answer.
///
/// The mock's fixture is the incident: `personal` saved minutes ago, `lab` thirty-nine days ago.
///
/// **The chip says `lab-notes`, not `lab`.** A vault's name is local to a machine — two devices
/// that cloned one repository call the same audience different things — so every surface shows the
/// repository behind the remote (`vaults.svelte.ts`). This chip did not, and neither did the
/// backup summary or the Settings vault list: the same vault read as `notes` on one screen and
/// `formicarium-vault` on another, on the owner's phone (reported 2026-09-08). The fixture already
/// modelled it — `lab` carries the label `lab-notes` and `personal` carries none — so these tests
/// had been passing *because* the chip ignored labels.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { backUpFromToolbar } from './lib/harness';
import { clearFaults, faults, reset, setLastCommits, setLastSent, setUnrecorded } from './lib/mock';

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
  const chip = await screen.findByRole('button', { name: /lab-notes: 39 days since a save/i });
  // The per-vault detail the short label had to drop lives in the hover text.
  expect(chip.getAttribute('title')).toContain('lab-notes: 39 days since a save');
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
  const chip = await screen.findByRole('button', { name: /lab-notes: 39 days since a save/i });
  expect(chip.getAttribute('title')).not.toContain('personal');
});

test('a vault that has never been committed is not accused of going quiet', async () => {
  // `null` read as `0` is 1970, which renders as twenty thousand days of silence — pointed at the
  // one person who has done nothing wrong yet. Anchored the same way, and that is what makes it
  // bite: with the bug `personal` joins `lab`, so the singular chip becomes "2 vaults: no save in
  // weeks" and this `findByRole` is the assertion that fails.
  setLastCommits({ personal: null, lab: Math.floor((Date.now() - 39 * 86_400_000) / 1000) });
  render(App);
  const chip = await screen.findByRole('button', { name: /lab-notes: 39 days since a save/i });
  expect(chip.getAttribute('title')).not.toContain('personal');
});

test('backing up clears it, so the alert does not outlive the act that answered it', async () => {
  // The `2026-08-24` lesson, applied before it can be reported again: the "not in history" count
  // was read once per vault-list change, so backing up recorded the notes and left the chip
  // saying the old number — which reads, from the only screen the owner uses, as a backup that
  // did not work. A commit is exactly the event this chip measures.
  setUnrecorded([{ vault: 'lab', count: 1, new: 1, modified: 0, deleted: 0, notes: [] }]);
  render(App);
  await screen.findByRole('button', { name: /lab-notes: 39 days since a save/i });

  await backUpFromToolbar(screen, fireEvent);

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
  expect(title).toContain('lab-notes: 39 days since a save');
  expect(title).toContain('personal: 20 days since a save');
});

/// **The chip and the backup sentence must never contradict each other**, and on a real phone they
/// did (2026-09-08): "“vault” has no destination yet — saved here, but nowhere to send" printed in
/// the same breath as "vault: 38 days since a save". Both cannot be true. The chip had asked
/// `git log`; the sentence had asked nothing — `commitStep` held `CommitResult.committed` and
/// dropped it, and the summary said "saved here" for every destination-less vault regardless.
///
/// Proven red by printing the old single sentence for every `local` vault.
test('a vault with no remote and nothing new does not claim it just committed', async () => {
  // No remote (both fixture vaults have `remote: null`) and both directions rejected: the shape a
  // vault with nowhere to push actually has. Nothing unrecorded, so the commit writes nothing.
  faults([
    { cmd: 'push', mode: 'reject', message: 'no remote configured' },
    { cmd: 'pull', mode: 'reject', message: 'no remote configured' },
  ]);
  render(App);
  await backUpFromToolbar(screen, fireEvent);

  const notice = await screen.findByText(/nothing new to save/i);
  expect(notice.textContent).not.toMatch(/saved here/i);
});

test('and it does say so when it really did commit', async () => {
  // The other half, so the fix is not "never claim a commit" — which would be just as untrue.
  setUnrecorded([{ vault: 'personal', count: 1, new: 1, modified: 0, deleted: 0, notes: [] }]);
  faults([
    { cmd: 'push', mode: 'reject', message: 'no remote configured' },
    { cmd: 'pull', mode: 'reject', message: 'no remote configured' },
  ]);
  render(App);
  await backUpFromToolbar(screen, fireEvent);

  const notice = await screen.findByText(/saved here, but nowhere to send/i);
  expect(notice.textContent).toContain('“personal”');
});

/// **The message that started this**: `“notes” need you: open backup options for detail.` The
/// summary interpolated the vault's *name*, so it announced a vault under a name that appeared on
/// no other screen — the Backup panel, one tap away, called the same vault `formicarium-vault`.
///
/// Proven red by interpolating `v` instead of `labelFor(v)`.
test('the backup summary names a vault the way every other surface does', async () => {
  faults([
    { cmd: 'push', mode: 'reject', message: 'no remote configured' },
    { cmd: 'pull', mode: 'reject', message: 'no remote configured' },
  ]);
  setUnrecorded([{ vault: 'lab', count: 1, new: 1, modified: 0, deleted: 0, notes: [] }]);
  render(App);
  await backUpFromToolbar(screen, fireEvent);

  const notice = await screen.findByText(/saved here, but nowhere to send/i);
  expect(notice.textContent).toContain('“lab-notes”');
  expect(notice.textContent).not.toContain('“lab”');
});

/// **The failure this chip could not see, wired end to end.**
///
/// Every test above drives the *saved* clock, and the owner's 2026-09-08 vault was perfect on that
/// clock: saving every few minutes. What it had was 125 changes that had never left the device,
/// across six days — and no chip. The policy for it is unit-tested in `quietVaults.test.ts`; this
/// is the half that rots, which is whether the number ever reaches the toolbar.
///
/// `lab` is deliberately made *fresh* on the saved clock first, because the fixture has it quiet at
/// 39 days: without that this would pass on the old chip and prove nothing.
test('a vault saving happily but reaching no backup says so, and says how much', async () => {
  const days = (n: number) => Math.floor((Date.now() - n * 86_400_000) / 1000);
  setLastCommits({ personal: days(0), lab: days(0) });
  setLastSent({ personal: days(0), lab: days(6) }, { personal: 0, lab: 125 });
  render(App);

  const chip = await screen.findByRole('button', { name: /lab-notes: 6 days without a backup/i });
  // The count is what makes it worth clicking, and the label had no room for it.
  expect(chip.getAttribute('title')).toContain('lab-notes: 125 changes not sent, 6 days ago');
});

/// **An old send with nothing waiting is not a problem**, and must not become a permanent alarm on
/// a vault someone has finished writing in. The distinction is `unsent`, not the age.
test('a vault sent long ago with nothing new to send is left alone', async () => {
  const days = (n: number) => Math.floor((Date.now() - n * 86_400_000) / 1000);
  setLastCommits({ personal: days(0), lab: days(0) });
  setLastSent({ personal: days(0), lab: days(90) }, { personal: 0, lab: 0 });
  render(App);

  // Anchored on something that must render, so this cannot pass by the app failing to boot.
  await screen.findByRole('button', { name: /settings/i });
  expect(document.body.textContent).not.toMatch(/without a backup/i);
});
