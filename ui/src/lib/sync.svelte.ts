// The sync loop, as one reactive singleton — so every trigger (a debounced edit, a
// "someone pushed" nudge, the backup panel's own button) runs the *same* sequence, and so
// the result is something the UI can show rather than something a `.catch(() => {})` ate.
//
// **The sequence is the whole point, and it is deliberately not a loop.**
//
//     commit → push
//              ├─ ok                 → synced
//              └─ rejected → pull ───┼─ merged     → push once more → synced | failed
//                                    ├─ conflicts  → STOP, name the notes
//                                    └─ up-to-date → STOP, the push failed for
//                                                    some other reason (auth, offline)
//
// Why not "auto-push after auto-commit", the obvious version: `push_squashed` is *designed*
// to refuse when the remote moved and to tell a human, and it deliberately never fetches
// (fetching advances the tracking ref, and an un-advanced tracking ref is what its ancestry
// guard survives on). Automate the push without automating the telling and a moved remote
// becomes a rejected push retried forever, silently — which is the documented
// "auto-commit is best-effort and silent" failure, moved onto the sync path where it costs
// more.
//
// So: **exactly one retry**, and every terminal state is nameable. In particular a pull that
// conflicts must never be followed by a push — that would publish conflict markers as though
// they were content.

import { backupStatus, commit, push, pull } from './ipc';
import type { CommitResult, PullResult } from './types';

/**
 * The three operations the sequence is made of, injectable so every branch of it can be
 * driven by a test. The interesting states — a remote that moved, a merge that conflicts,
 * a push that fails for its own reasons — are exactly the ones that are hard to stage
 * against a real git remote and easy to get wrong.
 */
export interface SyncOps {
  commit(message: string, vault: string): Promise<CommitResult>;
  push(message: string, vault: string): Promise<unknown>;
  pull(vault: string): Promise<PullResult>;
  /// Does this vault have a remote at all? **Asked, never inferred from a push error** — the same
  /// discipline the pull-after-push-rejection follows a few lines down ("we cannot tell them apart
  /// from the message, so ask git"). Only called on the failure path, so the happy path pays nothing:
  /// `backup_status` shells out `git ls-remote` per vault and is the one call in this file that is
  /// genuinely slow.
  hasRemote(vault: string): Promise<boolean>;
}

const realOps: SyncOps = {
  commit,
  push,
  pull,
  hasRemote: async (vault: string) => {
    const status = await backupStatus().catch(() => null);
    const v = status?.vaults.find((s) => s.name === vault);
    // **Unknown counts as "has one".** A status call that failed must not turn a real push failure
    // into a reassuring "no remote yet" — that would hide a broken backup behind a calm sentence.
    return v ? v.remote !== null : true;
  },
};

/** Where one vault stands. `idle` means nothing has been attempted since the last success. */
export type SyncPhase =
  | 'idle'
  | 'committing'
  | 'pushing'
  | 'pulling'
  | 'synced'
  | 'conflicts'
  /// **This vault has no git remote yet**, so there is nowhere to send anything. Whether it also
  /// *committed* is a separate fact, carried in `committed` — the summary sentence says "committed
  /// here, but nowhere to send", and until 2026-09-08 nobody had checked that half of it. Not a
  /// failure — nothing is wrong and nothing the user can fix by retrying — but emphatically not
  /// `synced` either, because the notes have not left the device. A vault with no remote used to
  /// come back `failed` and be listed under "Backup needs you", which put a vault that is working
  /// exactly as configured in the same sentence as a merge conflict, and (reported 2026-07-31) read
  /// as though it were blocking the vaults that *do* have remotes. It never was — they are pushed
  /// independently — but the message could not say so.
  | 'local'
  | 'failed';

