// **Removing a vault from the list, from the only place the owner can reach it.**
//
// Three commands created a vault and none removed one, so the empty default the phone auto-creates
// on first launch was unremovable from inside the app — and the app is the only way in (owner,
// 2026-07-31: "creates only confusion"). What is pinned here is the two-step and the wording: the
// notes stay on disk, and the message has to say so, or "removed from the list" reads as "erased".

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backupStatus, forgetVault } = vi.hoisted(() => ({
  backupStatus: vi.fn(),
  forgetVault: vi.fn(),
}));
// Spread the real module and override only these two: the panel pulls in `sync.svelte.ts`, which
// imports `push` — a hand-listed factory silently drops whatever the import graph also needs, and
// the failure is "no tests collected" rather than anything about the missing name.
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backupStatus,
  forgetVault,
}));

import BackupPanel from './BackupPanel.svelte';

const vault = (name: string, remote: string | null) => ({
  name,
  remote,
  unpushed: null,
  identity: null,
  remote_moved: null,
  conflicts: [],
  restic_repo: null,
  restic_ready: false,
  git_assets_max: null,
});

beforeEach(() => {
  vi.clearAllMocks();
  backupStatus.mockResolvedValue({
    vaults: [vault('vault', 'https://example.org/v.git'), vault('notes', null)],
    git: true,
    restic: false,
  });
});

describe('removing a vault', () => {
  it('asks first, then says the files stayed', async () => {
    forgetVault.mockResolvedValue({
      forgotten: 'notes',
      path: '/data/vaults/notes',
      notes: 0,
      remote: null,
      vaults: [],
    });
    render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });

    // One click arms it; it must not remove on the first click, in a list of several vaults.
    const [first] = await screen.findAllByRole('button', { name: /Remove this vault from the list/i });
    await fireEvent.click(first);
    expect(forgetVault).not.toHaveBeenCalled();
    expect(screen.getByText(/Its files stay where they are/i)).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: /Yes, remove it/i }));
    await vi.waitFor(() => expect(forgetVault).toHaveBeenCalledOnce());
    // An empty vault says so plainly — this is the phone's dummy-vault case.
    await vi.waitFor(() => expect(document.body.textContent).toMatch(/it was empty\. Nothing was deleted/i));
  });

  it('names how many notes were left behind, and where', async () => {
    forgetVault.mockResolvedValue({
      forgotten: 'vault',
      path: '/home/you/notes',
      notes: 209,
      remote: 'https://example.org/v.git',
      vaults: [],
    });
    render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });
    const [first] = await screen.findAllByRole('button', { name: /Remove this vault from the list/i });
    await fireEvent.click(first);
    await fireEvent.click(screen.getByRole('button', { name: /Yes, remove it/i }));

    // The count and the path are the whole point: nobody should have to wonder whether removing a
    // vault from a list deleted 209 notes.
    await vi.waitFor(() => expect(document.body.textContent).toMatch(/209 notes are still on disk/i));
    expect(document.body.textContent).toMatch(/\/home\/you\/notes/);
  });

  it('can be cancelled', async () => {
    render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });
    const [first] = await screen.findAllByRole('button', { name: /Remove this vault from the list/i });
    await fireEvent.click(first);
    await fireEvent.click(screen.getByRole('button', { name: /Cancel/i }));
    expect(forgetVault).not.toHaveBeenCalled();
  });
});
