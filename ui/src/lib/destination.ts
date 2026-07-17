// Did the data actually leave this machine? A backup destination that turns out
// to be a second folder on the same disk survives a mistake but not a fire, and
// the panel must not call that "backed up" without saying so.
//
// One helper covers both tiers, because the trap runs both ways: a restic repo
// is a bare path when it is local (`/mnt/drive`) and scheme-prefixed when it is
// not (`s3:…`), while a git remote is scp-like or a URL when it is remote
// (`git@host:repo`) and a path or `file://` when it is not.

export type Reach = 'remote' | 'local' | 'unset';

/** restic's backend prefixes — everything else it takes is a local path. */
const RESTIC_REMOTE = ['s3', 'b2', 'sftp', 'rest', 'rclone', 'gs', 'azure', 'swift'];

/** Scp-like git syntax: `user@host:path`, which has no scheme to look for. */
const SCP_LIKE = /^[\w.-]+@[\w.-]+:/;

/**
 * Whether `dest` names somewhere off this machine.
 *
 * Unrecognized shapes count as `remote`: over-promising safety is the one error
 * that lets data die quietly, so anything that isn't demonstrably a local path
 * gets the benefit of the doubt only in the direction that keeps the warning on.
 */
export function reachOf(dest: string | null | undefined): Reach {
  const d = dest?.trim();
  if (!d) return 'unset';
  if (d.startsWith('file://')) return 'local';
  if (SCP_LIKE.test(d)) return 'remote';

  const scheme = d.match(/^([a-zA-Z][\w+.-]*):/)?.[1]?.toLowerCase();
  if (!scheme) return 'local'; // a bare path: /mnt/x, ./x, ~/x, x
  if (RESTIC_REMOTE.includes(scheme)) return 'remote';
  // A lone drive letter (C:\…) is a path, not a scheme.
  if (scheme.length === 1) return 'local';
  return 'remote'; // https://, ssh://, git://, …
}

/** A destination shown to a human: no credentials, no noise. */
export function shortDest(dest: string): string {
  return dest
    .trim()
    .replace(/^[a-zA-Z][\w+.-]*:\/\//, '') // scheme
    .replace(/^[^@/]+@/, '') // user[:password]@ — never show a secret back
    .replace(/\.git$/, '')
    .replace(/\/$/, '');
}
