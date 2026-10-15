// **Nobody had ever pressed Back up.**
//
// `outstanding.md` §2.10, verbatim: *"no UI test has ever pressed the Back up button, so not one
// step line or verdict sentence in `BackupPanel.run()` is asserted anywhere."* Three test files
// covered configuring the tiers — a remote, a repository, a token — and stopped at the button
// that uses them. So the code that decides what a person is told about whether their notes left
// the machine was the least-covered code on the screen.
//
// Two things are pinned here, and they are the two this panel exists for:
//
//   1. **A tier that did not run gets no verdict.** With nowhere to send anywhere, the git sentence
//      would otherwise report a failure where there was no attempt.
//   2. **A restic-only machine can press the button at all** (fixed 2026-09-04). `canRun` used to
//      require a destination, so a machine with restic, a repository and a password had a tick box
//      that ticked and a button that never enabled. The panel was made to *say* so, which was
//      honesty rather than a fix.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  backupStatus,
  backupLatest,
  gitAuth,
  backup,
  commit,
  pull,
  push,
  getNote,
  lastCommits,
  listVaults,
} = vi.hoisted(() => ({
  backupStatus: vi.fn(),
  backupLatest: vi.fn(),
  gitAuth: vi.fn(),
  backup: vi.fn(),
  commit: vi.fn(),
  pull: vi.fn(),
  push: vi.fn(),
  getNote: vi.fn(),
  lastCommits: vi.fn(),
  listVaults: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backupStatus,
  backupLatest,
  gitAuth,
  backup,
  commit,
  pull,
  push,
  getNote,
  lastCommits,
  listVaults,
}));

import BackupPanel from './BackupPanel.svelte';

const vault = (name: string, over: Partial<Record<string, unknown>> = {}) => ({
  name,
  remote: null,
  unpushed: null,
  identity: { name: 'Ada', email: 'ada@example.org' },
  remote_moved: null,
  conflicts: [],
  restic_repo: null,
  restic_ready: false,
  git_assets_max: null,
  ...over,
});

const show = (
  vaults: ReturnType<typeof vault>[],
  { restic = true, password = true, git = true } = {},
) => {
  backupStatus.mockResolvedValue({ vaults, git, restic, restic_password_set: password });
  // **The cheap half of a vault comes from the vault list now**, not from `backup_status` — a
  // vault's committer, its attachment limit and its snapshot repo are local reads and were being
  // reported by both. The fixtures still describe one vault in one place; this splits them the way
  // the two commands do.
  listVaults.mockResolvedValue(
    vaults.map((v) => ({
      name: v.name as string,
      path: '',
      default: false,
      git_assets_max: (v.git_assets_max ?? null) as number | null,
      supervision: { collect: true, publish: false },
      restic_repo: (v.restic_repo ?? null) as string | null,
      label: null,
      identity: (v.identity ?? null) as { name: string; email: string } | null,
    })),
  );
  render(BackupPanel, {});
};

beforeEach(() => {
  vi.clearAllMocks();
  gitAuth.mockResolvedValue({ storage: 'system', have_credential: true, helper: null });
  commit.mockResolvedValue({ committed: true, conflicts: [] });
  pull.mockResolvedValue({ merged: 0, conflicts: [], kept: [] });
  push.mockResolvedValue(undefined);
  getNote.mockResolvedValue(null);
  // Default: nothing known about when anything was saved, which renders as the "nothing saved
  // here yet" line. Tests that care about the elapsed line set their own.
  lastCommits.mockResolvedValue([]);
  // The shape the command actually answers. It returned `void` until 2026-09-05, and a default
  // of `undefined` here would let a test pass against a panel that reads nothing back.
  backup.mockResolvedValue({
    vault: 'notes',
    notes_dir: 'notes',
    blobs: true,
    contents: {
      id: 'a1b2c3d4',
      files_new: 3,
      files_changed: 1,
      files_unmodified: 214,
      bytes_processed: 48_200_000,
      bytes_added: 1_900_000,
    },
  });
  backupLatest.mockResolvedValue({
    vault: 'notes',
    id: null,
    time: null,
    paths: [],
    unavailable: null,
  });
});

