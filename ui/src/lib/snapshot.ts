/// **Taking a snapshot is an action, so it lives in the Back up menu** (`decisions.md#ui`,
/// 2026-10-15). It used to be a tick box and a button inside the backup panel, which is a section of
/// Settings now, and Settings holds settings, not actions. The rules of the report moved here
/// unchanged: every vault is named, one that could not be snapshotted says why, and what a snapshot
/// held is read from what restic answered, never a fixed phrase.

import { backup, backupStatus, listVaults } from './ipc';
import { reachOf, shortDest } from './destination';
import { humanSize } from './size';
import type { BackupRun } from './types';

export type SnapshotLine = { vault: string; text: string; ok: boolean };

/// The password is the app's now, and the environment variable is the override rather than the
/// mechanism — so the missing thing is named as what the person can set.
export const NO_PASSWORD = 'no backup password is set on this machine';

const left = (r: string) => (r === 'remote' ? 'off this machine' : 'still on this machine');

/// What a snapshot held, from what restic reported: the directories that went in, and its own
/// file and byte counts — with "not reported" kept apart from "held nothing".
export function held(r: BackupRun): string {
  const dirs: string[] = [];
  if (r.notes_dir) dirs.push(`${r.notes_dir}/`);
  if (r.blobs) dirs.push('blobs/');
  const what = dirs.join(' + ') || 'nothing this vault owns';
  if (!r.contents) return `${what}, contents not reported`;
  const c = r.contents;
  const files = c.files_new + c.files_changed + c.files_unmodified;
  const fresh = c.files_new + c.files_changed;
  // "Nothing changed" is the healthy steady state, said plainly rather than as a row of zeros.
  return fresh
    ? `${what}, ${files} file(s), ${fresh} new or changed, ${humanSize(c.bytes_added)} added`
    : `${what}, ${files} file(s), nothing changed since the last one`;
}

/// A snapshot of every vault that has a backup repo, one line per vault — including the ones that
/// could not be taken, and why. `label` turns a vault's name into what the person calls it.
export async function takeSnapshots(label: (name: string) => string): Promise<SnapshotLine[]> {
  // The repo is a local read (the vault list); whether it can run now is `backup_status`'s answer.
  const [list, status] = await Promise.all([listVaults(), backupStatus()]);
  const ready = new Map(status.vaults.map((v) => [v.name, v.restic_ready]));
  const lines: SnapshotLine[] = [];
  for (const v of list.filter((x) => !!x.restic_repo)) {
    const name = `“${label(v.name)}”`;
    if (!ready.get(v.name)) {
      const why = !status.restic ? 'restic is not installed on this machine' : NO_PASSWORD;
      lines.push({ vault: v.name, text: `Snapshot of ${name} NOT taken — ${why}.`, ok: false });
      continue;
    }
    try {
      const run = await backup(v.name);
      lines.push({
        vault: v.name,
        text: `Snapshot of ${name} — ${held(run)} → ${shortDest(v.restic_repo ?? '')} — ${left(reachOf(v.restic_repo))}.`,
        ok: true,
      });
    } catch (e) {
      lines.push({
        vault: v.name,
        text: `Snapshot of ${name} failed: ${e instanceof Error ? e.message : String(e)}`,
        ok: false,
      });
    }
  }
  return lines;
}
