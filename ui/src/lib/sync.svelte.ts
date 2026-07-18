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

import { commit, push, pull } from './ipc';
import type { PullResult } from './types';

/**
 * The three operations the sequence is made of, injectable so every branch of it can be
 * driven by a test. The interesting states — a remote that moved, a merge that conflicts,
 * a push that fails for its own reasons — are exactly the ones that are hard to stage
 * against a real git remote and easy to get wrong.
 */
export interface SyncOps {
  commit(message: string, vault: string): Promise<unknown>;
  push(message: string, vault: string): Promise<unknown>;
  pull(vault: string): Promise<PullResult>;
}

const realOps: SyncOps = { commit, push, pull };

/** Where one vault stands. `idle` means nothing has been attempted since the last success. */
export type SyncPhase =
  | 'idle'
  | 'committing'
  | 'pushing'
  | 'pulling'
  | 'synced'
  | 'conflicts'
  | 'failed';

export interface VaultSync {
  phase: SyncPhase;
  /** Notes with conflict markers, waiting for a human. Only set in the `conflicts` phase. */
  conflicts: string[];
  /** Why it stopped, for the `failed` phase — shown to the user, never swallowed. */
  error?: string;
  /** Commits pulled in on the way, so the UI can say "3 changes arrived". */
  merged: number;
}

const state = $state<{ byVault: Record<string, VaultSync> }>({ byVault: {} });

function set(vault: string, patch: Partial<VaultSync>): void {
  const prev = state.byVault[vault] ?? { phase: 'idle' as SyncPhase, conflicts: [], merged: 0 };
  state.byVault[vault] = { ...prev, ...patch };
}

/** This vault's sync state. Reactive. */
export function syncFor(vault: string): VaultSync {
  return state.byVault[vault] ?? { phase: 'idle', conflicts: [], merged: 0 };
}

/** Every vault currently mid-flight — what a global "syncing…" indicator reads. */
export function syncing(): string[] {
  return Object.entries(state.byVault)
    .filter(([, s]) => s.phase === 'committing' || s.phase === 'pushing' || s.phase === 'pulling')
    .map(([v]) => v);
}

/** Vaults whose last sync ended somewhere the user needs to look at. */
export function needsAttention(): { vault: string; state: VaultSync }[] {
  return Object.entries(state.byVault)
    .filter(([, s]) => s.phase === 'conflicts' || s.phase === 'failed')
    .map(([vault, state]) => ({ vault, state }));
}

/** Forget a terminal state once the user has seen it (or once a later sync supersedes it). */
export function clearSync(vault: string): void {
  set(vault, { phase: 'idle', conflicts: [], error: undefined, merged: 0 });
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
  set(vault, { phase: 'committing', conflicts: [], error: undefined, merged: 0 });
  try {
    await ops.commit(message, vault);
  } catch (e) {
    // A commit that fails is not a sync problem to retry — it is the vault refusing
    // (mid-merge, most likely), and pushing past it would be worse.
    set(vault, { phase: 'failed', error: String(e) });
    return 'failed';
  }

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
      // Both directions failed. Report the *push* error: it is the thing the user asked
      // for, and the pull was our idea.
      set(vault, { phase: 'failed', error: String(pushErr) });
      return 'failed';
    }

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
  set(vault, { phase: 'committing', conflicts: [], error: undefined, merged: 0 });
  try {
    await ops.commit(`auto: ${new Date().toISOString()}`, vault);
  } catch (e) {
    set(vault, { phase: 'failed', error: String(e) });
    return 'failed';
  }

  set(vault, { phase: 'pulling' });
  try {
    const pulled = await ops.pull(vault);
    if (pulled.merged > 0) await onChanged?.();
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
