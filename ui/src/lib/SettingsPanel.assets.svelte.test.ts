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

import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import SettingsPanel from './SettingsPanel.svelte';
import type { VaultInfo } from './types';

function panel() {
  return render(SettingsPanel, {
    onclose: () => {},
    onnewvault: () => {},
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

  // **Music and videos apart** (2026-11-21: *"a dedicated filter for not syncing all the heavy
  // music part … user decides"*). Under the size limit, sound and video stay out until said
  // otherwise; the sentence under the field says what is left out.
  it('keeps music and videos out under the limit until the person lets them in', async () => {
    await mock.handle('set_git_media', { vault: 'lab', sound: false, video: false });
    const { container } = panel();
    await screen.findAllByText(/Send attachments under/);
    const music = screen.getAllByLabelText(
      'Music and recordings travel with the notes too',
    ) as HTMLInputElement[];
    expect(music.length).toBe(1);
    expect(music[0].checked).toBe(false);
    expect(container.textContent).toMatch(
      /pushed with your notes, except music, recordings and videos/,
    );
    await fireEvent.click(music[0]);
    const vaults = await mock.handle<VaultInfo[]>('list_vaults', {});
    const lab = vaults.find((v) => v.name === 'lab');
    expect(lab?.git_sound).toBe(true);
    expect(lab?.git_video).toBe(false);
    await screen.findAllByText(/except videos/);
  });

  // **Unused attachments** (2026-11-21: *"a broad clean up option in settings to allow the user to
  // remove assets that nobody is using"*): looked for, put aside for 30 days, put back.
  it('finds a file no note uses, puts it aside, and puts it back', async () => {
    mock.mockBlobs['lab'] = [{ hash: 'ab'.repeat(32), size: 3_000_000, kind: 'sound' }];
    const { container } = panel();
    await screen.findAllByText(/Send attachments under/);
    const groups = [
      container.querySelector('[aria-label="unused attachments in lab-notes"]') as HTMLElement,
    ];
    expect(groups[0]).toBeTruthy();
    await fireEvent.click(within(groups[0]).getByText('Look for attachments no note uses…'));
    const box = (await within(groups[0]).findByLabelText(
      /a sound · sound · 3.0 MB/,
    )) as HTMLInputElement;
    expect(box.checked).toBe(true);
    await fireEvent.click(within(groups[0]).getByText(/Remove selected \(1, 3.0 MB\)/));
    expect(await within(groups[0]).findByText(/Put aside 1 file \(3.0 MB\)/)).toBeTruthy();
    expect(mock.mockBlobs['lab']).toEqual([]);
    await fireEvent.click(within(groups[0]).getByText('Put them back'));
    expect(await within(groups[0]).findByText(/Put back 1 file/)).toBeTruthy();
    expect(mock.mockBlobs['lab'].map((b) => b.hash)).toEqual(['ab'.repeat(32)]);
  });
});
