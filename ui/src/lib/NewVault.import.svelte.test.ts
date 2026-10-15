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
  kinds: [
    { kind: 'bag', count: 16, bytes: 1_900_000_000 },
    { kind: 'png', count: 600, bytes: 300_000_000 },
  ],
  drawings: 0,
  previous: [],
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
  leftOut: 0,
  leftOutNames: [],
  tagsFromLinks: 3,
  drawings: 0,
  replaced: 0,
  keptEdited: 0,
  keptEditedNames: [],
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  config.mockResolvedValue({
    vault_list: '/home/x/.config/formicaria/vaults.json',
    vault_list_writable: true,
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
  // **The destination picker's vaults come from `list_vaults` now, not from `config`.** Both used
  // to carry the list; the settings screen and this form kept private copies of it, and the copies
  // drifted. One producer, so this is where the fixture belongs.
  listVaults.mockResolvedValue([{ name: 'personal', path: '/home/x/personal' }]);
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
    // `vault` empty is what asks the server for a new one; `stubs` false is the safe default; and
    // by default nothing is replaced and every attachment comes over.
    const [source, vault, , , stubs, rules] = runImport.mock.calls[0];
    expect(source).toBe('~/graph');
    expect(vault).toBe('');
    expect(stubs).toBe(false);
    expect(rules).toEqual({ replace: false });

    expect(await screen.findByText(/Imported from Logseq/i)).toBeTruthy();
    expect(screen.getByText(/210 links between notes now work/)).toBeTruthy();
    // The bad news is reported too, or it is not a report.
    expect(screen.getByText(/now a tag on the note that made it/)).toBeTruthy();
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

describe('importing a folder again, and choosing what comes over', () => {
  /** `decisions.md#data`, 2026-10-15: the same graph is not imported twice by accident. */
  it('names the vault that already holds this folder and offers to update it instead', async () => {
    checkImport.mockResolvedValue(scan({ previous: [{ vault: 'personal', pages: 150, of: 160 }] }));
    runImport.mockResolvedValue(
      report({
        vault: 'personal',
        replaced: 149,
        keptEdited: 1,
        keptEditedNames: ['Reading list'],
      }),
    );
    await intoImportMode();
    expect(await screen.findByText(/already imported into/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: /update personal instead/i }));
    // Updating means replacing, and the switch says so where the person can see and undo it.
    expect(
      (screen.getByLabelText(/replace what an earlier import/i) as HTMLInputElement).checked,
    ).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));
    await waitFor(() => expect(runImport).toHaveBeenCalled());
    const [, vault, , , , rules] = runImport.mock.calls[0];
    expect(vault).toBe('personal');
    expect(rules).toEqual({ replace: true });
    // What was kept is named, so the person knows their edits are safe.
    expect(
      await screen.findByText(/149 notes from the earlier import were brought up to date/),
    ).toBeTruthy();
    expect(screen.getByText(/kept as you left them: Reading list/)).toBeTruthy();
  });

  it('lets the person leave out a kind of attachment and anything over a size', async () => {
    runImport.mockResolvedValue(report({ leftOut: 16, leftOutNames: ['run_1.bag'] }));
    await intoImportMode();
    // The kinds are shown with their sizes, so the choice is an informed one.
    await fireEvent.click(await screen.findByLabelText(/leave some out/i));
    expect(screen.getByText(/Leave out \.bag — 16 files, 1\.9 GB/)).toBeTruthy();
    await fireEvent.click(screen.getByLabelText(/Leave out \.bag/));
    await fireEvent.input(screen.getByLabelText(/larger than/i), { target: { value: '100' } });
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));
    await waitFor(() => expect(runImport).toHaveBeenCalled());
    expect(runImport.mock.calls[0][5]).toEqual({ replace: false, maxMb: 100, skipKinds: ['bag'] });
    expect(await screen.findByText(/16 attachments were left out, as you asked/)).toBeTruthy();
  });

  /** `decisions.md#vault`, 2026-10-15: a vault holds every byte it owns — attachments are copied,
   *  and the space that takes is stated before the button is pressed. */
  it('says attachments are copied, and how much space they take, before importing', async () => {
    await intoImportMode();
    expect(await screen.findByText(/copied into the vault — up to 2\.4 MB/)).toBeTruthy();
  });
});

describe('what an import that skipped pages says', () => {
  it('says how to bring skipped pages up to date, and lists left-out files readably', async () => {
    runImport.mockResolvedValue(
      report({ notes: 0, alreadyImported: 1182, leftOut: 14, leftOutNames: ['a.bag', 'b.bag'] }),
    );
    await intoImportMode();
    await fireEvent.click(screen.getByRole('button', { name: /import notes/i }));
    expect(await screen.findByText(/were imported into this vault before/)).toBeTruthy();
    expect(screen.getByText(/To bring them up to date/)).toBeTruthy();
    // A space before the list, and a sign that the list is a sample when it is one.
    expect(screen.getByText(/left out, as you asked \(a\.bag, b\.bag, …\)\./)).toBeTruthy();
  });
});
