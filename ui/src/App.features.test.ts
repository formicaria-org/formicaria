import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

/** Reach a command the way a user now does.
 *
 *  Two surfaces, and the distinction is the point. The toolbar's red **plus** makes things — a
 *  note, a board, a window — and is deliberately three items long. Everything else, including
 *  opening a named view, lives in **Settings**, whose first section is the action list. There is
 *  no command palette any more: it was a second menu carrying preferences Settings also owned,
 *  so the two could disagree about one thing.
 *
 *  Two details this encodes, both of which broke it once:
 *  - The plus opens a real ARIA menu, so its entries are **`menuitem`**, not `button`.
 *  - There are two buttons named "settings" — the toolbar gear and the bottom `ViewBar`'s, which
 *    is where the control lives on a phone. Both open the same panel, so the duplication is
 *    deliberate; the query is scoped to the top bar to say which one it means. */
async function runCommand(label: string, via: 'create' | 'settings') {
  if (via === 'create') {
    await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
    await fireEvent.click(await screen.findByRole('menuitem', { name: label }));
    return;
  }
  const topbar = document.querySelector('header.topbar');
  if (!topbar) throw new Error('no top bar rendered');
  await fireEvent.click(within(topbar as HTMLElement).getByRole('button', { name: 'settings' }));
  await fireEvent.click(await screen.findByRole('button', { name: label }));
}

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

// A date/time input syncs `bind:value` on `input` and writes on `change`; a real
// picker fires both, so simulate both (change alone would write '').
async function setDate(el: HTMLElement, value: string): Promise<void> {
  await fireEvent.input(el, { target: { value } });
  await fireEvent.change(el, { target: { value } });
}

// There is no Edit button: you double-click the note to edit it.
async function openEditor(): Promise<void> {
  await fireEvent.dblClick(await screen.findByTitle('Double-click to edit'));
}

