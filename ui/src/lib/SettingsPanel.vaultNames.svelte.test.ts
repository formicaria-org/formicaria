// **A vault's name is not what it is called, and Settings was showing the name.**
//
// A vault's *name* is local to a machine: it is the write-routing key and the argument every
// vault-taking command carries. Two devices that cloned one repository therefore call the same
// audience different things — the owner's laptop says `vault` where the phone says `notes`. What
// both devices agree about is the repository behind the remote, so that is what every surface
// shows (`vaultLabels.svelte.ts`, and `list_vaults` derives it).
//
// The Backup panel obeyed that rule; this one never imported the helper. Reported 2026-09-08 from
// the phone: Settings listed `vault` and `notes` while the Backup panel, one tap away, headed the
// very same vaults `vault` and `formicarium-vault` — so the app looked like it had four vaults, or
// two with a bug. Neither screen was wrong on its own, which is what made it hard to see.
import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import * as mock from './mock';
import SettingsPanel from './SettingsPanel.svelte';
import { setVaultLabels } from './vaultLabels.svelte';
import type { VaultInfo } from './types';

beforeEach(() => mock.reset());

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
  } as never);
}

describe('SettingsPanel — what a vault is called', () => {
  it('lists a vault by its repository, not by the local folder name', async () => {
    // Exactly what `list_vaults` reports for a vault with a remote: the name stays the identity,
    // the label is the repository behind it.
    setVaultLabels([{ name: 'lab', label: 'lab-notes' } as VaultInfo]);
    panel();

    expect(await screen.findByText('lab-notes')).toBeTruthy();
    // And not under its local name, which is the half that made two screens disagree.
    expect(screen.queryByText('lab', { exact: true })).toBeNull();
  });

  it('falls back to the name for a vault with no remote to name it after', async () => {
    // `personal` carries no label in the fixture, and a vault with no remote never will — there is
    // no shared repository to agree about, so the local name is the honest answer.
    setVaultLabels([{ name: 'lab', label: 'lab-notes' } as VaultInfo]);
    panel();
    expect(await screen.findByText('personal')).toBeTruthy();
  });
});