export interface VaultSync {
  phase: SyncPhase;
  /** Notes with conflict markers, waiting for a human. Only set in the `conflicts` phase. */
  conflicts: string[];
  /** Why it stopped, for the `failed` phase — shown to the user, never swallowed. */
  error?: string;
  /** Commits pulled in on the way, so the UI can say "3 changes arrived". */
  merged: number;
  /** Notes a pull **kept** because the other device had deleted them while this one edited them
   *  (`decisions.md`, 2026-09-07). Carried here rather than reported only inside the Backup panel:
   *  a pull also happens from the "someone pushed" chip, and a decision the app made on the user's
   *  behalf must not depend on which door they came through. */
  kept: string[];
  /** Whether the commit at the start of this run actually wrote anything. **Carried because the UI
   *  says so out loud**: "committed here, but nowhere to send" is a claim, and `commitStep` had
   *  `CommitResult.committed` in its hand and dropped it. Observed on a phone (2026-09-08) telling
   *  the user a vault had just committed while the quiet-vault chip — which reads `git log` — said
   *  nothing had been saved there in 38 days. Both cannot be true. */
  committed: boolean;
}

const state = $state<{ byVault: Record<string, VaultSync> }>({ byVault: {} });

function set(vault: string, patch: Partial<VaultSync>): void {
  const prev = state.byVault[vault] ?? {
    phase: 'idle' as SyncPhase,
    conflicts: [],
    merged: 0,
    kept: [],
    committed: false,
  };
  state.byVault[vault] = { ...prev, ...patch };
}

/** This vault's sync state. Reactive. */
export function syncFor(vault: string): VaultSync {
  return (
    state.byVault[vault] ?? { phase: 'idle', conflicts: [], merged: 0, kept: [], committed: false }
  );
}

/** Every vault currently mid-flight — what a global "syncing…" indicator reads. */
export function syncing(): string[] {
  return Object.entries(state.byVault)
    .filter(([, s]) => s.phase === 'committing' || s.phase === 'pushing' || s.phase === 'pulling')
    .map(([v]) => v);
}

/** The steps a person actually waits through, slowest-to-explain first. */
const IN_FLIGHT = ['pulling', 'pushing', 'committing'] as const;
export type BusyPhase = (typeof IN_FLIGHT)[number];

/**
 * The one step a single global indicator should name, or `null` when nothing is running.
 *
 * **This is what `syncing()` was written for and never used to do.** That function's own docstring
 * calls itself "what a global 'syncing…' indicator reads", and until 2026-09-08 a grep for its
 * consumers returned only its definition: the app committed, pulled over the network and pushed
 * with nothing on screen to say so. Reported from a phone — *"I pressed get their changes… Ok, I
 * see them only now (time issue with pull I guess). Some icon rotating like a wheel should be
 * visually present"* — where it is worst, because a phone's pull is the slowest one there is and
 * `getTheirChanges` **clears the "get changes" chip before it starts**, so pressing the button
 * deleted the only evidence that anything was happening.
 *
 * **Ordered, because there is one line to spend.** With two vaults in flight at different steps,
 * naming the local one would describe the half the user is not waiting for; committing is a
 * moment's work on disk and the network steps are the wait.
 */
export function syncingPhase(): BusyPhase | null {
  const live = new Set(Object.values(state.byVault).map((s) => s.phase));
  return IN_FLIGHT.find((p) => live.has(p)) ?? null;
}

/**
 * What the indicator says. Pure, so the sentence can be tested without a render — the same reason
 * `quietVaults` keeps its wording in a plain module.
 *
 * Plain words, per `decisions.md` (*the app speaks the user's words, not git's*): a person waiting
 * on a spinner is the last person who should have to translate "pushing".
 */
export function busyLabel(phase: BusyPhase | null): string {
  if (phase === 'committing') return 'Saving…';
  if (phase === 'pulling') return 'Getting changes…';
  if (phase === 'pushing') return 'Sending…';
  return '';
}

/**
 * A send refused because the other side is ahead — the one failure here with a single, obvious
 * remedy, and a button that performs it.
 *
 * Matched on the *shape* of the sentence rather than on an exact string: the subprocess backend
 * relays git's wording, libgit2 writes its own, and both change between versions. All three
 * spellings say the same thing, and the remedy is the same for all of them.
 */
