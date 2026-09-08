// **A private HTTPS remote needs a token, and until now the panel had nowhere to put one.**
//
// The field existed only in the *clone* form. Someone who made a vault here and later pointed it
// at a private repo never went through that form — so Back up failed at the push with an
// authentication error, and the one screen that talks about remotes offered nothing to do about
// it. `outstanding.md` §2.6b named it; this pins the fix.
//
// Written as much about **when the field is absent** as when it is present: a token box beside an
// `ssh://` remote, or beside a machine whose helper already holds the credential, is an invitation
// to solve a problem you do not have with a credential that will never be read.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backupStatus, gitAuth, setGitCredential } = vi.hoisted(() => ({
  backupStatus: vi.fn(),
  gitAuth: vi.fn(),
  setGitCredential: vi.fn(),
}));
// Spread the real module and override only these: the panel pulls in `sync.svelte.ts`, and a
// hand-listed factory silently drops whatever else the import graph needs.
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backupStatus,
  gitAuth,
  setGitCredential,
}));

import BackupPanel from './BackupPanel.svelte';
import { syncVault } from './sync.svelte';

const vault = (name: string, remote: string | null) => ({
  name,
  remote,
  unpushed: null,
  identity: { name: 'Ada', email: 'ada@example.org' },
  remote_moved: null,
  conflicts: [],
  restic_repo: null,
  restic_ready: false,
  git_assets_max: null,
  supervision: { collect: true, publish: false },
});

const helper = { configured: 'store', plaintext: true, better: 'libsecret' };

beforeEach(() => {
  vi.clearAllMocks();
  gitAuth.mockResolvedValue({ storage: 'system', have_credential: false, helper });
});

const show = (vaults: ReturnType<typeof vault>[]) => {
  backupStatus.mockResolvedValue({ vaults, git: true, restic: false });
  render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });
};

describe('a token for a private HTTPS remote', () => {
  it('is asked for, and stored against that vault’s own remote', async () => {
    setGitCredential.mockResolvedValue({ storage: 'system', have_credential: true, helper });
    show([vault('notes', 'https://github.com/you/notes.git')]);

    const field = await screen.findByPlaceholderText(/github_pat_/i);
    await fireEvent.input(field, { target: { value: 'github_pat_secret' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save token/i }));

    expect(setGitCredential).toHaveBeenCalledWith(
      'https://github.com/you/notes.git',
      'github_pat_secret',
    );
    expect(await screen.findByText(/Saved\. Back up will use it/i)).toBeTruthy();
    // Write-only: a bearer credential has no business outliving the request that carried it, so
    // the box itself goes once the machine has what it needs — there is nothing left to read back.
    expect(screen.queryByPlaceholderText(/github_pat_/i)).toBeNull();
  });

  it('says where the token is going when the helper keeps it in plain text', async () => {
    show([vault('notes', 'https://github.com/you/notes.git')]);
    expect(await screen.findByText(/plain text/i)).toBeTruthy();
  });

  it('is not offered for an ssh remote, which uses the key in your agent', async () => {
    show([vault('notes', 'git@github.com:you/notes.git')]);
    await screen.findByText(/where your notes are copied to/i);
    expect(screen.queryByPlaceholderText(/github_pat_/i)).toBeNull();
    expect(gitAuth).not.toHaveBeenCalled();
  });

  it('is not offered once this machine already has the credential', async () => {
    gitAuth.mockResolvedValue({ storage: 'system', have_credential: true, helper });
    show([vault('notes', 'https://github.com/you/notes.git')]);
    await screen.findByText(/where your notes are copied to/i);
    expect(screen.queryByPlaceholderText(/github_pat_/i)).toBeNull();
  });
});

// **A stored credential the remote refuses is not a credential.**
//
// The gate above asks only whether one is *present*, so a token that has expired, been revoked, or
// was minted without the `repo` scope left the field hidden while every push failed on it: the
// panel said "authentication failed — check the token for this remote" and offered nothing that
// could change it. Reported from the owner's phone on 2026-09-08, with three commits stuck behind
// it and no way forward from the only screen they use — the same dead end this file's header
// describes for a *missing* credential, fixed then for that case only.
//
// A different vault name from the tests above, deliberately: `sync.svelte.ts`'s store is module
// state and outlives a test, so marking `notes` as failed here would reach back into
// "is not offered once this machine already has the credential" if the order ever changed.
const failWith = (message: string) =>
  syncVault('lab', 'backup', undefined, {
    hasRemote: async () => true,
    commit: async () => ({ committed: true, conflicts: [] }),
    push: () => Promise.reject(new Error(message)),
    pull: () => Promise.reject(new Error(message)),
  });

describe('a token the remote refuses', () => {
  it('is offered again when the credential it already has was rejected', async () => {
    gitAuth.mockResolvedValue({ storage: 'app', have_credential: true, helper: null });
    await failWith('io error: authentication failed — check the token for this remote');
    show([vault('lab', 'https://github.com/you/notes.git')]);

    // The reason, and — the whole point — something to do about it in the same block.
    expect(await screen.findByText(/authentication failed/i)).toBeTruthy();
    expect(await screen.findByPlaceholderText(/github_pat_/i)).toBeTruthy();
    expect(await screen.findByText(/was refused/i)).toBeTruthy();
  });

  it('stays hidden when the push failed for a reason a token cannot fix', async () => {
    gitAuth.mockResolvedValue({ storage: 'app', have_credential: true, helper: null });
    await failWith('could not resolve host github.com');
    show([vault('lab', 'https://github.com/you/notes.git')]);

    // Anchored on the reason, so this cannot pass before the panel has rendered anything.
    expect(await screen.findByText(/could not resolve host/i)).toBeTruthy();
    expect(screen.queryByPlaceholderText(/github_pat_/i)).toBeNull();
  });
});
