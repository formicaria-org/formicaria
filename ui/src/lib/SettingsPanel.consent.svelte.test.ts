// Consent is two independent answers, and the app must never let one imply the other: agreeing to
// keep a record of your own corrections is not agreeing to publish them. Collapsing the two is what
// stranded the only comparable open corpus of AI corrections, so it is pinned here.

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

describe('SettingsPanel — supervision consent', () => {
  it('starts as collect-yes / publish-no, because publication cannot be recalled', async () => {
    panel();
    const keep = (await screen.findAllByLabelText(/Keep a record of what you change/))[0];
    const share = (await screen.findAllByLabelText(/shared openly/))[0];
    expect((keep as HTMLInputElement).checked).toBe(true);
    expect((share as HTMLInputElement).checked).toBe(false);
    expect((await screen.findAllByText(/Sharing is off/)).length).toBeGreaterThan(0);
  });

  it('grants publishing without touching the other answer, and says it is irreversible', async () => {
    panel();
    const share = (await screen.findAllByLabelText(/shared openly/))[0];
    await fireEvent.click(share);

    const vaults = await mock.handle<VaultInfo[]>('list_vaults', {});
    expect(vaults[0].supervision).toEqual({ collect: true, publish: true });
    expect(vaults[1].supervision.publish).toBe(false); // one vault at a time, never all of them
    // The screen has to say the part that cannot be undone.
    expect((await screen.findAllByText(/cannot be taken back/)).length).toBeGreaterThan(0);
  });

  it('turning recording off does not silently leave sharing on', async () => {
    panel();
    const keep = (await screen.findAllByLabelText(/Keep a record of what you change/))[0];
    await fireEvent.click(keep);
    const vaults = await mock.handle<VaultInfo[]>('list_vaults', {});
    expect(vaults[0].supervision.collect).toBe(false);
    expect((await screen.findAllByText(/nothing is recorded/i)).length).toBeGreaterThan(0);
  });
});
