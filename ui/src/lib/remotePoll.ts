/// When to ask whether a collaborator has pushed.
///
/// `backup_status` shells `ls-remote` **per vault** — a real network round trip each. The desktop
/// can afford that on a 45 s timer; a phone cannot, and was doing it anyway.
///
/// **How it reached the phone.** The effect in `App.svelte` gated on `import.meta.env.PROD`, and a
/// Tauri build *is* `PROD` — `ipc.ts` says so twice, in the two other places that trap has already
/// been sprung. So the shipped APK ran a per-vault network poll every 45 seconds, plus one on
/// every window `focus`, on mobile data, on a battery. Before the IPC commands were made `(async)`
/// it was worse than costly: the round trip ran inside a *blocking* bridge call, so returning to
/// the app parked the UI thread until every vault's remote answered. That is a large share of "it
/// lags", and all of it for a nudge nobody is waiting on.
///
/// A pure module so the policy is testable without mounting the app or waiting on a clock.

/// The desktop's periodic check.
export const DESKTOP_POLL_MS = 45_000;

/// The floor between foreground checks on a phone.
///
/// Android delivers `focus`/`visibilitychange` on every return to the app — including the ones a
/// notification shade or a lock screen causes — so an ungated foreground check is not obviously
/// cheaper than the interval it replaced. Five minutes is well inside "I want to know before I
/// start editing" and well outside "every time the screen wakes".
export const PHONE_FOREGROUND_GAP_MS = 5 * 60_000;

/// How often to poll on a timer, or `null` for "never — only when the user comes back".
///
/// `mobile-design.md` calls for exactly this under *"don't port desktop's polling"*: convert the
/// 45 s `remote_moved` poll to a foreground check.
export function pollIntervalMs(phone: boolean): number | null {
  return phone ? null : DESKTOP_POLL_MS;
}

/// Is a foreground check due, given when the last one ran?
///
/// `last === 0` means "never checked", which is always due — otherwise a phone that started up
/// less than five minutes ago would skip its first foreground check and could sit unaware of a
/// collaborator's push for the whole gap. The caller seeds `last` from the startup check so the
/// two do not fire back to back.
///
/// The desktop has no floor: its focus events are user-initiated, and it is on mains power.
export function foregroundCheckDue(phone: boolean, now: number, last: number): boolean {
  if (!phone) return true;
  if (last === 0) return true;
  return now - last >= PHONE_FOREGROUND_GAP_MS;
}
