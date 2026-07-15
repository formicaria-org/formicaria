import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

// Layer-2 end-to-end: mount the REAL app and drive it the way a user does —
// clicking through the board, switching views, re-grouping, opening a card,
// editing it, closing it. No Tauri window: `ipc.ts` sees no `__TAURI_INTERNALS__`
// in jsdom and falls back to the in-memory mock backend (`mock.ts`), whose JSON
// shape Layer 1 pins against the real Rust DTOs. The only things stubbed are the
// two heavy lazy upgrades (KaTeX, Mermaid), exactly as in render.test.ts, so the
// read view renders hermetically.
const { mermaidInit, mermaidRender, katexAutoRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  katexAutoRender: vi.fn(),
}));
vi.mock('mermaid', () => ({ default: { initialize: mermaidInit, render: mermaidRender } }));
vi.mock('katex/dist/contrib/auto-render.js', () => ({ default: katexAutoRender }));
vi.mock('katex/dist/katex.min.css', () => ({}));

beforeEach(() => {
  mermaidRender.mockResolvedValue({ svg: '<svg data-mock-mermaid="1"></svg>' });
});

describe('the app, driven end to end as a user', () => {
  it('loads the board with real cards from the backend', async () => {
    render(App);
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
    expect(await screen.findByText(/Reply to reviewer 2/)).toBeTruthy();
  });

  it('walks board → capture → views → group-by → open → edit → close', async () => {
    const { container } = render(App);

    // 1. The board renders cards on load.
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();

    // 2. Capture a new note; it lands on the board.
    const capture = screen.getByPlaceholderText(/Capture a note/);
    await fireEvent.input(capture, { target: { value: 'a freshly captured thought' } });
    await fireEvent.submit(capture.closest('form')!);
    expect(await screen.findByText('a freshly captured thought')).toBeTruthy();

    // 3. Switch views: Agenda (due items), back to Board.
    await fireEvent.click(screen.getByRole('button', { name: 'Agenda' }));
    expect(await screen.findByText(/Reply to reviewer 2/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Board' }));
    await screen.findByText(/GAE lambda interacts badly/);

    // 4. Re-group the board by a custom property — no backend change.
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
    await waitFor(() => expect(katexAutoRender).toHaveBeenCalled());
    await waitFor(() => expect(mermaidRender).toHaveBeenCalled());

    // 6. Edit the body → leave edit mode (which flushes the save) → the read view
    //    re-renders the new content.
    await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
    const editor = screen.getByLabelText(/note body/);
    await fireEvent.input(editor, {
      target: { value: '# Edited in the window\n\nbrand new prose.' },
    });
    await fireEvent.click(screen.getByRole('button', { name: /Done|Saving/ }));
    expect(await screen.findByRole('heading', { name: 'Edited in the window' })).toBeTruthy();

    // 7. Close the panel; the read view goes away.
    await fireEvent.click(screen.getByRole('button', { name: 'close' }));
    await waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'Edited in the window' })).toBeNull(),
    );
  });
});
