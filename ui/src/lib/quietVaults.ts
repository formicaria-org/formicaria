/// When to say that a vault has stopped saving, and what to say.
///
/// **The signal that depends on nothing else working.** Two notes froze a vault for thirty-nine
/// days (2026-07/08). Every surface that could have named the cause was either absent — the
/// conflict *kinds* were not listed anywhere — or blocked by the same freeze that caused it, and
/// the vault sat there looking exactly like a vault nobody had written in. What was missing was
/// the crude fact underneath all of them: *nothing has been saved here in five weeks*. It is
/// deliberately incurious about why, which is the entire reason it would have caught a cause
/// nobody had thought of yet.
///
/// A pure module, like `remotePoll`, so the policy is testable without mounting the app or
/// waiting on a clock — and because the policy is a *sentence*, and a sentence is the thing worth
/// testing.

import type { LastCommit } from './types';

/// Silence worth mentioning. Two weeks, and the size is the argument: a week off is a holiday, and
/// a chip that fires on one would be noise a person learns to read past — which is how the
/// thirty-nine days would have passed anyway. Fourteen days also makes "in weeks" true of every
/// vault this ever reports, which is what lets the plural wording stay honest without a number.
export const QUIET_AFTER_DAYS = 14;

/// Work that has never left the device, and how long is too long.
///
/// **Deliberately far shorter than `QUIET_AFTER_DAYS`, and the asymmetry is the argument.** Silence
/// is merely *suspicious* — it might be a holiday, which is why fourteen days had to be long enough
/// not to nag. Unsent work is a **single point of failure that grows**: those notes exist in exactly
/// one place, and what a wrong guess costs is not a late warning but the writing itself. Three days
/// catches the six-day gap this was written for and stays silent for anyone who backs up twice a
/// week.
export const UNSENT_AFTER_DAYS = 3;

const DAY_MS = 86_400_000;

/// Which absence this is.
///
/// `saved` — nothing has been written into history here. `sent` — history is moving, but none of
/// it is reaching a backup. They are different failures with different fixes, and the chip has to
/// know which it is looking at: backing up will not unfreeze a vault stuck mid-merge, and
/// resolving a merge will not send six days of work.
export type Absence = 'saved' | 'sent';

/// One vault that needs saying something about, and for how long.
export interface QuietVault {
  vault: string;
  days: number;
  kind: Absence;
  /// How much has not left the device. Only on `sent`; it is the fact that makes the chip worth
  /// clicking, and it goes in the hover text where there is room for it.
  unsent?: number;
}

/// The vaults worth mentioning, longest silence first.
///
/// **`null` is skipped, not treated as forever.** A vault that has never been committed has not
/// *stopped* doing anything — it is a first run, or a vault someone just made, and the surfaces
/// that own that case are the welcome screen and "not in history". Reporting it here as an
/// enormous number would put an alarming chip in front of the one user who has done nothing wrong.
///
/// A commit dated in the future — clock skew between two devices, which is ordinary — gives a
/// negative age and is simply not quiet.
export function quietVaults(rows: LastCommit[], now: number): QuietVault[] {
  const age = (at: number | null) => (at === null ? null : Math.floor((now - at * 1000) / DAY_MS));
  return rows
    .flatMap((r): QuietVault[] => {
      // **The deeper fault wins.** A vault that cannot save is broken; one that has not been sent
      // is behind. Reporting the second while the first is true points at the wrong problem — and
      // a backup will not fix a vault frozen mid-merge, which is how the thirty-nine days passed.
      const saved = age(r.last_commit);
      if (saved !== null && saved >= QUIET_AFTER_DAYS) {
        return [{ vault: r.vault, days: saved, kind: 'saved' }];
      }
      // `unsent` of `0` is "all sent", and an old send with nothing waiting is *fine* — a vault
      // someone has finished with must not carry an alarm forever. A `null` send is "never sent",
      // which is the Backup panel's subject and a first run's normal state, not an age.
      const sent = age(r.last_sent);
      if (sent !== null && sent >= UNSENT_AFTER_DAYS && r.unsent) {
        return [{ vault: r.vault, days: sent, kind: 'sent', unsent: r.unsent }];
      }
      return [];
    })
    .sort((a, b) => b.days - a.days || a.vault.localeCompare(b.vault));
}

/// The chip's words. Short, because it sits in a toolbar beside three other chips.
///
/// Named for one vault and counted for several — the same shape the "someone pushed" chip uses,
/// for the same reason: with one vault the name *is* the useful half, and with several no single
/// number is true of all of them. "in weeks" is safe at any length this function is called with,
/// since `QUIET_AFTER_DAYS` is two of them.
/// **`label` resolves what a vault is *called*, which is not its name.** Two devices that cloned
/// one repository name the same audience differently — the owner's laptop says `vault` where the
/// phone says `notes` — so every surface shows the repository behind the remote instead
/// (`vaults.svelte.ts`). Injected rather than imported so this module stays a pure function of
/// its arguments, which is what lets its tests need no clock, no store and no render.
export function quietLabel(q: QuietVault[], label: (v: string) => string = (v) => v): string {
  if (!q.length) return '';
  if (q.length === 1) {
    const [v] = q;
    return v.kind === 'saved'
      ? `${label(v.vault)}: ${v.days} days since a save`
      : `${label(v.vault)}: ${v.days} days without a backup`;
  }
  // **Only claim what is true of all of them.** With two vaults failing in two different ways, a
  // sentence about either is a sentence that is false about the other — and the panel behind the
  // chip is where they get told apart anyway.
  if (q.every((v) => v.kind === 'saved')) return `${q.length} vaults: no save in weeks`;
  if (q.every((v) => v.kind === 'sent')) return `${q.length} vaults without a backup`;
  return `${q.length} vaults need attention`;
}

/// The hover text, which is where the per-vault detail goes when the label had to be short.
export function quietTitle(q: QuietVault[], label: (v: string) => string = (v) => v): string {
  return q
    .map((v) =>
      v.kind === 'saved'
        ? `${label(v.vault)}: ${v.days} days since a save`
        : `${label(v.vault)}: ${v.unsent} change${v.unsent === 1 ? '' : 's'} not sent, ${v.days} days ago`,
    )
    .join(' · ');
}
