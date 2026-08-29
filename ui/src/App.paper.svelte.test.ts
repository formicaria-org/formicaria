/// **Adding a paper is one paste, and it works with the network off.**
///
/// The core links no HTTP client — `fm-agent-run`'s Cargo.toml records that as the owner's ruling
/// ("the core stays minimal, an agent may carry its own heavier deps") — so a DOI can be
/// *recognised* here but never resolved. What fills a full record offline is the citation the user
/// already has: every publisher page and every reference manager exports BibTeX.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, afterEach, expect, test, vi } from 'vitest';

const { createPaper } = vi.hoisted(() => ({ createPaper: vi.fn() }));
vi.mock('./lib/ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/ipc')>()),
  createPaper,
}));

import App from './App.svelte';
import { clearFaults } from './lib/mock';

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  clearFaults();
  createPaper.mockResolvedValue({
    id: 'M0CKPAPER0000000000000001',
    type: 'note',
    title: 'Attention Is All You Need',
    tags: ['paper'],
    props: {},
    vault: 'personal',
  });
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

async function openAddPaper() {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: /new|create|add/i }));
  const item = await screen.findByText('New paper');
  await fireEvent.click(item);
  return await screen.findByRole('dialog', { name: /add a paper/i });
}

test('one box takes a citation, an identifier or a bare title', async () => {
  await openAddPaper();
  const box = screen.getByLabelText('paper citation or identifier');
  expect(box.tagName).toBe('TEXTAREA');
  // The promise the dialog makes, in the words a non-technical reader needs: nothing is looked up.
  expect(screen.getByText(/nothing is looked up online/i)).toBeTruthy();
});

test('a pasted BibTeX entry is handed to the backend verbatim', async () => {
  await openAddPaper();
  const entry = '@inproceedings{v2017, title={Attention Is All You Need}, year={2017}}';
  await fireEvent.input(screen.getByLabelText('paper citation or identifier'), {
    target: { value: entry },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add paper' }));
  // Parsing lives in Rust, tested there, with one implementation — the UI does not second-guess it.
  await waitFor(() => expect(createPaper).toHaveBeenCalledWith(entry, expect.anything()));
});

test('an identifier alone is enough', async () => {
  await openAddPaper();
  await fireEvent.input(screen.getByLabelText('paper citation or identifier'), {
    target: { value: 'https://arxiv.org/abs/1706.03762' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add paper' }));
  await waitFor(() =>
    expect(createPaper).toHaveBeenCalledWith('https://arxiv.org/abs/1706.03762', expect.anything()),
  );
});
