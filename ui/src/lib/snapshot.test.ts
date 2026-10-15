// **A snapshot is taken from the Back up menu** (`decisions.md#ui`, 2026-10-15). These were the
// backup panel's "pressing Back up" tests; the action left Settings and its rules came with it:
// a snapshot-only machine can take one, a failure names its vault, and what a snapshot held is
// read from restic's answer — never a fixed phrase, never zeros for "not reported".
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { backup, backupStatus, listVaults } = vi.hoisted(() => ({
  backup: vi.fn(),
  backupStatus: vi.fn(),
  listVaults: vi.fn(),
}));
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  backup,
  backupStatus,
  listVaults,
}));

import { takeSnapshots } from './snapshot';

const label = (n: string) => n;

/// One vault, with a snapshot repo, and whether this machine can run it now.
function oneVault({ ready = true, restic = true } = {}) {
  listVaults.mockResolvedValue([{ name: 'notes', path: '', restic_repo: '/backup/notes' }]);
  backupStatus.mockResolvedValue({
    vaults: [{ name: 'notes', remote: null, restic_ready: ready }],
    git: true,
    restic,
    restic_password_set: ready,
  });
}

const run = (over: Record<string, unknown> = {}) => ({
  vault: 'notes',
  notes_dir: 'notes',
  blobs: true,
  contents: {
    id: 'a1b2c3d4',
    files_new: 3,
    files_changed: 1,
    files_unmodified: 10,
    bytes_processed: 8192,
    bytes_added: 4096,
  },
  ...over,
});

beforeEach(() => vi.clearAllMocks());

describe('taking a snapshot', () => {
  // The 2026-09-04 rule: a machine set up for snapshots alone — no git remote anywhere — can take
  // one. It no longer waits on a git tier at all.
  it('runs for a vault with a repo, whether or not it has a git remote', async () => {
    oneVault();
    backup.mockResolvedValue(run());
    const lines = await takeSnapshots(label);
    expect(backup).toHaveBeenCalledWith('notes');
    expect(lines).toHaveLength(1);
    expect(lines[0].ok).toBe(true);
  });

  // "Backed up" while one vault sat still is the sentence this exists to prevent.
  it('names the vault whose snapshot failed, with the reason', async () => {
    oneVault();
    backup.mockRejectedValue(new Error('repository is locked'));
    const [line] = await takeSnapshots(label);
    expect(line.ok).toBe(false);
    expect(line.text).toContain('“notes”');
    expect(line.text).toContain('repository is locked');
  });

  it('names what the snapshot actually held, not a fixed phrase', async () => {
    oneVault();
    backup.mockResolvedValue(
      run({
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
      }),
    );
    const [line] = await takeSnapshots(label);
    // The vault keeps its notes in `docs/`, so the default name would be a lie.
    expect(line.text).toContain('docs/');
    expect(line.text).not.toContain('blobs/');
    expect(line.text).toContain('7 file(s)');
    expect(line.text).toContain('2 new or changed');
  });

  // `contents: null` is restic having written the snapshot and not described it — a third state,
  // not an empty snapshot.
  it('says the contents were not reported rather than printing zeros', async () => {
    oneVault();
    backup.mockResolvedValue(run({ contents: null }));
    const [line] = await takeSnapshots(label);
    expect(line.text).toContain('contents not reported');
    expect(line.text).toContain('notes/');
    expect(line.text).not.toContain('0 file(s)');
  });

  it('says why a configured vault could not be snapshotted, and does not try', async () => {
    oneVault({ ready: false });
    const [line] = await takeSnapshots(label);
    expect(backup).not.toHaveBeenCalled();
    expect(line.ok).toBe(false);
    expect(line.text).toContain('no backup password is set');

    oneVault({ ready: false, restic: false });
    const [again] = await takeSnapshots(label);
    expect(again.text).toContain('restic is not installed');
  });
});
