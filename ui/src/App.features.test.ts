import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

// Covers the v2 additions on top of App.flow.test.ts: editing a note's
// properties from the panel (round-tripped through the mock backend), the
// day-grouped Timeline view, and delete-with-confirm. Same hermetic setup as the
// flow test — jsdom + the in-memory mock, with the heavy lazy upgrades stubbed.
const { mermaidInit, mermaidRender, katexRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  katexRender: vi.fn((tex: string) => `<span class="katex">${tex}</span>`),
}));
vi.mock('mermaid', () => ({ default: { initialize: mermaidInit, render: mermaidRender } }));
vi.mock('katex', () => ({ default: { renderToString: katexRender } }));
vi.mock('katex/dist/katex.min.css', () => ({}));

beforeEach(() => {
  mermaidRender.mockResolvedValue({ svg: '<svg data-mock-mermaid="1"></svg>' });
});

describe('v2: property editing, timeline, delete', () => {
  it('edits a property in the panel and it persists on reopen', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Open the GAE note, edit, set a due date (an immediate write — the `due`
    // field is not debounced). Notes carry no user-settable type anymore; tags
    // differentiate them, so the round-trip is proven on a plain property.
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await fireEvent.click(await screen.findByText('Edit'));
    // A date input syncs `bind:value` on `input` and writes on `change`; a real
    // date-picker fires both, so simulate both (change alone would write '').
    const dueField = await screen.findByLabelText('due');
    await fireEvent.input(dueField, { target: { value: '2026-08-01' } });
    await fireEvent.change(dueField, { target: { value: '2026-08-01' } });

    // Close the panel, then reopen the same note — the change survived the round trip.
    await fireEvent.click(screen.getByLabelText('close note'));
    await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
    await fireEvent.click(await screen.findByText('Edit'));
    const reopened = (await screen.findByLabelText('due')) as HTMLInputElement;
    expect(reopened.value).toBe('2026-08-01');
  });

  it('shows notes grouped by day in the Timeline view', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    await fireEvent.click(screen.getByText('Timeline'));
    // The seeded mock notes are created "now", so they land under Today.
    expect(await screen.findByText('Today')).toBeTruthy();
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
  });

  it('deletes a note only after the second confirmation', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Create a throwaway note — "New note" opens it straight in the panel — so we
    // exercise delete without mutating the shared seed.
    await fireEvent.click(screen.getByText('New note'));
    await screen.findByLabelText('note body (Markdown)'); // panel open in edit mode

    // First click only arms the confirmation — the panel is still open.
    await fireEvent.click(await screen.findByLabelText('delete note'));
    await screen.findByText(/permanently/i);
    expect(screen.queryByLabelText('close note')).not.toBeNull();

    // The confirm button's accessible name is "Delete" (the header button uses
    // the aria-label "delete note"), so this targets the second, final click.
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(screen.queryByLabelText('close note')).toBeNull());
  });
});