describe('v2: property editing, timeline, delete', () => {
  it('edits a property in the panel and it persists on reopen', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Open the GAE note, edit, set a due date (an immediate write — the `due`
    // field is not debounced). Notes carry no user-settable type anymore; tags
    // differentiate them, so the round-trip is proven on a plain property.
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await openEditor();
    // A date input syncs `bind:value` on `input` and writes on `change`; a real
    // date-picker fires both, so simulate both (change alone would write '').
    const dueField = await screen.findByLabelText('due');
    await fireEvent.input(dueField, { target: { value: '2026-08-01' } });
    await fireEvent.change(dueField, { target: { value: '2026-08-01' } });

    // Close the panel, then reopen the same note — the change survived the round trip.
    await fireEvent.click(screen.getByLabelText('close'));
    await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
    await openEditor();
    const reopened = (await screen.findByLabelText('due')) as HTMLInputElement;
    expect(reopened.value).toBe('2026-08-01');
  });

  it('sets a due time beside the date and reloads it into both inputs', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await openEditor();

    // Establish the starting state rather than assuming it — the mock backend is
    // module-level state that earlier tests in this file have already written to.
    const due = await screen.findByLabelText('due');
    await setDate(due, '');

    // The time input is inert until there's a date to hang it on — a time with
    // no day isn't a point on any calendar.
    const dueTime = (await screen.findByLabelText('due time')) as HTMLInputElement;
    await waitFor(() => expect(dueTime.disabled).toBe(true));

    await setDate(due, '2026-08-01');
    await waitFor(() =>
      expect((screen.getByLabelText('due time') as HTMLInputElement).disabled).toBe(false),
    );

    await setDate(dueTime, '14:30');

    // Round-trip: the two inputs recombine into one wire value, and split again.
    await fireEvent.click(screen.getByLabelText('close'));
    await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
    await openEditor();
    expect(((await screen.findByLabelText('due')) as HTMLInputElement).value).toBe('2026-08-01');
    expect(((await screen.findByLabelText('due time')) as HTMLInputElement).value).toBe('14:30');
  });

  it('clearing the date clears the whole stamp, not just the day', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await openEditor();

    const due = await screen.findByLabelText('due');
    await setDate(due, '2026-08-01');
    await setDate(await screen.findByLabelText('due time'), '14:30');
    await setDate(due, ''); // clearing the day must take the orphaned time with it

    await fireEvent.click(screen.getByLabelText('close'));
    await fireEvent.click(await screen.findByText(/GAE lambda interacts badly/));
    await openEditor();
    expect(((await screen.findByLabelText('due')) as HTMLInputElement).value).toBe('');
    expect(((await screen.findByLabelText('due time')) as HTMLInputElement).value).toBe('');
  });

  // Two ways into the editor, and both must reach the SAME thing: the property
  // form plus the body textarea. Double-click is the shortcut; the button is the
  // discoverable route and stays on every note.
  it('opens the editor — with its property form — from the Edit button', async () => {
    render(App);
    await screen.findByText(/Muesli/);
    await fireEvent.click(screen.getByText(/Muesli/));

    await fireEvent.click(await screen.findByRole('button', { name: 'Edit' }));
    expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();
    for (const field of ['status', 'due', 'start', 'title', 'tags']) {
      expect(await screen.findByLabelText(field)).toBeTruthy();
    }
  });

  it('opens the same editor by double-clicking the note', async () => {
    render(App);
    await screen.findByText(/Muesli/);
    await fireEvent.click(screen.getByText(/Muesli/));

    await openEditor();
    expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();
    expect(await screen.findByLabelText('status')).toBeTruthy();
    expect(await screen.findByLabelText('tags')).toBeTruthy();
  });

  it('shows notes grouped by day in a Timeline pane', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Rotate the default pane to Timeline via its view rotator — a click advances one step,
    // so Board → Agenda → Timeline is two clicks (the workspace model: panes, not one global view).
    await fireEvent.click(screen.getByLabelText('pane view'));
    await fireEvent.click(screen.getByLabelText('pane view'));
    // The seeded mock notes are created "now", so they land under Today.
    expect(await screen.findByText('Today')).toBeTruthy();
    expect(await screen.findByText(/GAE lambda interacts badly/)).toBeTruthy();
  });

  // **The header rotates the view, not just the small label inside it.** The rotator button
  // worked and nobody found it: one modest target among the header's controls, with nothing
  // saying "scroll me". These pin the two gestures that replaced hunting for it.
  it('changes a pane view by scrolling anywhere on its header', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();

    // One mouse notch — 120px is what a wheel reports for one detent.
    await fireEvent.wheel(head, { deltaY: 120, deltaX: 0 });
    expect(view()).toBe('Agenda');
    // And back the other way — a ring, so it spins in both directions. Reversing is deliberate
    // by definition, so it answers immediately rather than waiting out the cooldown.
    await fireEvent.wheel(head, { deltaY: -120, deltaX: 0 });
    expect(view()).toBe('Board');
  });

  // **The over-sensitivity that shipped**: a wheel emits a burst per gesture, and rotating on
  // each event span several views before any of them could be read. Reported from real use.
  it('spends one step per gesture, not one per wheel event', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();

    // A trackpad's stream: many small deltas well past a single notch in total.
    for (let i = 0; i < 12; i++) await fireEvent.wheel(head, { deltaY: 30, deltaX: 0 });
    expect(view()).toBe('Agenda');

    // Inertia keeps arriving after the fingers have gone. It must not bank further steps.
    for (let i = 0; i < 20; i++) await fireEvent.wheel(head, { deltaY: 18, deltaX: 0 });
    expect(view()).toBe('Agenda');
  });

  // **The one-directional bug.** Blocking a step during the cooldown used to *hold* the
  // accumulator at the threshold, which left the same direction pre-charged: continuing forward
  // turned on the very next event while turning back started from zero and cost a second notch.
  // Reported as "only one direction works".
  it('comes back to where it started when you scroll back the same amount', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();

    // Three detents forward is three views — a detent is a deliberate act and is never throttled,
    // which is the difference between a control that feels calm and one that feels ignored.
    for (let i = 0; i < 3; i++) await fireEvent.wheel(head, { deltaY: 100, deltaX: 0 });
    expect(view()).toBe('Search');

    // And three back is exactly back. The two shipped bugs both broke this: one made the return
    // trip cost twice as much, the other made it cost half.
    for (let i = 0; i < 3; i++) await fireEvent.wheel(head, { deltaY: -100, deltaX: 0 });
    expect(view()).toBe('Board');
  });

  // **Symmetry as a property, not an example.** Two direction-dependent bugs shipped from this
  // one control in an hour — a held accumulator that pre-charged the way you were already going,
  // then a cooldown that a reversal cleared and continuing did not. Both times a single example
  // passed while the control was lopsided in use. This drives an arbitrary sequence of detents
  // and requires the view to be exactly where the arithmetic says, which no direction-dependent
  // rule can satisfy.
  it('treats up and down identically over an arbitrary sequence of notches', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();
    // The rotator's ring, in order, as `Pane.svelte` builds it from KINDS.
    const ring = ['Board', 'Agenda', 'Timeline', 'Search', 'Activity'];
    const at = (i: number) => ring[((i % ring.length) + ring.length) % ring.length];

    // Deliberately lopsided: runs of one direction, single reversals, and a long run back.
    const moves = [1, 1, -1, 1, 1, 1, -1, -1, -1, -1, 1, -1, 1, 1, -1];
    let expected = 0;
    for (const dir of moves) {
      await fireEvent.wheel(head, { deltaY: 100 * dir, deltaX: 0 });
      expected += dir;
      expect(view()).toBe(at(expected));
    }
    // Every notch spent exactly one step, so returning to zero returns to where it started.
    expect(expected).toBe(moves.reduce((a, b) => a + b, 0));
  });

  it('turns the view once for a single mouse notch', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();

    // Chrome reports 100 for one detent on many mice; a threshold above that made one notch do
    // nothing and two do one step, which reads as an unresponsive control rather than a calm one.
    await fireEvent.wheel(head, { deltaY: 100, deltaX: 0 });
    expect(view()).toBe('Agenda');
  });

  it('measures a wheel in pixels whatever unit the browser reports', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const view = () => screen.getByLabelText('pane view').textContent?.trim();

    // Firefox reports lines (`deltaMode: 1`), and **three** lines is one notch. Compared raw
    // against a pixel threshold that is ~40x too small, and the control feels dead there while
    // working in Chrome.
    await fireEvent.wheel(head, { deltaY: 3, deltaX: 0, deltaMode: 1 });
    expect(view()).toBe('Agenda');
  });

  it('ignores a sideways trackpad flick over the header', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];

    // Horizontal scrolling is how a board is read. Turning that into a view change would trade
    // one discoverable action for a broken one.
    await fireEvent.wheel(head, { deltaY: 0, deltaX: 40 });
    expect(screen.getByLabelText('pane view').textContent?.trim()).toBe('Board');
  });

  it('changes a pane view by swiping its header, but not on a stray tap', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const head = screen.getAllByRole('toolbar')[0];
    const swipe = async (fromX: number, toX: number, toY = 0) => {
      await fireEvent.touchStart(head, { changedTouches: [{ clientX: fromX, clientY: 0 }] });
      await fireEvent.touchEnd(head, { changedTouches: [{ clientX: toX, clientY: toY }] });
    };

    // Left = forward, the way every carousel moves.
    await swipe(200, 100);
    expect(screen.getByLabelText('pane view').textContent?.trim()).toBe('Agenda');
    await swipe(100, 200);
    expect(screen.getByLabelText('pane view').textContent?.trim()).toBe('Board');

    // A tap that wandered a few pixels is not a swipe — the header is also the drag handle,
    // so a trigger-happy threshold would change the view every time a pane was picked up.
    await swipe(200, 180);
    expect(screen.getByLabelText('pane view').textContent?.trim()).toBe('Board');

    // Nor is a mostly-vertical drag that happened to start on the header.
    await swipe(200, 140, 300);
    expect(screen.getByLabelText('pane view').textContent?.trim()).toBe('Board');
  });

  it('copies a note to another vault behind a warning, then undoes it', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    // Open the GAE note (it lives in the default `personal` vault; the mock also has `lab`).
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));

    // The Copy-to control opens a popover that warns this is permanent in the target's history.
    await fireEvent.click(await screen.findByLabelText('more actions'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy to…' }));
    expect(await screen.findByText(/permanent in that vault/i)).toBeTruthy();
    // Restrictive by default: the "also copy the files" opt-in starts unchecked.
    expect((screen.getByLabelText('also copy the files') as HTMLInputElement).checked).toBe(false);

    // Pick the other vault (targeted by title — the name also appears as a filter chip).
    await fireEvent.click(screen.getByTitle('Copy into lab'));

    // The post-copy Undo strip appears; undoing it recedes the copy.
    expect(await screen.findByText(/Copied to lab/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    await waitFor(() => expect(screen.queryByText(/Copied to lab/)).toBeNull());
  });

  it('requires an extra warning-confirm to carry the files, and can back out', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));
    await fireEvent.click(await screen.findByLabelText('more actions'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy to…' }));

    // Tick "also copy the files" — the sharper path.
    await fireEvent.click(screen.getByLabelText('also copy the files'));
    // Now a target click arms a warning-confirm instead of copying immediately.
    await fireEvent.click(screen.getByTitle('Copy into lab'));
    expect(await screen.findByText(/permanent in its git history/i)).toBeTruthy();
    expect(screen.queryByText(/Copied to lab/)).toBeNull(); // nothing copied yet

    // Back out — still fully reversible before it runs.
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText(/Copied to lab/)).toBeNull();

    // Try again and confirm this time.
    await fireEvent.click(screen.getByTitle('Copy into lab'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy with files' }));
    expect(await screen.findByText(/Copied to lab/)).toBeTruthy();

    // Recede it, so this test leaves the shared mock state as it found it.
    await fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    await waitFor(() => expect(screen.queryByText(/Copied to lab/)).toBeNull());
  });

  it('warns that re-copying replaces the existing copy, and can undo', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    await fireEvent.click(screen.getByText(/GAE lambda interacts badly/));

    // First copy (prose-only, first time) goes straight through — nothing to replace.
    await fireEvent.click(await screen.findByLabelText('more actions'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy to…' }));
    await fireEvent.click(screen.getByTitle('Copy into lab'));
    expect(await screen.findByText(/Copied to lab/)).toBeTruthy();

    // Copying the same note again is warned as a replace, not a duplicate.
    await fireEvent.click(screen.getByLabelText('more actions'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy to…' }));
    await fireEvent.click(screen.getByTitle('Copy into lab'));
    expect(await screen.findByText(/copying again replaces it/i)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Replace copy' }));
    expect(await screen.findByText(/Replaced the copy in lab/)).toBeTruthy();

    // Undo, leaving the shared mock state clean.
    await fireEvent.click(screen.getByRole('button', { name: 'Undo' }));
    await waitFor(() => expect(screen.queryByText(/Replaced the copy in lab/)).toBeNull());
  });

  it('shows a git "edited by" label on cards', async () => {
    const { container } = render(App);
    await screen.findByText(/GAE lambda interacts badly/);
    const labels = container.querySelectorAll('.edited-by');
    expect(labels.length).toBeGreaterThan(0);
    // The mock attributes edits to two people; a label carries one of their names.
    expect([...labels].some((l) => /Ada|Ravi/.test(l.textContent ?? ''))).toBe(true);
  });

  it('opens a git activity pane and filters it by contributor', async () => {
    const { container } = render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Open the Activity stream from the top bar.
    await runCommand('Open Activity', 'settings');
    const before = await waitFor(() => {
      const rows = container.querySelectorAll('.activity .row');
      expect(rows.length).toBeGreaterThan(1);
      return rows.length;
    });

    // Hiding a contributor drops their rows from the stream (and from every other view).
    await fireEvent.click(screen.getByTitle(/Hide Ada Lovelace/));
    await waitFor(() => {
      expect(container.querySelectorAll('.activity .row').length).toBeLessThan(before);
    });
  });

  it('deletes a note only after the second confirmation', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Create a throwaway note — "New note" opens it straight in the panel — so we
    // exercise delete without mutating the shared seed.
    await runCommand('New note', 'create');
    await screen.findByLabelText('note body (Markdown)'); // panel open in edit mode

    // Delete lives in the ⋯ overflow now; opening it and clicking Delete only arms
    // the confirmation — the panel is still open.
    await fireEvent.click(await screen.findByLabelText('more actions'));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await screen.findByText(/permanently/i);
    expect(screen.queryByLabelText('close')).not.toBeNull();

    // Clicking the menu item closed the menu, so the only remaining "Delete" is the
    // confirm strip's — this targets the second, final click.
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(screen.queryByLabelText('close')).toBeNull());
  });
});
