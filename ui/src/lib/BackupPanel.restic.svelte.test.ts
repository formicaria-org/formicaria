// **Encrypted media backup, set up from the app instead of from a text editor.**
//
// Both halves of this tier used to live outside the product: the repository was a key you
// hand-edited into `vaults.json` — Settings said so in as many words, *"there is no UI for it"* —
// and the password was an environment variable whose documented answer was a launcher you had
// edited yourself. So the one tier that protects attachments was reachable only by someone willing
// to be their own system administrator (`outstanding.md` §2.6b).
//
// What is pinned here is as much about restraint as about the fields: no form for a tool this
// machine does not have, and no password asked for before there is a repository it would open.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backupStatus, gitAuth, setResticRepo, setResticPassword } = vi.hoisted(() => ({
  backupStatus: vi.fn(),
  gitAuth: vi.fn(),
  setResticRepo: vi.fn(),
  setResticPassword: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backupStatus,
  gitAuth,
  setResticRepo,
  setResticPassword,
}));

import BackupPanel from './BackupPanel.svelte';

const vault = (name: string, restic_repo: string | null) => ({
  name,
  remote: null,
  unpushed: null,
  identity: { name: 'Ada', email: 'ada@example.org' },
  remote_moved: null,
  conflicts: [],
  restic_repo,
  restic_ready: false,
  git_assets_max: null,
});

const show = (vaults: ReturnType<typeof vault>[], { restic = true, password = false } = {}) => {
  backupStatus.mockResolvedValue({ vaults, git: true, restic, restic_password_set: password });
  render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });
};

beforeEach(() => {
  vi.clearAllMocks();
  gitAuth.mockResolvedValue({ storage: 'system', have_credential: true, helper: null });
});

describe('setting up encrypted media backup', () => {
  it('takes a repo for a vault that has none', async () => {
    setResticRepo.mockResolvedValue({
      vaults: [vault('notes', '/backup/notes')],
      git: true,
      restic: true,
      restic_password_set: false,
    });
    show([vault('notes', null)]);

    const field = await screen.findByPlaceholderText(/\/backup\/notes/);
    await fireEvent.input(field, { target: { value: '/backup/notes' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save the notes backup repo/i }));

    expect(setResticRepo).toHaveBeenCalledWith('/backup/notes', 'notes');
  });

  // The "panel has rendered" anchor is the remote field's own label, not a bare /git remote/:
  // the panel also explains, in prose, that Back up needs a git remote when no vault has one —
  // and a loose substring matched both, so a true sentence broke three tests.
  it('offers nothing at all where restic is not installed', async () => {
    show([vault('notes', null)], { restic: false });
    await screen.findByText(/your notes' git remote/i);
    // A form that configures a tool the machine does not have is a form that cannot be completed;
    // the capability line elsewhere in the panel is what says why.
    expect(screen.queryByPlaceholderText(/\/backup\/notes/)).toBeNull();
  });

  it('asks for a password once a repo exists, and says what losing it costs', async () => {
    setResticPassword.mockResolvedValue({
      vaults: [vault('notes', '/backup/notes')],
      git: true,
      restic: true,
      restic_password_set: true,
    });
    show([vault('notes', '/backup/notes')]);

    const field = await screen.findByPlaceholderText(/a password you can find again/i);
    // The warning is the point: restic has no recovery for a repository whose password is gone.
    expect(screen.getByText(/cannot be opened again/i)).toBeTruthy();

    await fireEvent.input(field, { target: { value: 'correct horse battery staple' } });
    await fireEvent.click(screen.getByRole('button', { name: /Save password/i }));

    expect(setResticPassword).toHaveBeenCalledWith('correct horse battery staple');
    expect(await screen.findByText(/Backup password saved/i)).toBeTruthy();
    // No secret is left in the page once the machine has it.
    expect(screen.queryByPlaceholderText(/a password you can find again/i)).toBeNull();
  });

  it('does not ask for a password while there is nothing for it to unlock', async () => {
    show([vault('notes', null)]);
    await screen.findByText(/your notes' git remote/i);
    expect(screen.queryByPlaceholderText(/a password you can find again/i)).toBeNull();
  });

  it('does not ask again once this machine has one', async () => {
    show([vault('notes', '/backup/notes')], { password: true });
    await screen.findByText(/your notes' git remote/i);
    expect(screen.queryByPlaceholderText(/a password you can find again/i)).toBeNull();
  });
});
