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

const DAY_MS = 86_400_000;

/// One vault that has gone quiet, and for how long.
export interface QuietVault {
  vault: string;
  days: number;
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
  return rows
    .flatMap((r) =>
      r.last_commit === null
        ? []
        : [{ vault: r.vault, days: Math.floor((now - r.last_commit * 1000) / DAY_MS) }],
    )
    .filter((q) => q.days >= QUIET_AFTER_DAYS)
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
/// (`vaultLabels.svelte.ts`). Injected rather than imported so this module stays a pure function of
/// its arguments, which is what lets its tests need no clock, no store and no render.
export function quietLabel(q: QuietVault[], label: (v: string) => string = (v) => v): string {
  if (!q.length) return '';
  if (q.length === 1) return `${label(q[0].vault)}: ${q[0].days} days since a save`;
  return `${q.length} vaults: no save in weeks`;
}

/// The hover text, which is where the per-vault detail goes when the label had to be short.
export function quietTitle(q: QuietVault[], label: (v: string) => string = (v) => v): string {
  return q.map((v) => `${label(v.vault)}: ${v.days} days since a save`).join(' · ');
}