const AHEAD_OF_US =
  /(non-fast-forward|not present locally|do not have locally|contains work that you do not have|fetch first)/i;

/**
 * The headline for a failed send, in words the reader already has.
 *
 * The owner's phone, 2026-09-08, showed this as the entire message: *"io error cannot push because
 * a reference that you are trying to update on the remote contains commits that are not present
 * locally"*. They decoded it themselves and then did by hand the one thing the app could have
 * offered. `decisions.md` (*the app speaks the user's words, not git's*) permits git's vocabulary
 * in a **diagnostic**; this was the headline, for a state with exactly one remedy.
 *
 * **Anything unrecognised is returned untouched, deliberately.** Flattening an unknown fault into a
 * friendly shrug would throw away its only diagnosis, which is a worse failure than an ugly
 * sentence — the same rule that keeps `restic did not say` from being rendered as success.
 */
export function plainError(raw: string): string {
  if (AHEAD_OF_US.test(raw)) {
    return "The other device has changes you don't have yet — get their changes first, then send.";
  }
  return raw;
}

/** Vaults whose last sync ended somewhere the user needs to look at. */
export function needsAttention(): { vault: string; state: VaultSync }[] {
  return Object.entries(state.byVault)
    .filter(([, s]) => s.phase === 'conflicts' || s.phase === 'failed')
    .map(([vault, state]) => ({ vault, state }));
}

/** Forget a terminal state once the user has seen it (or once a later sync supersedes it). */
export function clearSync(vault: string): void {
  set(vault, { phase: 'idle', conflicts: [], error: undefined, merged: 0, kept: [] });
}

/**
 * Run the commit step, and report whether the sequence may continue.
 *
 * **An unfinished merge stops the sequence here**, so it reaches a terminal, nameable state
 * instead of failing two steps later in git's words. Staging a conflicted path would publish
 * `<<<<<<<` as a note's content, so `commit_all` never does — and `pull` and `push_squashed`
 * both refuse outright while a merge is in flight.
 *
 * **Keyed on `conflicts`, not on `!committed`** (corrected 2026-09-07). It used to require both,
 * which was sound while a mid-merge commit committed *nothing*: `!committed` was then a reliable
 * proxy for "mid-merge". `commit_all` now commits every path except the conflicted ones
 * (`decisions.md`, *a conflict blocks its own notes and nothing else*), so the ordinary outcome
 * is `committed: true` with notes still stuck — and the old test would sail straight past into a
 * push that cannot work. The conflict list is the thing that actually says a merge is unfinished.
 */
async function commitStep(vault: string, message: string, ops: SyncOps): Promise<SyncPhase | null> {
  let result: CommitResult;
  try {
    result = await ops.commit(message, vault);
  } catch (e) {
    // A commit that fails is not a sync problem to retry — it is the vault refusing,
    // and pushing past it would be worse.
    set(vault, { phase: 'failed', error: String(e) });
    return 'failed';
  }
  set(vault, { committed: !!result?.committed });
  if (result?.conflicts?.length) {
    set(vault, { phase: 'conflicts', conflicts: result.conflicts });
    return 'conflicts';
  }
  return null; // carry on
}

/**
 * Commit and publish one vault, healing a moved remote once.
 *
 * `onChanged` is called whenever a pull actually brought work in, so the caller can re-run
 * its queries — a merge rewrites notes behind the index's back, and the views are served
 * from SQLite.
 *
 * Never throws: the whole reason this exists is that the failure has somewhere to go.
 * Returns the terminal phase so a caller can chain on it.
 */
