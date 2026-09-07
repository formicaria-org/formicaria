// **The mock has to agree with itself, because nothing else makes it.**
//
// `ui/src/lib/mock.ts` stands in for the Rust backend under `pnpm dev` and in every UI test here.
// It is hand-written, and ninety-odd of its arms answer through a bare `as T` — so two arms can
// report different values for one fact and `tsc` will not say a word. That is not hypothetical: the
// `config` arm hardcoded `repo: null` and `restic_password_set: false` while `set_restic_repo` and
// `set_restic_password` wrote to the mock's state and `backup_status` read it. Under `pnpm dev` you
// could save a repository in the backup panel and watch Settings go on reporting none — and the
// project's own notes carried this as an open defect for a week.
//
// **A type annotation would not have caught it.** Both shapes are `string | null`, so `satisfies
// Config` is satisfied by the wrong answer. What catches it is asserting that two arms answer the
// *same* question the same way, after a write. The annotation went on anyway; this is the part that
// does the work.
//
// These call the mock directly rather than rendering a component: the drift is in the backend
// stand-in, and a component test would only find it somewhere a panel happens to read both.

import { beforeEach, describe, expect, it } from 'vitest';
import { handle, reset } from './mock';
import type { BackupStatus, Config } from './types';

const config = () => handle<Config>('config', {});
const status = () => handle<BackupStatus>('backup_status', {});

beforeEach(() => reset());

describe('the mock answers one fact the same way from every arm', () => {
  it('config and backup_status agree about a repository after one is set', async () => {
    const name = (await config()).vaults[0].name;
    expect((await config()).restic[0].repo).toBeNull();

    await handle('set_restic_repo', { vault: name, repo: '/backup/lab' });

    // Settings reads `config`; the backup panel reads `backup_status`. They are the two surfaces
    // that disagreed, so both are asserted rather than just the one that was wrong.
    expect((await status()).vaults.find((s) => s.name === name)?.restic_repo).toBe('/backup/lab');
    expect((await config()).restic.find((r) => r.vault === name)?.repo).toBe('/backup/lab');
  });

  it('config and backup_status agree about the password after one is set', async () => {
    expect((await config()).restic_password_set).toBe(false);

    await handle('set_restic_password', { password: 'correct horse battery staple' });

    expect((await status()).restic_password_set).toBe(true);
    expect((await config()).restic_password_set).toBe(true);
  });
});
