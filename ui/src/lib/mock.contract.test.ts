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
import type { BackupStatus, Config, VaultInfo } from './types';

const config = () => handle<Config>('config', {});
const status = () => handle<BackupStatus>('backup_status', {});
const vaults = () => handle<VaultInfo[]>('list_vaults', {});

beforeEach(() => reset());

describe('the mock answers one fact the same way from every arm', () => {
  // **Retired as a cross-arm test, because the duplication is gone.** This once held `config` and
  // `backup_status` together over a vault's snapshot repo — they had drifted, and `config`
  // hardcoded `null` while `backup_status` read real state. Since 2026-09-08 the repo is reported
  // by `list_vaults` alone (`VaultInfo.restic_repo`) and by nothing else, so there is no second
  // answer to disagree with. What is left worth asserting is that the one producer reflects the
  // writer — a fact with one source can still be a fact nobody updated.
  it('the one arm that reports a repository reflects the one that sets it', async () => {
    const name = (await vaults())[0].name;
    expect((await vaults())[0].restic_repo).toBeNull();

    await handle('set_restic_repo', { vault: name, repo: '/backup/lab' });

    expect((await vaults()).find((v) => v.name === name)?.restic_repo).toBe('/backup/lab');
  });

  it('config and backup_status agree about the password after one is set', async () => {
    expect((await config()).restic_password_set).toBe(false);

    await handle('set_restic_password', { password: 'correct horse battery staple' });

    expect((await status()).restic_password_set).toBe(true);
    expect((await config()).restic_password_set).toBe(true);
  });
});