export async function syncVault(
  vault: string,
  message: string,
  onChanged?: () => void | Promise<void>,
  ops: SyncOps = realOps,
): Promise<SyncPhase> {
  set(vault, {
    phase: 'committing',
    conflicts: [],
    error: undefined,
    merged: 0,
    kept: [],
    committed: false,
  });
  const stopped = await commitStep(vault, message, ops);
  if (stopped) return stopped;

  set(vault, { phase: 'pushing' });
  try {
    await ops.push(message, vault);
    set(vault, { phase: 'synced' });
    return 'synced';
  } catch (pushErr) {
    // The rejection this whole function exists for — or an ordinary one (offline, no
    // remote, bad credentials). We cannot tell them apart from the message, so ask git:
    // pull, and let what comes back say which it was.
    set(vault, { phase: 'pulling' });
    let pulled;
    try {
      pulled = await ops.pull(vault);
    } catch (pullErr) {
      // Both directions failed — which is exactly what a vault with no remote looks like, since
      // there is nothing to push to and nothing to pull from. Ask before calling it a failure.
      if (!(await ops.hasRemote(vault).catch(() => true))) {
        set(vault, { phase: 'local' });
        return 'local';
      }
      // Report the *push* error: it is the thing the user asked for, and the pull was our idea.
      set(vault, { phase: 'failed', error: String(pushErr) });
      return 'failed';
    }

    // Recorded before the conflict branch returns, so a pull that both kept a note *and* left a
    // marker conflict still reports the keep. They are independent outcomes of one merge.
    if (pulled.kept.length > 0) set(vault, { kept: pulled.kept });

    if (pulled.conflicts.length > 0) {
      // A result, not an error — the notes still open, with markers in the body. But the
      // sequence stops here: pushing now would publish the markers.
      await onChanged?.();
      set(vault, { phase: 'conflicts', conflicts: pulled.conflicts });
      return 'conflicts';
    }

    if (pulled.merged > 0) await onChanged?.();

    if (pulled.merged === 0) {
      // Nothing came down, so the remote had not moved and the push failed for its own
      // reasons. Retrying would fail identically.
      if (!(await ops.hasRemote(vault).catch(() => true))) {
        set(vault, { phase: 'local' });
        return 'local';
      }
      set(vault, { phase: 'failed', error: String(pushErr) });
      return 'failed';
    }

    // Their work is in and merged cleanly. One more push, and only one.
    set(vault, { phase: 'pushing', merged: pulled.merged });
    try {
      await ops.push(message, vault);
      set(vault, { phase: 'synced' });
      return 'synced';
    } catch (e) {
      set(vault, { phase: 'failed', error: String(e) });
      return 'failed';
    }
  }
}

/**
 * Bring one vault's remote work down, without publishing anything.
 *
 * The "someone pushed" nudge, and the safe half of the loop: it can surface conflicts but
 * can never send. Kept separate from [`syncVault`] because pulling is a thing the user may
 * want on a timer, and pushing is not.
 */
export async function pullVault(
  vault: string,
  onChanged?: () => void | Promise<void>,
  ops: SyncOps = realOps,
): Promise<SyncPhase> {
  // **Commit first.** git will not merge over uncommitted edits, so pulling straight into
  // a dirty tree either refuses or strands the merge — and the 5 s auto-commit debounce
  // means the tree is dirty exactly when the user has just been typing, which is exactly
  // when they reach for "get changes". This policy already existed in the backup panel and
  // not in the top-bar nudge; two spellings of one rule is how they drift.
  set(vault, {
    phase: 'committing',
    conflicts: [],
    error: undefined,
    merged: 0,
    kept: [],
    committed: false,
  });
  const stopped = await commitStep(vault, `auto: ${new Date().toISOString()}`, ops);
  if (stopped) return stopped;

  set(vault, { phase: 'pulling' });
  try {
    const pulled = await ops.pull(vault);
    if (pulled.merged > 0) await onChanged?.();
    if (pulled.kept.length > 0) set(vault, { kept: pulled.kept });
    if (pulled.conflicts.length > 0) {
      await onChanged?.();
      set(vault, { phase: 'conflicts', conflicts: pulled.conflicts });
      return 'conflicts';
    }
    set(vault, { phase: 'synced', merged: pulled.merged });
    return 'synced';
  } catch (e) {
    set(vault, { phase: 'failed', error: String(e) });
    return 'failed';
  }
}
