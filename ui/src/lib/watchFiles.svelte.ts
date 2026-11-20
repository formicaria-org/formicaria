// **A watch's files on the phone** (`decisions.md` 2026-11-20, *The phone takes watch files: the
// share menu, and one folder the person grants*), and **workouts out** (the Coach's item 6).
//
// Two ways in, one import: a `.fit` shared to formicaria (`share.svelte.ts` hands it here), and new
// `.fit` files in the one folder the person granted, which the shell lists on opening and on every
// return to the app (`__fmWatch`, written by `ci/android-inject-service.sh`'s `MainActivity`). Both
// go to `importFitFiles` — the same notes as the computer's import, and no attachment note.
//
// On a computer there is no bridge: the folder part is never seen, and a workout file is downloaded.
import { importFitFiles } from './ipc';
import type { FitReport, WorkoutFile } from './types';
import { toast } from './web/bridge.svelte';

type Bridge = {
  state(): string;
  choose(): void;
  forget(): void;
  look(): void;
  pending(): string;
  read(key: string): string;
  done(keys: string): void;
  send?(name: string, data: string): void;
  save?(name: string, data: string): void;
};

const bridge = (): Bridge | null => (globalThis as { __fmWatch?: Bridge }).__fmWatch ?? null;

/** Whether this is the phone app, which can be granted a folder and hand a file to another app. */
export const phoneWatch = () => bridge() !== null;

/** What the shell says about the granted folder. `folder` is empty when none is granted. */
export interface FolderState {
  folder: string;
  readable: boolean;
  looking: boolean;
  held: number;
  waiting: number;
  tooLarge: number;
}

/** The last import from the phone's watch files, for the Settings section to show. */
export interface WatchTaken {
  from: 'share' | 'folder';
  report: FitReport | null;
  error: string;
}

export const watchFiles = $state<{ folder: FolderState | null; last: WatchTaken | null }>({
  folder: null,
  last: null,
});

/** A shared file that is a watch's activity file (the shell has checked its signature). */
export const isWatchFile = (f: File) =>
  /\.fit$/i.test(f.name) || f.type === 'application/vnd.ant.fit';

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** One line for the toast: what an import added. */
export function said(r: FitReport): string {
  if (r.written) {
    return `Added ${plural(r.activities, 'activity', 'activities')}${
      r.sets ? ` and ${plural(r.sets, 'set', 'sets')}` : ''
    } from the watch to Fitness.`;
  }
  if (r.refused.length) return `${r.refused[0].file}: ${r.refused[0].reason}`;
  if (r.known) return 'Nothing new: that activity is in Fitness already.';
  return 'Nothing new was added from the watch.';
}

/** Files shared from another app: imported, and said in one line. */
export async function importShared(files: File[]): Promise<void> {
  try {
    const report = await importFitFiles(files);
    watchFiles.last = { from: 'share', report, error: '' };
    toast.now = { text: said(report) };
  } catch (e) {
    const error = e instanceof Error ? e.message : String(e);
    watchFiles.last = { from: 'share', report: null, error };
    toast.now = { text: `The watch file was not imported: ${error}` };
  }
}

function refreshState() {
  const b = bridge();
  if (!b) return;
  try {
    watchFiles.folder = JSON.parse(b.state()) as FolderState;
  } catch {
    watchFiles.folder = null;
  }
}

function bytesOf(b64: string): Uint8Array<ArrayBuffer> {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let k = 0; k < bin.length; k++) out[k] = bin.charCodeAt(k);
  return out;
}

/** A handful at a time: a first grant can find a season of files. */
const BATCH = 20;
let taking = false;

/** Import what the shell copied from the folder. A file stays held if the import fails, and is
 *  offered again; one the import read (added, known or refused) is never taken again. */
export async function takeFolder(): Promise<void> {
  const b = bridge();
  if (!b || taking) return;
  taking = true;
  try {
    refreshState();
    let pending: { key: string; name: string }[] = [];
    try {
      pending = JSON.parse(b.pending());
    } catch {
      return;
    }
    for (let at = 0; at < pending.length; at += BATCH) {
      const batch = pending.slice(at, at + BATCH);
      const files = batch.map((p) => new File([bytesOf(b.read(p.key))], p.name));
      try {
        const report = await importFitFiles(files);
        b.done(JSON.stringify(batch.map((p) => p.key)));
        watchFiles.last = { from: 'folder', report, error: '' };
        if (report.written) toast.now = { text: said(report) };
      } catch (e) {
        watchFiles.last = {
          from: 'folder',
          report: null,
          error: e instanceof Error ? e.message : String(e),
        };
        break;
      }
    }
  } finally {
    taking = false;
    refreshState();
  }
}

/** Listen for the shell's word that the folder was looked at; take what it found. */
export function startWatchFolder(): () => void {
  if (!bridge()) return () => {};
  const on = () => void takeFolder();
  window.addEventListener('fm-watch', on);
  void takeFolder();
  return () => window.removeEventListener('fm-watch', on);
}

/** Android's folder picker, for the folder the watch's app exports to. */
export const chooseFolder = () => bridge()?.choose();
/** Stop reading the folder (the grant is given back). */
export const forgetFolder = () => {
  bridge()?.forget();
  refreshState();
};
/** Look at the folder now. */
export const lookNow = () => {
  bridge()?.look();
  refreshState();
};
export const folderState = () => {
  refreshState();
  return watchFiles.folder;
};

// ── workouts out ─────────────────────────────────────────────────────────────────────────────────

/** Save a workout file: Android's *Save as* on the phone, a download elsewhere. */
export function saveWorkout(f: WorkoutFile): void {
  const b = bridge();
  if (b?.save) return b.save(f.name, f.data);
  const url = URL.createObjectURL(
    new Blob([bytesOf(f.data)], { type: 'application/octet-stream' }),
  );
  const a = document.createElement('a');
  a.href = url;
  a.download = f.name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Hand a workout file to the watch's app (Android's share sheet). Phone only. */
export function sendWorkout(f: WorkoutFile): boolean {
  const b = bridge();
  if (!b?.send) return false;
  b.send(f.name, f.data);
  return true;
}

/** Whether this device can hand a file to another app (the phone). */
export const canSendWorkout = () => typeof bridge()?.send === 'function';
