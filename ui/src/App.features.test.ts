import { render, screen, fireEvent } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

// Covers the v2 additions on top of App.flow.test.ts: creating a note with a
// chosen type, editing its properties from the note panel (round-tripped through
// the mock backend), and the day-grouped Timeline view. Same hermetic setup as
// the flow test — jsdom + the in-memory mock, with the heavy lazy upgrades stubbed.
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

describe('v2: create-with-type, property editing, timeline', () => {
  it('captures a note as a meeting and the panel shows the meeting label', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Choose the type in the capture bar, then capture.
    await fireEvent.change(screen.getByLabelText('new note type'), { target: { value: 'meeting' } });
    const capture = screen.getByPlaceholderText(/Capture a note/);
    await fireEvent.input(capture, { target: { value: 'sync with the lab' } });
    await fireEvent.submit(capture.closest('form')!);

    // Open it and enter edit mode; the Type field reflects the meeting label
    // (create → set_property type → get all round-tripped through the mock).
    await fireEvent.click(await screen.findByText('sync with the lab'));
    await fireEvent.click(await screen.findByText('Edit'));
    const typeField = (await screen.findByLabelText('type')) as HTMLSelectElement;
    expect(typeField.value).toBe('meeting');
  });

  it('edits a property in the panel and it persists on reopen', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Open the GAE note, edit, change its type to task (an immediate write).
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await fireEvent.click(await screen.findByText('Edit'));
    await fireEvent.change(await screen.findByLabelText('type'), { target: { value: 'task' } });

    // Close the panel, then reopen the same note — the change survived the round trip.
    await fireEvent.click(screen.getByLabelText('close note'));
    await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
    await fireEvent.click(await screen.findByText('Edit'));
    const reopened = (await screen.findByLabelText('type')) as HTMLSelectElement;
    expect(reopened.value).toBe('task');
  });

  it('shows notes grouped by day in the Timeline view', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    await fireEvent.click(screen.getByText('Timeline'));
    // The seeded mock notes are created "now", so they land under Today.
    expect(await screen.findByText('Today')).toBeTruthy();
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
  });
});
