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
//   1. **A tier that did not run gets no verdict.** With no git remote anywhere, the git sentence
//      would otherwise report a failure where there was no attempt.
//   2. **A restic-only machine can press the button at all** (fixed 2026-09-04). `canRun` used to
//      require a git remote, so a machine with restic, a repository and a password had a tick box
//      that ticked and a button that never enabled. The panel was made to *say* so, which was
//      honesty rather than a fix.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backupStatus, backupLatest, gitAuth, backup } = vi.hoisted(() => ({
  backupStatus: vi.fn(),
  backupLatest: vi.fn(),
  gitAuth: vi.fn(),
  backup: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backupStatus,
  backupLatest,
  gitAuth,
  backup,
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
  render(BackupPanel, { onclose: () => {}, onnewvault: () => {} });
};

beforeEach(() => {
  vi.clearAllMocks();
  gitAuth.mockResolvedValue({ storage: 'system', have_credential: true, helper: null });
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

describe('pressing Back up', () => {
  // The whole point of the 2026-09-04 fix. Before it this button was permanently disabled on a
  // machine that was, in fact, entirely capable of backing up.
  it('a restic-only machine can run the snapshot tier on its own', async () => {
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    const box = await screen.findByRole('checkbox', { name: /snapshot|attachments|media/i });
    await fireEvent.click(box);

    const button = (await screen.findByRole('button', { name: /^Back up/i })) as HTMLButtonElement;
    // The house idiom: the DOM property, not a jest-dom matcher — this project does not load them.
    await waitFor(() => expect(button.disabled).toBe(false));

    await fireEvent.click(button);
    await waitFor(() => expect(backup).toHaveBeenCalledWith('notes'));
  });

  // **The assertion that would have caught the old wording.** With nothing to push, "your notes
  // are still on this machine" is not merely unhelpful — it is false the moment the snapshot tier
  // has just sent them somewhere, because a snapshot carries the notes directory as well as
  // `blobs/`.
  it('does not report a git outcome when no vault has a remote', async () => {
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await fireEvent.click(await screen.findByRole('checkbox', { name: /snapshot|attachments|media/i }));
    const button = (await screen.findByRole('button', { name: /^Back up/i })) as HTMLButtonElement;
    // The house idiom: the DOM property, not a jest-dom matcher — this project does not load them.
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);

    await waitFor(() => expect(backup).toHaveBeenCalled());
    const body = document.body.textContent ?? '';
    expect(body).toContain('No vault has a git remote');
    expect(body).not.toContain('Your notes are still on this machine');
    expect(body).not.toContain('Your notes are off this machine');
  });

  // A vault whose snapshot fails must be **named**, not folded into a cheerful summary. This is
  // the sentence the panel exists to prevent: "backed up" while one vault sat still.
  it('names the vault whose snapshot failed', async () => {
    backup.mockRejectedValue(new Error('repository is locked'));
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await fireEvent.click(await screen.findByRole('checkbox', { name: /snapshot|attachments|media/i }));
    const button = (await screen.findByRole('button', { name: /^Back up/i })) as HTMLButtonElement;
    // The house idiom: the DOM property, not a jest-dom matcher — this project does not load them.
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);

    await waitFor(() => {
      expect(document.body.textContent).toContain('repository is locked');
    });
    expect(document.body.textContent).toContain('notes');
  });

  // **The fixed phrase, and why it had to go.** This line said "notes and attachments" over every
  // vault for as long as `backup` answered nothing — including the ordinary vault that has no
  // `blobs/` yet, which is most of them on a desktop. The assertion is scoped to the step line
  // itself, because the panel's attachments explainer mentions `blobs/` too and a search of the
  // whole document would pass for the wrong reason.
  it('names what the snapshot actually held, not a fixed phrase', async () => {
    backup.mockResolvedValue({
      vault: 'notes',
      notes_dir: 'docs',
      blobs: false,
      contents: {
        id: 'a1b2c3d4',
        files_new: 2,
        files_changed: 0,
        files_unmodified: 5,
        bytes_processed: 4096,
        bytes_added: 3455,
      },
    });
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await fireEvent.click(await screen.findByRole('checkbox', { name: /snapshot|attachments|media/i }));
    const button = (await screen.findByRole('button', { name: /^Back up/i })) as HTMLButtonElement;
    // The house idiom: the DOM property, not a jest-dom matcher — this project does not load them.
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);

    const line = await screen.findByText(/^Snapshot/);
    // The vault keeps its notes in `docs/`, so the default name would be a lie.
    expect(line.textContent).toContain('docs/');
    expect(line.textContent).not.toContain('blobs/');
    expect(line.textContent).toContain('7 file(s)');
    expect(line.textContent).toContain('2 new or changed');
  });

  // **A snapshot restic did not describe is not an empty snapshot.** `contents: null` is the
  // third state — the same distinction `backup_latest` draws between "never backed up" and "this
  // machine cannot tell you" — and the line must say so rather than render zeros.
  it('says the contents were not reported rather than printing zeros', async () => {
    backup.mockResolvedValue({
      vault: 'notes',
      notes_dir: 'notes',
      blobs: true,
      contents: null,
    });
    show([vault('notes', { restic_repo: '/backup/notes', restic_ready: true })]);

    await fireEvent.click(await screen.findByRole('checkbox', { name: /snapshot|attachments|media/i }));
    const button = (await screen.findByRole('button', { name: /^Back up/i })) as HTMLButtonElement;
    // The house idiom: the DOM property, not a jest-dom matcher — this project does not load them.
    await waitFor(() => expect(button.disabled).toBe(false));
    await fireEvent.click(button);

    const line = await screen.findByText(/^Snapshot/);
    expect(line.textContent).toContain('contents not reported');
    // What went in is still known — we chose the directories — so it is still said.
    expect(line.textContent).toContain('notes/');
    expect(line.textContent).not.toContain('0 file(s)');
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

  it('says so when everything is pushed', async () => {
    show([withRemote(0)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('Everything here is pushed');
    });
    expect(document.body.textContent).not.toContain('not pushed');
  });

  it('counts them when some are waiting', async () => {
    show([withRemote(3)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('3 commits not pushed');
    });
    expect(document.body.textContent).not.toContain('Everything here is pushed');
  });

  // Null is "git could not say" — no remote, never pushed, or offline. A sleeping laptop is not an
  // error, so this one stays silent on purpose; the test exists so the silence is a decision.
  it('stays quiet when git could not say', async () => {
    show([withRemote(null)]);
    await waitFor(() => {
      expect(document.body.textContent).toContain('Notes →');
    });
    expect(document.body.textContent).not.toContain('Everything here is pushed');
    expect(document.body.textContent).not.toContain('not pushed');
  });
});
