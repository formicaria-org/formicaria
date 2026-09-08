// **"Open backup options for detail" has to lead to a detail.**
//
// The toolbar's backup summary names a vault that needs attention and sends you here. But the
// panel's step list is only ever filled by the panel's *own* buttons — `run()` and `bringDown()`
// both start with `steps = []` — so a backup started from the toolbar left this screen with
// nothing on it, while the reason sat unread in the sync store. Reported from the phone on
// 2026-09-08: a vault named as needing attention, three commits unpushed, and no way to find out
// why from the screen the message pointed at.
//
// A conflict was always visible here (it comes from `backup_status`, which the panel reloads on
// open). A *push failure* was not, because its text lives only in `syncFor(vault).error`.
import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as mock from './mock';
import BackupPanel from './BackupPanel.svelte';
import { syncVault } from './sync.svelte';

beforeEach(() => {
  mock.reset();
  vi.clearAllMocks();
});

const panel = () => render(BackupPanel, { onclose: () => {} } as never);

describe('BackupPanel — why the notes did not go', () => {
  it('shows the reason a push failed, without needing you to press its own button first', async () => {
    // Exactly what a toolbar backup leaves behind: a `failed` phase with the push error, recorded
    // in the sync store by `syncVault` and never rendered anywhere the user could reach.
    await syncVault('personal', 'backup', undefined, {
      hasRemote: async () => true,
      commit: async () => ({ committed: true, conflicts: [] }),
      push: () => Promise.reject(new Error('403 forbidden — check the token')),
      pull: () => Promise.reject(new Error('403 forbidden — check the token')),
    });

    panel();
    expect(await screen.findByText(/403 forbidden — check the token/)).toBeTruthy();
  });

  it('says nothing of the kind for a vault that is fine', async () => {
    // Anchored on something that must render, so this cannot pass before the panel has loaded.
    await syncVault('personal', 'backup', undefined, {
      hasRemote: async () => true,
      commit: async () => ({ committed: true, conflicts: [] }),
      push: async () => {},
      pull: async () => ({ merged: 0, conflicts: [], kept: [] }),
    });

    panel();
    await screen.findByRole('heading', { name: /back ?up/i });
    expect(screen.queryByText(/Notes did not go/)).toBeNull();
  });
});
