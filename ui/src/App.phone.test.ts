// **The whole app, on a phone, doing the things it is actually used for.**
//
// The owner ran the Android build for weeks and reported it unusable: freezes on big notes,
// freezes when adding a photo, general lag. The suite could not see any of it, because until
// `harness.ts` existed no test had run a single line of the mobile path — `ipc.ts` picks its
// backend on `'__TAURI_INTERNALS__' in window`, false in jsdom, and jsdom's `matchMedia` reports
// a fine pointer, so every touch branch was dead too.
//
// **The assertions are work budgets, not timings.** A stopwatch in jsdom measures this machine;
// what actually hurts on the device is how many commands a gesture causes and how many bytes each
// one carries — and those numbers are identical everywhere, so they can be asserted exactly.
// `countingBridge` records both, keyed by the dispatch name rather than the `fm` envelope, so a
// budget reads in the app's own vocabulary.
//
// Deliberately a *ceiling* per journey rather than an exact transcript: an exact one fails on any
// harmless reordering and gets deleted, while a ceiling fails only when a change makes the phone
// do materially more work. Each number below is set just above what the app currently does.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import { asPhone, asDesktop, fakeFile, type CountingBridge } from './lib/harness';

const { mermaidInit, mermaidRender, katexRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  katexRender: vi.fn((tex: string) => `<span class="katex">${tex}</span>`),
}));
vi.mock('mermaid', () => ({ default: { initialize: mermaidInit, render: mermaidRender } }));
vi.mock('katex', () => ({ default: { renderToString: katexRender } }));
vi.mock('katex/dist/katex.min.css', () => ({}));

const lsStore = new Map<string, string>();
let bridge: CountingBridge;

beforeEach(() => {
  mermaidRender.mockResolvedValue({ svg: '<svg data-mock-mermaid="1"></svg>' });
  lsStore.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => lsStore.get(k) ?? null,
    setItem: (k: string, v: string) => void lsStore.set(k, String(v)),
    removeItem: (k: string) => void lsStore.delete(k),
    clear: () => lsStore.clear(),
    key: (i: number) => [...lsStore.keys()][i] ?? null,
    get length() {
      return lsStore.size;
    },
  });
  mock.reset();
  bridge = asPhone();
});

afterEach(() => {
  asDesktop();
  lsStore.clear();
  vi.unstubAllGlobals();
});

/// A vault the size the owner actually has. Nine fixture notes were never going to reveal a
/// full-corpus scan — that is the whole reason these defects survived a 39-file suite.
const REAL_VAULT = 800;

describe('a phone-sized vault', () => {
  it('opens onto a board without a command per note', async () => {
    mock.seed({ notes: REAL_VAULT });
    render(App);

    await screen.findByText(/GAE lambda interacts badly/);

    // The whole cold open. A handful of feed/vault/status commands — never anything proportional
    // to the note count, which is what a per-note `get` on boot would look like.
    expect(bridge.calls.length).toBeLessThan(20);
    expect(bridge.count('get')).toBeLessThan(5);
  });

  it('opens a large note without a round trip per reference', async () => {
    // Two references, each named twice. Before `resolveAssets`/`resolveNotes` deduped, that was
    // four sequential blocking round trips for two distinct things; now it is two, concurrently.
    const ids = mock.seed({ notes: 40 });
    const target = ids[0];
    const body = [
      '# A note that points at things',
      '',
      '![figure](asset:sha256-deadbeef)',
      '',
      'See [the ablation](note:M0CK0000000000000000000001).',
      '',
      '![the same figure again](asset:sha256-deadbeef)',
      '',
      'And [the ablation again](note:M0CK0000000000000000000001).',
    ].join('\n');
    await mock.handle('update_body', { id: target, body, base: '' });

    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    bridge.reset();

    await fireEvent.click(await screen.findByText(/Seeded 0/));
    await screen.findByRole('heading', { name: /A note that points at things/ });

    // One `asset_status` for the one distinct asset, and the note chips resolved without a `get`
    // each. Asserted as exact counts because deduplication is exactly what is being pinned.
    await waitFor(() => expect(bridge.count('asset_status')).toBe(1));
    expect(bridge.count('asset_status')).toBe(1);

    // **Wait for the note walk to finish before budgeting it.** `resolveNotes` replaces each
    // `<a href="note:…">` with a chip, so no such anchor remaining means every reference has been
    // resolved. Without this the `get` budget below was asserted while the walk was still in
    // flight and passed whether or not it deduplicated — an adversarial review caught it, and it
    // is the exact failure mode a work budget is most prone to: measuring work that has not
    // happened yet.
    await waitFor(() => expect(document.querySelectorAll('a[href^="note:"]').length).toBe(0));
    // `get` covers the note being opened plus at most one for the distinct referenced note —
    // *not* one per mention, which is what it was before the dedupe.
    expect(bridge.count('get')).toBeLessThanOrEqual(2);
  });

  it('adds a photo in one command, carrying the file once', async () => {
    mock.seed({ notes: 40 });
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    bridge.reset();

    // Straight at the transport: the picker is an OS surface, and what is being pinned is what
    // crosses the bridge, not how the file gets chosen.
    const { ingestFile } = await import('./lib/ipc');
    const { file, bytes } = fakeFile('IMG_2201.jpg', 500_000);
    const meta = await ingestFile(file, 'personal');

    expect(bridge.count('ingest')).toBe(1);
    expect(bridge.ingested[0].length).toBe(bytes.length);
    // Base64 is 4 bytes per 3, and nothing may copy the payload into the message twice.
    expect(bridge.bytes('ingest')).toBeLessThan(bytes.length * 1.4 + 4096);
    expect(meta.assets[0]).toMatch(/^sha256:/);
  });

  it('searches without loading the corpus into the page', async () => {
    mock.seed({ notes: REAL_VAULT });
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    bridge.reset();

    await fireEvent.click(screen.getByRole('button', { name: 'search notes' }));
    const box = await screen.findByRole('searchbox').catch(() => screen.getByLabelText(/search/i));
    await fireEvent.input(box, { target: { value: 'gradients' } });

    await waitFor(() => expect(bridge.count('search')).toBeGreaterThan(0));
    // Debounced, so typing one word is a small number of queries — never one per keystroke, and
    // never a `get` per hit (the rows carry their own previews).
    expect(bridge.count('search')).toBeLessThan(4);
    expect(bridge.count('get')).toBe(0);
  });
});

describe('the touch shell', () => {
  it('renders the coarse-pointer surfaces', async () => {
    // Not a screenshot test — jsdom applies no CSS. What is checkable is that the branches keyed
    // on `matchMedia('(pointer: coarse)')` are *taken*, which they never were before
    // `test-setup.ts` supplied a stub. `NotePanel`'s persistent format bar is the visible one: on
    // a fine pointer it is a floating bar shown only while text is selected, on a coarse pointer
    // it is always present above the editor, because the float hides behind Android's own
    // Cut/Copy menu.
    const ids = mock.seed({ notes: 5 });
    await mock.handle('update_body', { id: ids[0], body: 'something to edit', base: '' });
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    await fireEvent.click(await screen.findByText(/Seeded 0/));
    await fireEvent.click(await screen.findByLabelText('note options'));
    await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
    await screen.findByLabelText('note body (Markdown)');

    // Present without any selection having been made — the coarse-pointer branch.
    expect(await screen.findByRole('toolbar', { name: 'format text' })).toBeTruthy();
  });
});
