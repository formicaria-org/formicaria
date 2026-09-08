// **The attachment limit, and the ceiling it cannot pass.**
//
// There is no git-lfs in this project, so an attachment that travels in git is permanent history:
// every clone downloads it again, forever, and it cannot be taken back without rewriting history
// other people have already pulled. Above ~100 MB most hosts refuse the push outright — and they
// refuse it *after* the commit is made, which is the worst moment to find out.
//
// So the field has three bands, and what is pinned here is that the screen says which one you are
// in: quiet below the warning line, explicit between there and the ceiling, and refused above it.
// A limit that is merely accepted in silence is how someone discovers the wall from `git push`.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import SettingsPanel from './SettingsPanel.svelte';
import type { VaultInfo } from './types';

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onbackup: () => {},
    layout: 'auto',
    onlayout: () => {},
    columns: 2,
    oncolumns: () => {},
    theme: 'system',
    ontheme: () => {},
    commands: [],
  } as never);
}

const field = async (vault: string) =>
  (await screen.findByLabelText(`largest attachment to send for ${vault}`)) as HTMLInputElement;

describe('SettingsPanel — how large an attachment may travel in git', () => {
  it('says nothing extra about a limit that is unremarkable', async () => {
    panel();
    // `lab` starts at 2MB in the mock — well under the warning line.
    const input = await field('lab-notes');
    expect(input.value).toBe('2MB');
    expect(screen.queryByText(/That is large for git/)).toBeNull();
    expect(screen.queryByText(/more than will ever be sent/)).toBeNull();
  });

  it('names the cost once the limit is large, without refusing it', async () => {
    panel();
    await fireEvent.change(await field('personal'), { target: { value: '80MB' } });

    // It saved: this band works, and the warning is information rather than a rejection.
    const vaults = await mock.handle<VaultInfo[]>('list_vaults', {});
    expect(vaults.find((v) => v.name === 'personal')?.git_assets_max).toBe(80_000_000);

    const warning = await screen.findByText(/That is large for git/);
    expect(warning.textContent).toMatch(/every\s+clone/);
    // The way out is named, not just the problem.
    expect(warning.textContent).toMatch(/backup\s+snapshot/);
  });

  it('refuses a limit past the ceiling, and says what the ceiling is', async () => {
    panel();
    const input = await field('personal');
    await fireEvent.change(input, { target: { value: '500MB' } });

    expect(await screen.findByText(/100MB/)).toBeTruthy();
    expect((await screen.findAllByText(/most hosts refuse the push/)).length).toBeGreaterThan(0);

    // And nothing was half-applied: the vault keeps whatever it had.
    const vaults = await mock.handle<VaultInfo[]>('list_vaults', {});
    expect(vaults.find((v) => v.name === 'personal')?.git_assets_max).not.toBe(500_000_000);
  });

  it('states the bound before you have chosen anything at all', async () => {
    // `mock.ts` keeps its vaults in module state, so the tests above have already written to
    // `personal`. Clear it explicitly rather than relying on running first — an order-dependent
    // test is one that passes until someone adds a case above it.
    await mock.handle('set_git_assets_max', { vault: 'personal', max: '' });
    panel();
    // With no limit — the default — the screen still says how far this can go, so the ceiling is
    // discoverable without first typing something that gets refused.
    expect((await screen.findAllByText(/notes only/)).length).toBeGreaterThan(0);
    expect((await screen.findAllByText(/100MB per file/)).length).toBeGreaterThan(0);
  });
});
