import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { newPane } from './lib/panes';

// Layer-2 end-to-end: mount the REAL app and drive it the way a user does —
// clicking through the board, switching views, re-grouping, opening a card,
// editing it, closing it. No Tauri window: `ipc.ts` sees no `__TAURI_INTERNALS__`
// in jsdom and falls back to the in-memory mock backend (`mock.ts`), whose JSON
// shape Layer 1 pins against the real Rust DTOs. The only things stubbed are the
// two heavy lazy upgrades (KaTeX, Mermaid), exactly as in render.test.ts, so the
// read view renders hermetically.
const { mermaidInit, mermaidRender, katexRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  katexRender: vi.fn((tex: string) => `<span class="katex">${tex}</span>`),
}));
vi.mock('mermaid', () => ({ default: { initialize: mermaidInit, render: mermaidRender } }));
vi.mock('katex', () => ({ default: { renderToString: katexRender } }));
vi.mock('katex/dist/katex.min.css', () => ({}));

// jsdom in this config exposes no global `localStorage`, and the app wraps every access in
// try/catch — so persistence simply no-ops in tests, which is exactly why the duplicate-id
// crash (a reload from real persisted state) went uncaught. Provide a minimal in-memory
// `localStorage` so a persisted workspace can be seeded, and reset it between tests.
const lsStore = new Map<string, string>();
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
});
afterEach(() => {
  lsStore.clear();
  vi.unstubAllGlobals();
});

// The two gestures that replaced the Edit/Done button: double-click the read
// view to start editing, Ctrl+S to flush the write and go back to reading.
async function openEditor(): Promise<void> {
  await fireEvent.dblClick(await screen.findByTitle('Double-click to edit'));
}
async function saveWithCtrlS(): Promise<void> {
  await fireEvent.keyDown(window, { key: 's', ctrlKey: true });
}

