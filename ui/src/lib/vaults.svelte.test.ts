// **The vault label is display; the vault name is identity.**
//
// Two devices that cloned one repository can call that audience two different things — the owner's
// laptop said `vault` where the phone said `notes` — so the same audience looked like two vaults
// depending on the screen. The label follows the remote, which both devices agree about; the name
// stays the write-routing key and the key behind every persisted view preference, so it must not
// move. These tests pin that separation, which is the whole reason the registry exists.

import { beforeEach, describe, expect, it } from 'vitest';
import { labelFor, setVaults, vaultList } from './vaults.svelte';
import type { VaultInfo } from './types';

const v = (name: string, label: string | null): VaultInfo => ({
  name,
  path: `/vaults/${name}`,
  default: false,
  git_assets_max: null,
  supervision: { collect: true, publish: false },
  restic_repo: null,
  label,
  identity: null,
});

beforeEach(() => setVaults([]));

describe('labelFor', () => {
  it('shows the repository name when the vault has one', () => {
    setVaults([v('vault', 'formicarium-vault')]);
    expect(labelFor('vault')).toBe('formicarium-vault');
  });

  it('falls back to the vault name when there is no label', () => {
    setVaults([v('scratch', null)]);
    expect(labelFor('scratch')).toBe('scratch');
  });

  it('falls back for a vault it has never heard of, rather than rendering nothing', () => {
    // A note can carry a vault the list has not caught up with (a fresh clone, mid-refresh). Showing
    // the raw name is right; showing an empty badge would read as "no audience", which is a lie about
    // who can see the note.
    expect(labelFor('surprise')).toBe('surprise');
  });

  it('is empty only for no vault at all', () => {
    expect(labelFor(null)).toBe('');
    expect(labelFor(undefined)).toBe('');
    expect(labelFor('')).toBe('');
  });

  it('keys on the name, so the identity is what callers keep passing', () => {
    setVaults([v('notes', 'personal-notes'), v('vault', 'formicarium-vault')]);
    // The label of one vault must never answer for another — this is the map that would make the
    // vault filter hide the wrong audience if it keyed on anything else.
    expect(labelFor('notes')).toBe('personal-notes');
    expect(labelFor('vault')).toBe('formicarium-vault');
  });

  it('forgets labels that are no longer in the list', () => {
    setVaults([v('gone', 'old-repo')]);
    expect(labelFor('gone')).toBe('old-repo');
    // After a vault is forgotten, `list_vaults` no longer mentions it; a stale label would keep
    // renaming a vault that has been removed and re-added under a different remote.
    setVaults([]);
    expect(labelFor('gone')).toBe('gone');
  });
});

// ── One vault, one name, on every surface ──
//
// Caught on the phone 2026-07-31: the vault filter chips showed `formicarium-vault` (the repository)
// while the create-destination selector beside them still showed the raw local name `notes`, so one
// vault appeared under two different names on a single screen — the exact confusion labels exist to
// remove. The fix is that every place a vault name is *displayed* goes through `labelFor`, while every
// place it is *used* keeps the name. A test cannot enumerate future call sites, so what is pinned here
// is the contract they must follow.
describe('the label contract', () => {
  it('maps a name to its label, and leaves the name usable as the key', () => {
    setVaults([v('notes', 'formicarium-vault'), v('vault', null)]);
    // What a surface shows.
    expect(labelFor('notes')).toBe('formicarium-vault');
    expect(labelFor('vault')).toBe('vault');
    // What a surface sends. The identity is unchanged by labelling — `hiddenVaults`,
    // `fm-board-order` and every vault-taking command still key on these.
    const names = ['notes', 'vault'];
    expect(names.map(labelFor)).toEqual(['formicarium-vault', 'vault']);
    expect(names).toEqual(['notes', 'vault']);
  });
});

// **The list is the thing, and the labels are derived from it.**
//
// This module was a label registry fed *beside* the vault list rather than from it: `App` held its
// own `vaults` and three places assigned it, only one of which also fed the registry. So a vault
// created or removed kept its old name on screen — and after a removal the vault filter and the
// "Create in" picker went on describing the world before it, until the app was relaunched. That is
// what made one mistaken removal read as lost notes (2026-09-08).
describe('the vault list itself', () => {
  it('has no way to hold a label that the list does not', () => {
    setVaults([v('notes', 'formicarium-vault')]);
    expect(labelFor('notes')).toBe('formicarium-vault');

    // A second list is the *only* way to change a label, so there is nowhere for a stale one to
    // live. Before this, the list and the labels were fed separately and could disagree.
    setVaults([v('notes', 'renamed-repo')]);
    expect(labelFor('notes')).toBe('renamed-repo');

    setVaults([]);
    expect(labelFor('notes')).toBe('notes');
  });

  it('says "not loaded yet" and "no vaults" differently', () => {
    // The boot gate turns on exactly this: a `list_vaults` that failed must never be mistaken for a
    // machine with no vaults, which is the first-run screen. `App.boot.test.ts` pins the screens;
    // this pins the distinction they read.
    setVaults(null);
    expect(vaultList()).toBeNull();

    setVaults([]);
    expect(vaultList()).toEqual([]);
  });
});
