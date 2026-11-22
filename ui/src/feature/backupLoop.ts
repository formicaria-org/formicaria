// **Backing up by itself, on the computer** (found 2026-11-22 by the shell's parity list). The loop
// that saves, gets the other devices' changes and sends (`autoBackup` in `App.svelte`, the policy in
// `lib/autoBackup.ts`) ran only in the older frame. Since the computer moved to formicaria's
// browser pages (`decisions.md` 2026-11-17, *one frame*), nothing on the computer sent notes to the
// person's remote by itself, while Settings said it did. The same policy and the same `syncVault`,
// for the feature pages.
//
// **One tab runs it.** Every open formicaria page would otherwise back up at once. The browser's lock
// (`navigator.locks`) gives the loop to exactly one tab, and to another the moment that one closes.
// With no formicaria page open the server has stopped too, so there is nothing to back up.
import { backupStatus, lastCommits } from '../lib/ipc';
import { syncFor, syncVault, syncingPhase } from '../lib/sync.svelte';
import {
  autoBackupOn,
  dueVaults,
  RETURN_AFTER_MS,
  runsByItself,
  TICK_MS,
  unsentVaults,
  type Attempt,
  type Occasion,
  type Standing,
  type Unsent,
} from '../lib/autoBackup';
import { setLevelHooks } from '../lib/keepLevel.svelte';

const NOT_MID_SENTENCE: Occasion[] = ['leaving', 'returning', 'editing'];
const LEFT_NOTE_SETTLE_MS = 1500;
const REMOTES_EVERY_MS = 5 * 60_000;

export interface BackupLoop {
  stop(): void;
  /** Run once now, as if for this occasion (tests, and the page's own events). */
  run(occasion?: Occasion): Promise<void>;
  /** The vaults whose changes are not leaving this device (for a warning). */
  unsent(): Unsent[];
}

export function startBackupLoop(opts: {
  /** Something changed on disk because of a pull: read again. */
  refresh?: () => void | Promise<void>;
  /** Called after each vault's backup, with what came back. */
  said?: (vault: string, phase: string) => void;
  /** Called when the list of unsent vaults may have changed. */
  onUnsent?: (u: Unsent[]) => void;
  /** Other work only one tab should do while it holds the loop (keeping the program's reminders
   *  current); returns how to stop it. */
  whileLeading?: () => (() => void) | void;
}): BackupLoop {
  const attempts: Record<string, Attempt> = {};
  let moved: string[] = [];
  let rows: Standing[] = [];
  let running = false;
  let again: Occasion | null = null;
  let stopped = false;
  let hiddenAt = 0;

  const typing = () => {
    const el = document.activeElement as HTMLElement | null;
    return (
      !!el &&
      (el.isContentEditable ||
        /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) ||
        !!el.closest?.('.cm-editor'))
    );
  };
  const tellUnsent = () =>
    opts.onUnsent?.(unsentVaults(rows, attempts, Date.now(), autoBackupOn()));

  async function checkRemotes() {
    try {
      const st = await backupStatus();
      moved = st.vaults.filter((v) => v.remote_moved).map((v) => v.name);
    } catch {
      /* offline, or no remote */
    }
  }

  async function run(occasion: Occasion = 'tick') {
    if (stopped || !runsByItself() || !autoBackupOn()) return;
    if (syncingPhase() !== null || navigator.onLine === false) return;
    if (!NOT_MID_SENTENCE.includes(occasion) && typing()) return;
    if (running) {
      again = occasion;
      return;
    }
    running = true;
    try {
      rows = await lastCommits().catch(() => [] as Standing[]);
      const due = dueVaults(rows, moved, attempts, Date.now(), occasion);
      const stamp = `${new Date().toISOString().slice(0, 16)}Z`;
      for (const v of due) {
        const phase = await syncVault(v, `backup: ${stamp}`, () => opts.refresh?.());
        attempts[v] = { at: Date.now(), ok: phase === 'synced', phase, error: syncFor(v).error };
        if (phase !== 'failed') moved = moved.filter((m) => m !== v);
        opts.said?.(v, phase);
      }
      if (due.length) rows = await lastCommits().catch(() => rows);
      tellUnsent();
    } finally {
      running = false;
      const next = again;
      again = null;
      if (next && !stopped) void run(next);
    }
  }

  const tick = setInterval(() => void run('tick'), TICK_MS);
  const remotes = setInterval(() => void checkRemotes(), REMOTES_EVERY_MS);
  const onHide = () => void run('leaving');
  const onVisible = () => {
    if (document.hidden) {
      hiddenAt = Date.now();
      return;
    }
    if (hiddenAt && Date.now() - hiddenAt >= RETURN_AFTER_MS) {
      void checkRemotes().then(() => run('returning'));
    }
    hiddenAt = 0;
  };
  const onOnline = () => void run('online');
  addEventListener('pagehide', onHide);
  document.addEventListener('visibilitychange', onVisible);
  addEventListener('online', onOnline);
  setLevelHooks({
    editing: (vault) => {
      if (moved.includes(vault)) void run('editing');
    },
    left: () => {
      setTimeout(() => void run('left-note'), LEFT_NOTE_SETTLE_MS);
    },
  });
  void checkRemotes().then(() => run('tick'));
  const alsoStop = opts.whileLeading?.();

  return {
    stop() {
      stopped = true;
      alsoStop?.();
      clearInterval(tick);
      clearInterval(remotes);
      removeEventListener('pagehide', onHide);
      document.removeEventListener('visibilitychange', onVisible);
      removeEventListener('online', onOnline);
    },
    run,
    unsent: () => unsentVaults(rows, attempts, Date.now(), autoBackupOn()),
  };
}

/** Run the loop in **one** tab of formicaria's browser: the one holding the browser's lock. The
 *  lock is let go when the tab closes, and the next tab waiting takes the loop. Without the lock
 *  API (an old browser, a test), the page runs it itself. */
export function backUpInOneTab(opts: Parameters<typeof startBackupLoop>[0]): () => void {
  let loop: BackupLoop | null = null;
  let release: (() => void) | null = null;
  const locks = (navigator as Navigator & { locks?: LockManager }).locks;
  if (!locks?.request) {
    loop = startBackupLoop(opts);
    return () => loop?.stop();
  }
  void locks.request(
    'fm-auto-backup',
    () =>
      new Promise<void>((done) => {
        loop = startBackupLoop(opts);
        release = done;
      }),
  );
  return () => {
    loop?.stop();
    release?.();
  };
}
