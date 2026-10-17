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
///
/// `phase` and `error` are kept since 2026-10-17 so a failure can be **said**, not only waited out:
/// *"a failure is quiet here"* was the 2026-10-11 ruling, and five silent days on the owner's phone
/// were what it cost (`decisions.md#sync`, *one person's devices are kept level*).
export interface Attempt {
  at: number;
  ok: boolean;
  /// What `syncVault` came back with: `synced`, `conflicts`, `failed`, `local`.
  phase?: string;
  /// The raw reason, when it failed — `plainError` turns it into words.
  error?: string;
}

/// **Why a backup is being asked for.** Each occasion relaxes the waits differently, and the
/// differences are the policy (`decisions.md#sync`, 2026-10-17):
///
/// - `tick` — the once-a-minute look. Every wait holds.
/// - `leaving` — the app is being put away. No quiet wait (there may be no later); the gaps hold,
///   so an offline phone does not retry on every screen-off.
/// - `returning` — back after `RETURN_AFTER_MS` away. No quiet wait and no gap: this is the moment a
///   stale device is about to be written on. A failure's wait still holds.
/// - `editing` — a note is about to be edited in a vault another device has sent to. Only for
///   *incoming* changes, and past every wait but a failure's: the point is to be level before the
///   first keystroke.
/// - `left-note` — a note was left. No quiet wait, and `LEFT_NOTE_GAP_MS` instead of `GAP_MS`, so
///   the other device has it before it is picked up.
/// - `online` — the network came back. Past a failure's wait, which was waiting for exactly this.
export type Occasion = 'tick' | 'leaving' | 'returning' | 'editing' | 'left-note' | 'online';

/// Away for at least this long counts as coming back; a notification shade or a lock screen does not.
export const RETURN_AFTER_MS = 30_000;
/// The gap after leaving a note: long enough that flicking between notes is one backup.
export const LEFT_NOTE_GAP_MS = 20_000;

/// The vaults to back up now.
///
/// `moved` are the vaults another device has sent changes to; `occasion` is why this is being asked
/// (see [`Occasion`]).
export function dueVaults(
  rows: Standing[],
  moved: string[],
  attempts: Record<string, Attempt | undefined>,
  now: number,
  occasion: Occasion = 'tick',
): string[] {
  return rows
    .filter((r) => {
      // Never sent: nowhere to send to yet, or a first backup nobody has chosen.
      if (r.last_sent === null) return false;
      const incoming = moved.includes(r.vault);
      if (occasion === 'editing') return incoming && !failedRecently(attempts[r.vault], now);
      if (!r.unsent && !incoming) return false;
      const last = attempts[r.vault];
      if (last) {
        const since = now - last.at;
        if (!last.ok && occasion !== 'online' && since < RETRY_MS) return false;
        const gap = occasion === 'left-note' ? LEFT_NOTE_GAP_MS : GAP_MS;
        if (last.ok && occasion !== 'returning' && since < gap) return false;
      }
      if (occasion !== 'tick') return true;
      // `last_commit` is in seconds, like every instant the backend sends.
      const quiet = r.last_commit === null || now - r.last_commit * 1000 >= QUIET_MS;
      return incoming || quiet;
    })
    .map((r) => r.vault);
}

function failedRecently(last: Attempt | undefined, now: number): boolean {
  return !!last && !last.ok && now - last.at < RETRY_MS;
}

/// Changes held here longer than this, with backing up by itself on, mean it is not working.
///
/// A working loop sends within about three and a half minutes of the last save (half a minute's
/// quiet, a minute's tick, two minutes' gap), so ten minutes is not a slow send — it is a loop that
/// has stopped, **whatever stopped it**. That independence is the point: the five days the phone held
/// its notes were a loop that never started, and an alert that waited for a failure to report would
/// have said nothing at all.
export const STUCK_AFTER_MS = 10 * 60_000;

/// A vault whose changes are not leaving this device, and how it is known.
///
/// `failed` — the last automatic attempt came back without sending. `stuck` — changes have waited
/// past `STUCK_AFTER_MS` and no attempt has said why.
export interface Unsent {
  vault: string;
  kind: 'failed' | 'stuck';
  unsent: number;
  /// Minutes since the newest change was saved here.
  minutes: number;
  phase?: string;
  error?: string;
}

/// The vaults to warn about. Nothing when backing up by itself is off: the person chose to press,
/// and the three-day chip (`quietVaults`) is still there for them.
export function unsentVaults(
  rows: Standing[],
  attempts: Record<string, Attempt | undefined>,
  now: number,
  on: boolean,
): Unsent[] {
  if (!on) return [];
  return rows.flatMap((r): Unsent[] => {
    // A vault never sent is the Back up panel's subject, and `0` is "all sent".
    if (r.last_sent === null || !r.unsent) return [];
    const minutes = r.last_commit === null ? 0 : Math.floor((now - r.last_commit * 1000) / 60_000);
    const last = attempts[r.vault];
    if (last && !last.ok) {
      return [
        {
          vault: r.vault,
          kind: 'failed',
          unsent: r.unsent,
          minutes,
          phase: last.phase,
          error: last.error,
        },
      ];
    }
    if (r.last_commit !== null && now - r.last_commit * 1000 >= STUCK_AFTER_MS) {
      return [{ vault: r.vault, kind: 'stuck', unsent: r.unsent, minutes }];
    }
    return [];
  });
}

/// The chip's words: short, and in the person's words — *sent*, never a git verb.
export function unsentLabel(u: Unsent[], label: (v: string) => string = (v) => v): string {
  if (!u.length) return '';
  if (u.length > 1) return `${u.length} vaults not sent`;
  const [v] = u;
  return `${label(v.vault)}: ${v.unsent} not sent`;
}

/// The hover text: what happened, and that Back up is where to see it and send now. `reason`
/// turns a raw failure into a sentence (`plainError`), injected so this stays a pure function.
export function unsentTitle(
  u: Unsent[],
  label: (v: string) => string = (v) => v,
  reason: (raw: string) => string = (raw) => raw,
): string {
  const one = (v: Unsent) => {
    const what = `${v.unsent} change${v.unsent === 1 ? '' : 's'} saved here and not sent`;
    const why =
      v.kind === 'stuck'
        ? `for ${v.minutes} minutes — backing up by itself should have sent ${v.unsent === 1 ? 'it' : 'them'} by now`
        : v.phase === 'conflicts'
          ? 'because some notes have conflicting edits to settle first'
          : v.error
            ? `— the last try failed: ${reason(v.error)}`
            : '— the last try did not send';
    return `${label(v.vault)}: ${what} ${why}.`;
  };
  return u.map(one).join(' ') + ' Open Back up to send now.';
}