describe('the app, driven end to end as a user', () => {
  it('loads the board with real cards from the backend', async () => {
    render(App);
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
    expect(await screen.findByText(/Reply to reviewer 2/)).toBeTruthy();
  });

  it('shows a name-coloured vault badge on cards, so every entity’s audience is visible', async () => {
    const { container } = render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    // The mock spans two vaults (personal + lab); board cards carry a VaultBadge for each.
    const badges = container.querySelectorAll('.vault-badge');
    expect(badges.length).toBeGreaterThan(0);
    expect([...badges].some((b) => /personal|lab/.test(b.textContent ?? ''))).toBe(true);
  });

  // Regression: the id counter resets each page load, so a workspace persisted by an earlier
  // session can carry two panes with the SAME id. A keyed {#each} rejects duplicate keys and the
  // error blanks the whole window — exactly the crash a reload from real localStorage hit but the
  // unit tests (which never reload from storage) missed. Mounting from a duplicate-id workspace
  // must render, not throw: loadWorkspace re-mints ids on the way in.
  it('recovers from a persisted workspace with duplicate pane ids instead of blanking', async () => {
    const dup = { ...newPane('board'), id: 'p1' };
    const dup2 = { ...newPane('agenda'), id: 'p1' }; // same id as dup — the corruption
    localStorage.setItem('fm-workspace', JSON.stringify({ cols: 2, panes: [dup, dup2] }));

    const { container } = render(App);
    // The board still renders its cards — the workspace was repaired, not crashed.
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
    // Both panes rendered (board + agenda): the duplicate id was de-duplicated, not dropped.
    expect(container.querySelectorAll('.pane')).toHaveLength(2);
  });

  it('walks board → capture → views → group-by → open → edit → close', async () => {
    const { container } = render(App);

    // 1. The board renders cards on load.
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();

    // 2. Create a new note; "New note" opens it straight in the editor. Type a
    //    body, leave edit mode (flushes the write), close — it lands on the board
    //    with its first line as the card preview.
    await fireEvent.click(screen.getByText('New note'));
    const body = await screen.findByLabelText('note body (Markdown)');
    await fireEvent.input(body, { target: { value: 'a freshly captured thought' } });
    await saveWithCtrlS();
    await fireEvent.click(screen.getByLabelText('close'));
    expect(await screen.findByText('a freshly captured thought')).toBeTruthy();

    // 3. Rotate this pane's view: a click advances Board → Agenda (due items); a wheel-scroll
    //    spins it back to Board. The picker is a rotator now, not a dropdown — but still the
    //    workspace model: each pane chooses what it shows.
    await fireEvent.click(screen.getByLabelText('pane view'));
    expect(await screen.findByText(/Reply to reviewer 2/)).toBeTruthy();
    await fireEvent.wheel(screen.getByLabelText('pane view'), { deltaY: -1 });
    await screen.findByText(/GAE lambda interacts badly/);

    // 4. Re-group the board pane by a custom property — no backend change.
    const groupBy = screen.getByLabelText('group by');
    await fireEvent.input(groupBy, { target: { value: 'project' } });
    expect(await screen.findByText('alpha')).toBeTruthy(); // a project column label

    // 5. Open a card → the read view renders the note's markdown, upgrades math +
    //    mermaid (mocked), and degrades the missing asset to a placeholder. This is
    //    render.ts's contract, now reached by a real click through the real panel.
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    expect(
      await screen.findByRole('heading', { name: /GAE and inner-loop adaptation/ }),
    ).toBeTruthy();
    // These upgrades run in sequence after the markdown is in the DOM, so wait for
    // each rather than assuming it fired the instant the heading appeared.
    await waitFor(() => expect(container.querySelector('.asset-missing-inline')).not.toBeNull());
    await waitFor(() => expect(katexRender).toHaveBeenCalled());
    await waitFor(() => expect(mermaidRender).toHaveBeenCalled());

    // 6. Double-click the read view to edit the body → Ctrl+S (which flushes the
    //    save and leaves edit mode) → the read view re-renders the new content.
    await openEditor();
    const editor = screen.getByLabelText(/note body/);
    await fireEvent.input(editor, {
      target: { value: '# Edited in the window\n\nbrand new prose.' },
    });
    await saveWithCtrlS();
    expect(await screen.findByRole('heading', { name: 'Edited in the window' })).toBeTruthy();

    // 7. Close the panel; the read view goes away.
    await fireEvent.click(screen.getByRole('button', { name: 'close' }));
    await waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'Edited in the window' })).toBeNull(),
    );
  });

  // The mock backend keeps its body overrides in module state, which outlives a
  // single test — so these open the Muesli note rather than the GAE one the edit
  // walk above rewrites. Every plain mock note shares SAMPLE_BODY, and that body
  // carries the note reference under test.
  it('follows a note reference into a second pane, keeping the first', async () => {
    const { container } = render(App);
    const panes = () => container.querySelectorAll('.panel');

    // Open a note whose body references another note.
    await fireEvent.click(await screen.findByText(/Read the Muesli paper/));
    await screen.findByRole('heading', { name: /GAE and inner-loop adaptation/ });

    // The reference resolved to a chip carrying the target's live status.
    const chip = await waitFor(() => {
      const c = container.querySelector<HTMLElement>('.note-chip');
      expect(c).not.toBeNull();
      return c!;
    });
    // The chip shows the *target's* live status, not this note's.
    expect(chip.textContent).toContain('the clipping ablation');
    expect(chip.querySelector('.note-chip-status')?.textContent).toBe('todo');
    expect(panes()).toHaveLength(1);

    // Following it opens a second pane, and the note we came from stays put.
    // Every mock note shares SAMPLE_BODY, so "both panes rendered" reads as the
    // heading appearing twice — one per pane.
    await fireEvent.click(chip);
    await waitFor(() => expect(panes()).toHaveLength(2));
    await waitFor(() =>
      expect(screen.getAllByRole('heading', { name: /GAE and inner-loop adaptation/ })).toHaveLength(
        2,
      ),
    );

    // Closing the second pane truncates the trail back to the first.
    const closes = screen.getAllByRole('button', { name: 'close' });
    await fireEvent.click(closes[closes.length - 1]);
    await waitFor(() => expect(panes()).toHaveLength(1));
    expect(screen.getAllByRole('heading', { name: /GAE and inner-loop adaptation/ })).toHaveLength(
      1,
    );
  });

  it('re-following an already-open note truncates rather than duplicating it', async () => {
    const { container } = render(App);
    const panes = () => container.querySelectorAll('.panel');

    await fireEvent.click(await screen.findByText(/Read the Muesli paper/));
    const chip = await waitFor(() => {
      const c = container.querySelector<HTMLElement>('.note-chip');
      expect(c).not.toBeNull();
      return c!;
    });
    await fireEvent.click(chip);
    await waitFor(() => expect(panes()).toHaveLength(2));

    // The second pane shows the same SAMPLE_BODY, so it carries a chip pointing at
    // itself — the note already at the end of the trail. Following that must not
    // open a third pane: two panes over one file would be two editors over it.
    const chips = await waitFor(() => {
      const found = container.querySelectorAll<HTMLElement>('.note-chip');
      expect(found).toHaveLength(2); // one per pane, each resolved independently
      return found;
    });
    await fireEvent.click(chips[1]);
    await waitFor(() => expect(panes()).toHaveLength(2));
  });
});
