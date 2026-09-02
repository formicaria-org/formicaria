// **Importing a Logseq graph or an Obsidian vault, from the only place the owner can reach it.**
//
// Three things are pinned here, and each of them is a decision that would be easy to undo by
// accident:
//
//  1. **The server owns the verdict.** The button follows `check_import`'s single `ok`, which
//     answers for the source *and* the destination. The moment this file ANDs two answers of its
//     own, it has become the second opinion that makes a button enable and then fail.
//  2. **The source is only read.** The copy has to say so before the button, because "point the
//     app at my notes" is a frightening thing to do and the reassurance is worthless afterwards.
//  3. **Stub notes are off unless asked for.** They are ordinary notes, so every one of them shows
//     up in every board, agenda and timeline — the default must be the safe one.

import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { checkImport, checkPath, config, runImport, listVaults } = vi.hoisted(() => ({
  checkImport: vi.fn(),
  checkPath: vi.fn(),
  config: vi.fn(),
  runImport: vi.fn(),
  listVaults: vi.fn(),
}));
// Spread the real module and override only what this surface calls: a hand-listed factory drops
// whatever else the import graph needs, and the failure reads as "no tests collected".
vi.mock('./ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./ipc')>()),
  checkImport,
  checkPath,
  config,
  runImport,
  listVaults,
}));

import NewVault from './NewVault.svelte';

const scan = (over: Partial<Record<string, unknown>> = {}) => ({
  format: 'logseq',
  label: 'Logseq',
  pages: 120,
  journals: 40,
  attachments: 6,
  attachmentBytes: 2_400_000,
  leftBehind: [],
  problem: null,
  ok: true,
  ...over,
});

const report = (over: Partial<Record<string, unknown>> = {}) => ({
  format: 'logseq',
  notes: 160,
  stubs: 0,
  alreadyImported: 0,
  attachments: 6,
  deduped: 0,
  links: 210,
  dangling: 3,
  danglingNames: ['Someday'],
  blocks: 12,
  blocksUnresolved: 0,
  renamedProperties: 0,
  leftBehind: [],
  warnings: [],
  recorded: true,
  vault: 'notes',
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  config.mockResolvedValue({
    vault_list: '/home/x/.config/formicaria/vaults.json',
    vault_list_writable: true,
    vaults: [{ name: 'personal', path: '/home/x/personal' }],
    restic: [],
    env: [],
    git: true,
    restic_installed: false,
    pdf_text: true,
    restic_password_set: false,
    vault_root: null,
  });
  checkPath.mockResolvedValue({ ok: true, name_ok: true, name_taken: false, path_taken: false });
  checkImport.mockResolvedValue(scan());
  listVaults.mockResolvedValue([]);
});

/** Switch to import mode and type a source folder, then wait for the debounced probe. */
async function intoImportMode(source = '~/graph') {
  render(NewVault, { props: { oncreated: vi.fn(), git: true, restic: false } });
  await fireEvent.click(await screen.findByRole('button', { name: /import from another app/i }));
  await fireEvent.input(screen.getByLabelText(/source folder/i), { target: { value: source } });
  await waitFor(() => expect(checkImport).toHaveBeenCalled(), { timeout: 3000 });
}

describe('importing another app’s notes', () => {
  it('states what is in the folder before the button is pressed', async () => {
    await intoImportMode();
    expect(await screen.findByText(/Logseq: 160 pages/)).toBeTruthy();
    expect(screen.getByText(/40 daily notes/)).toBeTruthy();
  });

  it('promises the source is only read, before anything happens', async () => {
    await intoImportMode();
    expect(screen.getByText(/only ever/i).textContent).toMatch(/read/i);
  });

  /** The whole point of `check_import` answering for both halves at once. */
  it('follows the server’s single verdict rather than forming its own', async () => {
    checkImport.mockResolvedValue(scan({ ok: false, problem: 'there is no folder at that path' }));
    await intoImportMode('/nope');
    expect(await screen.findByText('there is no folder at that path')).toBeTruthy();
    const button = screen.getByRole('button', { name: /import notes/i });
    expect(button).toHaveProperty('disabled', true);
  });

  it('imports into a new vault by default and shows what happened', async () => {
    runImport.mockResolvedValue(report());
    await intoImportMode();
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));

    await waitFor(() => expect(runImport).toHaveBeenCalled());
    // `vault` empty is what asks the server for a new one; `stubs` false is the safe default.
    const [source, vault, , , stubs] = runImport.mock.calls[0];
    expect(source).toBe('~/graph');
    expect(vault).toBe('');
    expect(stubs).toBe(false);

    expect(await screen.findByText(/Imported from Logseq/i)).toBeTruthy();
    expect(screen.getByText(/210 links between notes now work/)).toBeTruthy();
    // The bad news is reported too, or it is not a report.
    expect(screen.getByText(/left as plain text/)).toBeTruthy();
    expect(screen.getByText(/undone in one step/)).toBeTruthy();
  });

  it('can put the notes in a vault that already exists', async () => {
    runImport.mockResolvedValue(report({ vault: 'personal' }));
    await intoImportMode();
    await fireEvent.change(screen.getByLabelText(/put the notes in/i), {
      target: { value: 'personal' },
    });
    // Only the first two arguments matter here: once a vault is named, the server answers for
    // that vault and never looks at the new-vault name or folder (`destination_problem` returns
    // on the existing-vault branch before it reaches `check_path`).
    await waitFor(() => {
      const call = checkImport.mock.calls.at(-1);
      expect(call?.slice(0, 2)).toEqual(['~/graph', 'personal']);
    });
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));
    await waitFor(() => expect(runImport.mock.calls[0][1]).toBe('personal'));
  });

  /** Off by default, because a stub is an ordinary note and lands in every view. */
  it('only creates notes for linked-but-absent pages when asked', async () => {
    runImport.mockResolvedValue(report({ stubs: 2 }));
    await intoImportMode();
    await fireEvent.click(screen.getByLabelText(/pages that are only linked to/i));
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));
    await waitFor(() => expect(runImport.mock.calls[0][4]).toBe(true));
  });

  it('says when a graph is large enough to be felt', async () => {
    checkImport.mockResolvedValue(scan({ pages: 30_000 }));
    await intoImportMode();
    expect(await screen.findByText(/That is a lot of notes/)).toBeTruthy();
  });
});