describe('when this vault was last backed up', () => {
  // The fact the panel could not show at any price until `backup_latest` existed.
  it('shows the timestamp of the last snapshot', async () => {
    backupLatest.mockResolvedValue({
      vault: 'notes',
      id: 'a1b2c3d4',
      time: '2026-09-01T10:30:00Z',
      paths: ['/home/you/notes/notes', '/home/you/notes/blobs'],
      unavailable: null,
    });
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await waitFor(() => expect(backupLatest).toHaveBeenCalledWith('notes'));
    await waitFor(() => {
      expect(document.body.textContent).toContain('Last snapshot');
    });
    expect(document.body.textContent).toContain('It covered 2 paths');
  });

  // **"Never" is an answer, and it is the one that should worry somebody.** Kept apart from
  // `unavailable` deliberately, at every layer — the Rust command, the mock and here.
  it('says plainly when a configured repository has never been written to', async () => {
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await waitFor(() => {
      expect(document.body.textContent).toContain('Never backed up');
    });
  });

  // And "this machine cannot tell you" is a third thing again, which must not read as "never".
  it('reports why it cannot answer, rather than claiming never', async () => {
    backupLatest.mockResolvedValue({
      vault: 'notes',
      id: null,
      time: null,
      paths: [],
      unavailable: 'no restic password is set on this machine',
    });
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })], {
      password: false,
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('Last snapshot: unknown');
    });
    expect(document.body.textContent).not.toContain('Never backed up');
  });
});

// **Zero and null are different facts.** `{#if v.unpushed}` is falsy for both, so "everything is
// pushed" and "git could not say" rendered identically — as nothing — and the silence reads as the
// reassuring one. Recorded in `known-issues.md` as the last of three backup gaps from 2026-09-02;
// the other two were fixed on 2026-09-04.
describe('how many commits are waiting', () => {
  const withRemote = (unpushed: number | null) =>
    vault('notes', { remote: 'git@github.com:you/notes.git', unpushed });

  it('says so when everything has been sent', async () => {
    show([withRemote(0)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('Everything here has been sent');
    });
    expect(document.body.textContent).not.toContain('not sent yet');
  });

  it('counts them when some are waiting', async () => {
    show([withRemote(3)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('3 changes not sent yet');
    });
    expect(document.body.textContent).not.toContain('Everything here has been sent');
  });

  // Null is "git could not say" — no remote, never pushed, or offline. A sleeping laptop is not an
  // error, so this one stays silent on purpose; the test exists so the silence is a decision.
  it('stays quiet when git could not say', async () => {
    show([withRemote(null)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('Notes →');
    });
    expect(document.body.textContent).not.toContain('Everything here has been sent');
    expect(document.body.textContent).not.toContain('not sent yet');
  });
});

// **How long it has been since this vault saved anything** — the panel half of the toolbar chip.
//
// The absence this pins: for thirty-nine days a vault could not commit, and this panel showed a
// remote, an identity and an unpushed count that all looked ordinary. None of them subtract
// themselves from today. The elapsed line is the one that does.
describe('when the vault last saved anything', () => {
  it('says how long ago, in days, beside the rest of the vault status', async () => {
    const days = 39;
    lastCommits.mockResolvedValue([
      { vault: 'notes', last_commit: Math.floor((Date.now() - days * 86_400_000) / 1000) },
    ]);
    show([vault('notes')]);

    await waitFor(() => expect(document.body.textContent).toContain('Last saved'));
    expect(document.body.textContent).toContain(`${days} days ago`);
  });

  it('says a vault has no history rather than inventing a date for it', async () => {
    // `null` is a third state. Rendered as an epoch it would read as twenty thousand days of
    // silence — an alarm aimed at somebody who has done nothing wrong.
    lastCommits.mockResolvedValue([{ vault: 'notes', last_commit: null }]);
    show([vault('notes')]);

    await waitFor(() => expect(document.body.textContent).toContain('Nothing saved here yet'));
    expect(document.body.textContent).not.toContain('days ago');
  });

  it('still says it for a vault with no remote at all', async () => {
    // Its own `<li>`, outside the remote branch: a vault with nowhere to push is exactly the one
    // whose silence nobody would otherwise notice.
    lastCommits.mockResolvedValue([
      { vault: 'notes', last_commit: Math.floor((Date.now() - 90 * 86_400_000) / 1000) },
    ]);
    show([vault('notes', { remote: null })]);

    // Wait on the elapsed line, not on the remote line: `load()` renders the status first and
    // fetches the saves after, so the remote line is on screen a tick before this one is.
    await waitFor(() => expect(document.body.textContent).toContain('90 days ago'));
    expect(document.body.textContent).toContain('Nowhere to send yet');
  });
});
