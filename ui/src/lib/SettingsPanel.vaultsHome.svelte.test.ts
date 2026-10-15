// **A vault is set up in one place: Settings → Vaults & backup** (`decisions.md#ui`,
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

function panel(category: 'vaults' | 'look', onnewvault = () => {}) {
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
    panel('vaults');
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
    panel('vaults', onnewvault);
    await fireEvent.click(await screen.findByRole('button', { name: 'New vault…' }));
    expect(onnewvault).toHaveBeenCalledOnce();
  });

  it('asks nothing about backups until Vaults & backup is opened', async () => {
    panel('look');
    // Let the panel settle on the category it opened at.
    await screen.findAllByText(/look/i);
    expect(backupStatus).not.toHaveBeenCalled();
  });

  it('reads backup state once Vaults & backup is shown', async () => {
    panel('vaults');
    await waitFor(() => expect(backupStatus).toHaveBeenCalled());
  });
});

/// **Moving a vault's folder from where its folder is shown** (`decisions.md#vault`, 2026-10-15).
describe('moving a vault to another folder', () => {
  it('checks the folder as it is typed, then moves it and says from where', async () => {
    panel('vaults');
    const [first] = await screen.findAllByRole('button', { name: 'Move to another folder…' });
    await fireEvent.click(first);
    const field = screen.getByLabelText(/new folder for/i);

    // A refusal is the server's sentence, and Move stays off.
    await fireEvent.input(field, { target: { value: 'relative/path' } });
    expect(await screen.findByText(/give the whole path/)).toBeTruthy();
    const move = screen.getByRole('button', { name: 'Move' }) as HTMLButtonElement;
    expect(move.disabled).toBe(true);

    await fireEvent.input(field, { target: { value: '/home/you/elsewhere' } });
    // What will move, and the promise that a failure leaves it where it is.
    expect(await screen.findByText(/will move to/)).toBeTruthy();
    expect(screen.getByText(/if anything fails, it stays where it is/)).toBeTruthy();
    await waitFor(() => expect(move.disabled).toBe(false));
    await fireEvent.click(move);

    expect(await screen.findByText(/Moved from/)).toBeTruthy();
    expect(await screen.findByText('/home/you/elsewhere')).toBeTruthy();
  });
});
