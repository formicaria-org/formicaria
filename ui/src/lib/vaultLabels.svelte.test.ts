// **The vault label is display; the vault name is identity.**
//
// Two devices that cloned one repository can call that audience two different things — the owner's
// laptop said `vault` where the phone said `notes` — so the same audience looked like two vaults
// depending on the screen. The label follows the remote, which both devices agree about; the name
// stays the write-routing key and the key behind every persisted view preference, so it must not
// move. These tests pin that separation, which is the whole reason the registry exists.

import { beforeEach, describe, expect, it } from 'vitest';
import { labelFor, setVaultLabels } from './vaultLabels.svelte';
import type { VaultInfo } from './types';

const v = (name: string, label: string | null): VaultInfo => ({
  name,
  path: `/vaults/${name}`,
  default: false,
  git_assets_max: null,
  label,
});

beforeEach(() => setVaultLabels([]));

describe('labelFor', () => {
  it('shows the repository name when the vault has one', () => {
    setVaultLabels([v('vault', 'formicarium-vault')]);
    expect(labelFor('vault')).toBe('formicarium-vault');
  });

  it('falls back to the vault name when there is no label', () => {
    setVaultLabels([v('scratch', null)]);
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
    setVaultLabels([v('notes', 'personal-notes'), v('vault', 'formicarium-vault')]);
    // The label of one vault must never answer for another — this is the map that would make the
    // vault filter hide the wrong audience if it keyed on anything else.
    expect(labelFor('notes')).toBe('personal-notes');
    expect(labelFor('vault')).toBe('formicarium-vault');
  });

  it('forgets labels that are no longer in the list', () => {
    setVaultLabels([v('gone', 'old-repo')]);
    expect(labelFor('gone')).toBe('old-repo');
    // After a vault is forgotten, `list_vaults` no longer mentions it; a stale label would keep
    // renaming a vault that has been removed and re-added under a different remote.
    setVaultLabels([]);
    expect(labelFor('gone')).toBe('gone');
  });
});
