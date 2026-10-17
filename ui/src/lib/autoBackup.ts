/// When formicaria backs up by itself.
///
/// **Why it exists** (the owner, 2026-10-11): *"I would have some automatic backup and pull, a user
/// can't always remember to do that."* Dates changed on the phone were not on the desktop, because
/// sending was a press and the press had not happened; from the other device, "nothing arrived"
/// looked exactly like "we are level".
///
/// **What it does is press Back up**, no more: `syncVault`, the one function that saves, gets the
/// other devices' changes and sends, in that order (`decisions.md`, 2026-09-09). There is no second
/// path that only sends, which is what the 2026-07 ruling *auto-push is explicit, never silent*
/// was against.
///
/// **When** is what this module decides, as pure functions so the policy has tests and the app
/// only has wiring:
/// - a vault is due when it holds changes that were never sent, or another device has sent some;
/// - not while writing is still arriving (`QUIET_MS` since the last save), so a burst of typing is
///   one backup and not ten;
/// - not more often than `GAP_MS` per vault, and after a failure not before `RETRY_MS`: an offline
///   laptop must not try every minute for a week;
/// - never for a vault that has never been sent. A first backup chooses where notes go, and that
///   stays a decision somebody makes.

/// Whether this build backs up by itself at all: a real build does; the test build does not,
/// because its mock has nowhere to send. **A function read at call time**, not the inline
/// `import.meta.env.PROD` it replaced: no test build sets that, so until 2026-10-17 no test had
/// ever run the wiring on any device — which is how the phone went five days without sending.
/// A test turns it on with `setRunsByItself(true)`, and back with `null`.
let runsOverride: boolean | null = null;
export function runsByItself(): boolean {
  return runsOverride ?? import.meta.env.PROD;
}
export function setRunsByItself(on: boolean | null): void {
  runsOverride = on;
}

/// The preference, per device, on unless turned off.
export const AUTO_KEY = 'fm-auto-backup';
/// How often the app looks at whether a backup is due. The look itself is local and cheap.
export const TICK_MS = 60_000;
/// Writing must have been still for this long.
export const QUIET_MS = 30_000;
/// The least time between two automatic backups of one vault.
export const GAP_MS = 2 * 60_000;
/// The wait after one that failed.
export const RETRY_MS = 15 * 60_000;

export function autoBackupOn(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setAutoBackup(on: boolean): void {
  try {
    localStorage.setItem(AUTO_KEY, on ? 'on' : 'off');
  } catch {
    /* a browser with no storage keeps the default */
  }
}

/// What the cheap, local status says about one vault (`last_commits`).
export interface Standing {
  vault: string;
  last_commit: number | null;
  last_sent: number | null;
  unsent: number | null;
}

/// What happened the last time a vault was backed up automatically.
export interface Attempt {
  at: number;
  ok: boolean;
}

/// The vaults to back up now.
///
/// `moved` are the vaults another device has sent changes to. `leaving` is the app being put away:
/// the quiet wait is skipped, because there will be no later, but the gaps still hold.
export function dueVaults(
  rows: Standing[],
  moved: string[],
  attempts: Record<string, Attempt | undefined>,
  now: number,
  leaving = false,
): string[] {
  return rows
    .filter((r) => {
      // Never sent: nowhere to send to yet, or a first backup nobody has chosen.
      if (r.last_sent === null) return false;
      const incoming = moved.includes(r.vault);
      if (!r.unsent && !incoming) return false;
      const last = attempts[r.vault];
      if (last && now - last.at < (last.ok ? GAP_MS : RETRY_MS)) return false;
      // `last_commit` is in seconds, like every instant the backend sends.
      const quiet = r.last_commit === null || now - r.last_commit * 1000 >= QUIET_MS;
      return leaving || incoming || quiet;
    })
    .map((r) => r.vault);
}
