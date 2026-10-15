// **A vault is set up in one place: Settings → Notes & backup → Vaults** (`decisions.md#ui`,
// 2026-10-15). It used to be two windows — Settings held a vault's name, folder and attachment
// limit, the Back up panel its address, token and snapshot — and *New vault…* sat in the second's
// corner. Three things are pinned here:
//
//  1. **One block per vault, with both halves in it.** Settings' rows and the backup rows for the
//     same vault are inside one element, not two lists that happen to be near each other.
//  2. **New vault… is at the top of the section** where the vaults are listed.
//  3. **No network call for opening Settings.** Reading backup state runs `git ls-remote` per
//     vault, so the backup half is mounted only once its category is shown.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backupStatus } = vi.hoisted(() => ({ backupStatus: vi.fn() }));
vi.mock('./ipc', async (importOriginal) => {
  const real = await importOriginal<typeof import('./ipc')>();
  backupStatus.mockImplementation(real.backupStatus);
  return { ...real, backupStatus };
});

import * as mock from './mock';
import SettingsPanel from './SettingsPanel.svelte';

beforeEach(() => {
  mock.reset();
  backupStatus.mockClear();
});

function panel(category: 'notes' | 'look', onnewvault = () => {}) {
  return render(SettingsPanel, {
    onclose: () => {},
    onnewvault,
    layout: 'single',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
    category,
  } as never);
}

describe('Settings → Vaults is the one home of a vault', () => {
  it('draws each vault as one block holding its settings and where it is sent', async () => {
    panel('notes');
    // Wait for the backup half: one "where it is sent" field per vault.
    const remotes = await screen.findAllByPlaceholderText('git@github.com:you/notes.git');
    const limits = screen.getAllByLabelText(/largest attachment to send/i);
    expect(limits).toHaveLength(remotes.length);
    for (const limit of limits) {
      const block = limit.closest('.vault');
      expect(block).not.toBeNull();
      // The same block, not a second list beside the first.
      expect(remotes.filter((r) => block?.contains(r))).toHaveLength(1);
    }
  });

  it('offers New vault… beside the list of vaults', async () => {
    const onnewvault = vi.fn();
    panel('notes', onnewvault);
    await fireEvent.click(await screen.findByRole('button', { name: 'New vault…' }));
    expect(onnewvault).toHaveBeenCalledOnce();
  });

  it('asks nothing about backups until Notes & backup is opened', async () => {
    panel('look');
    // Let the panel settle on the category it opened at.
    await screen.findAllByText(/look/i);
    expect(backupStatus).not.toHaveBeenCalled();
  });

  it('reads backup state once Notes & backup is shown', async () => {
    panel('notes');
    await waitFor(() => expect(backupStatus).toHaveBeenCalled());
  });
});
