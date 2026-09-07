// Turning a `check_path` answer into sentences a person can act on.
//
// This is `destination.ts` for vault creation, and it exists for the same reason: a local
// path is a legitimate backup destination that must never be *called* off-site, and a
// directory with notes in it is a legitimate vault location that must never be adopted
// *silently*. Both files are the place where "never overstate what happened" is enforced,
// which is why both are pure and tested.
//
// It does **not** decide `ok`. The server does. Duplicating the policy here is exactly how
// `restic_ready` came to enable a checkbox that then failed — the browser said yes, the
// backend said no, and the user found out afterwards.

import type { PathCheck } from './types';

export interface Described {
  /** Why the button is off. Empty when the server says `ok`. */
  blocking: string[];
  /** True things the user must know before they press it. Never stops them. */
  warnings: string[];
}

export function describe(c: PathCheck | null): Described {
  if (!c) return { blocking: [], warnings: [] };

  const blocking: string[] = [];
  const warnings: string[] = [];

  // Ordered by how fundamental the problem is, not by how it reads: someone whose config
  // is unwritable cannot fix that by renaming the vault, so it is said first.
  if (!c.config_writable) {
    blocking.push(
      "The vault list on this machine isn't something we can safely write. Fix it, then try again — nothing here will touch it until you do.",
    );
  }
  if (!c.name_ok) blocking.push('Give the vault a name.');
  if (c.name_taken) blocking.push('You already have a vault with this name.');
  if (c.path_taken) blocking.push('This folder is already a vault.');
  if (c.overlaps) {
    blocking.push(
      `This folder is inside “${c.overlaps}” (or contains it). One folder cannot be in two audiences at once.`,
    );
  }
  if (c.not_a_directory) blocking.push('That is a file, not a folder.');
  if (!c.writable && !c.not_a_directory) blocking.push(`We cannot write to ${c.path}.`);

  // The one that matters most, and the reason this module is not just `ok ? [] : [...]`:
  // adoption is free — `FileStore` reindexes whatever is already there, with no import
  // step and no migration — but free is not the same as expected.
  if (c.notes > 0) {
    warnings.push(
      `This folder already has ${c.notes} ${c.notes === 1 ? 'note' : 'notes'} in it. They'll be adopted into the vault, exactly as they are.`,
    );
  }
  if (c.parent_missing) warnings.push("The folders don't exist yet — we'll create them.");
  else if (!c.exists) warnings.push("The folder doesn't exist yet — we'll create it.");
  if (c.git_repo) {
    warnings.push("This is already a git repository. We'll leave its history alone.");
  }

  return { blocking, warnings };
}

/** What this machine can do for a vault's notes, said once, at the moment it matters —
 *  never discovered on the day you needed the history. `git` is the `ping` heartbeat's
 *  capability flag, not a guess. */
export function historyNote(git: boolean): string {
  return git
    ? 'Your notes will be versioned from the first one you write — nothing to set up.'
    : "git isn't installed, so nothing here will be versioned. Your notes are still files on disk and are safe; you just won't have history until you install it.";
}
