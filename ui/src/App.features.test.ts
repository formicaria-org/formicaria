import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

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
  // form plus the body textarea. Double-click is the shortcut; the Edit action in
  // the options window is the discoverable route and stays on every note.
  it('opens the editor — with its property form — from the Edit action', async () => {
    render(App);
    await screen.findByText(/Muesli/);
    await fireEvent.click(screen.getByText(/Muesli/));

    await fireEvent.click(await screen.findByLabelText('note options'));
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
    await fireEvent.click(await screen.findByLabelText('note options'));
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
    await fireEvent.click(await screen.findByLabelText('note options'));
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
    await fireEvent.click(await screen.findByLabelText('note options'));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy to…' }));
    await fireEvent.click(screen.getByTitle('Copy into lab'));
    expect(await screen.findByText(/Copied to lab/)).toBeTruthy();

    // Copying the same note again is warned as a replace, not a duplicate.
    await fireEvent.click(screen.getByLabelText('note options'));
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
    await fireEvent.click(await screen.findByLabelText('note options'));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await screen.findByText(/permanently/i);
    expect(screen.queryByLabelText('close')).not.toBeNull();

    // Clicking the menu item closed the menu, so the only remaining "Delete" is the
    // confirm strip's — this targets the second, final click.
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(screen.queryByLabelText('close')).toBeNull());
  });

  // Leaving the editor is an intuitive gesture, not a hunt for a "Done" button: clicking the
  // header chrome (the note's title / identity line — anything but a control) flushes the draft
  // and drops back to the read view, the twin of Ctrl+S / Escape. Runs on a throwaway note so it
  // neither depends on nor perturbs the shared seed.
  it('clicking the header while editing returns to the read view', async () => {
    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    await runCommand('New note', 'create');
    await screen.findByLabelText('note body (Markdown)'); // opens straight in edit mode

    // The title in the header is not a control, so a click on it means "done".
    await fireEvent.click(screen.getByRole('heading', { level: 2 }));
    await waitFor(() =>
      expect(screen.queryByLabelText('note body (Markdown)')).toBeNull(),
    );
  });

  // Templates must be *discoverable*: a note tagged `template` shows up in the ＋ "make something
  // new" menu as "New from …", and picking it opens a fresh, pre-filled editor. Seeded through the
  // mock (the debounced tags field is covered by the property tests) so this isolates the App
  // wiring a mock-only test misses — that the ＋ menu actually surfaces the template and runs it.
  it('a note tagged `template` appears in the ＋ menu and creates a note when picked', async () => {
    const tpl = await mock.handle<{ id: string }>('capture', { body: '# Weekly review' });
    await mock.handle('set_property', { id: tpl.id, key: 'tags', value: 'template' });

    render(App);
    await screen.findByText(/GAE lambda interacts badly/);

    // Open the ＋ menu — the template is offered as "New from …".
    await fireEvent.click(screen.getByRole('button', { name: 'make something new' }));
    const fromTemplate = await screen.findByRole('menuitem', { name: /New from/ });

    // Picking it opens a fresh editor (a new note spun off the template's body).
    await fireEvent.click(fromTemplate);
    expect(await screen.findByLabelText('note body (Markdown)')).toBeTruthy();
  });

  // The `/` menu inserts a chip link on Enter and an inline embed on Shift+Enter — the modifier
  // is the only way to reach an embed from the picker, so it's the wiring worth proving.
  it('the / menu inserts an embed (not a link) on Shift+Enter', async () => {
    render(App);
    // Wait on a unique control, not note text: earlier tests may have left a note open whose body
    // repeats a card's text, which would make a text match ambiguous.
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;

    // Open the / menu: "/reviewer" with the caret at the end so the token is detected. The seeded
    // "Reply to reviewer 2" note is the FTS hit.
    body.value = '/reviewer';
    body.selectionStart = body.selectionEnd = body.value.length;
    await fireEvent.input(body, { target: { value: '/reviewer' } });
    await screen.findByRole('option', { name: /reviewer/i });

    // Shift+Enter → the embed form (image syntax `![…](note:…)`), not a chip link `[…](note:…)`.
    await fireEvent.keyDown(body, { key: 'Enter', shiftKey: true });
    await waitFor(() =>
      expect(body.value).toMatch(/^!\[[^\]]*\]\(note:[0-9A-HJKMNP-TV-Z]{26}\)/),
    );
  });

  // `//` opens the menu in embed mode, so a plain Enter (a tap on a phone — no Shift) embeds.
  it('the // menu embeds on a plain Enter (no Shift needed)', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;

    // "//reviewer" — two slashes → embed mode.
    body.value = '//reviewer';
    body.selectionStart = body.selectionEnd = body.value.length;
    await fireEvent.input(body, { target: { value: '//reviewer' } });
    await screen.findByRole('option', { name: /reviewer/i });

    // Plain Enter, no Shift — because the menu is already in embed mode, it inserts the embed and
    // consumes both slashes (no leading `/` left behind).
    await fireEvent.keyDown(body, { key: 'Enter' });
    await waitFor(() =>
      expect(body.value).toMatch(/^!\[[^\]]*\]\(note:[0-9A-HJKMNP-TV-Z]{26}\)/),
    );
  });

  // Tapping a rendered checkbox flips only its `[ ]`↔`[x]` byte in the source — a surgical patch,
  // byte-for-byte everywhere else, exactly what a Vim user toggling that box would produce.
  it('tapping a checkbox in the read view toggles [ ]↔[x] in the source', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });

    // A note with two tasks: first unchecked, second done. Author it in the editor, then leave.
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    const SRC = '# chores\n\n- [ ] milk\n- [x] eggs\n';
    await fireEvent.input(body, { target: { value: SRC } });
    await fireEvent.keyDown(body, { key: 's', ctrlKey: true }); // save + leave edit → read view

    // The read view renders real checkboxes; the first is unchecked.
    const boxes = await waitFor(() => {
      const found = document.querySelectorAll<HTMLInputElement>('.read input[type="checkbox"]');
      if (found.length < 2) throw new Error('checkboxes not rendered yet');
      return found;
    });
    expect(boxes[0].checked).toBe(false);

    // Tap the first checkbox → its source `[ ]` becomes `[x]`; the second task is untouched.
    await fireEvent.click(boxes[0]);
    await openEditor(); // reopen the editor to read the saved source back
    await waitFor(() =>
      expect((screen.getByLabelText('note body (Markdown)') as HTMLTextAreaElement).value).toBe(
        '# chores\n\n- [x] milk\n- [x] eggs\n',
      ),
    );
  });

  // Tapping a callout's type badge opens a picker of the closed callout vocabulary; choosing one
  // rewrites just that `[!type]` token in the source — the callout analog of the checkbox toggle.
  it('tapping a callout type badge picks a new type and rewrites [!type] in the source', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });

    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    await fireEvent.input(body, { target: { value: '> [!note] heads up\n> body\n' } });
    await fireEvent.keyDown(body, { key: 's', ctrlKey: true }); // save + leave to read view

    // The rendered callout carries a "note" type badge; tap it → the picker of types appears.
    const badge = await waitFor(() => {
      const b = document.querySelector<HTMLElement>('.read .callout-kind');
      if (!b) throw new Error('callout not rendered yet');
      return b;
    });
    expect(badge.textContent).toBe('note');
    await fireEvent.click(badge);

    // Choose "warning" → the source `[!note]` becomes `[!warning]`.
    const warning = await screen.findByRole('button', { name: 'warning' });
    await fireEvent.click(warning);
    await openEditor();
    await waitFor(() =>
      expect((screen.getByLabelText('note body (Markdown)') as HTMLTextAreaElement).value).toBe(
        '> [!warning] heads up\n> body\n',
      ),
    );
  });

  // Selecting text in the editor floats a toolbar that wraps the selection in the right syntax —
  // byte-for-byte source, no typed markup. Bold, then a colour token from the closed set.
  it('the selection toolbar wraps the selection (bold, then a colour token)', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    await fireEvent.input(body, { target: { value: 'make this pop' } });

    // Select "pop" (chars 10–13) and raise the toolbar.
    body.selectionStart = 10;
    body.selectionEnd = 13;
    await fireEvent.select(body);
    await fireEvent.click(await screen.findByRole('button', { name: 'bold' }));
    await waitFor(() => expect(body.value).toBe('make this **pop**'));

    // The wrap re-selects "pop"; now colour it. Open the colour menu and pick "ok".
    await fireEvent.click(screen.getByRole('button', { name: 'colour' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'ok' }));
    await waitFor(() => expect(body.value).toBe('make this **[pop]{.ok}**'));
  });

  // The block submenu applies line-level formats: heading, lists, quote, callout. Whole-line, so a
  // selection anywhere in the line counts; the callout drops in a `[!note]` the read-view badge can retype.
  it('the block menu applies heading and callout formats to the line', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    await fireEvent.input(body, { target: { value: 'a plain line' } });

    body.selectionStart = 2;
    body.selectionEnd = 7; // inside the line
    await fireEvent.select(body);

    // Heading 2 → prefixes the whole line.
    await fireEvent.click(await screen.findByRole('button', { name: 'block format' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Heading 2' }));
    await waitFor(() => expect(body.value).toBe('## a plain line'));

    // Now Callout → a `[!note]` blockquote (the heading marker is inside the quoted text).
    await fireEvent.click(screen.getByRole('button', { name: 'block format' }));
    await fireEvent.click(await screen.findByRole('button', { name: /Callout/ }));
    await waitFor(() => expect(body.value).toBe('> [!note] ## a plain line'));
  });

  // Tapping a table cell edits it in place, patching ONLY that cell's bytes — every other cell,
  // the pipes, and the alignment row are left exactly as they were.
  it('tapping a table cell edits it in place and patches only that cell', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    await fireEvent.input(body, { target: { value: '| A | B |\n|---|---|\n| 1 | 2 |\n' } });
    await fireEvent.keyDown(body, { key: 's', ctrlKey: true }); // read view

    // Tap the body cell showing "1".
    const cell = await waitFor(() => {
      const c = [...document.querySelectorAll('.read tbody td')].find(
        (td) => td.textContent?.trim() === '1',
      );
      if (!c) throw new Error('table not rendered yet');
      return c as HTMLTableCellElement;
    });
    await fireEvent.click(cell);

    // The overlay input is prefilled with the cell's SOURCE text; change it and commit with Enter.
    const input = (await screen.findByLabelText('edit cell')) as HTMLInputElement;
    expect(input.value).toBe('1');
    await fireEvent.input(input, { target: { value: '9' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    await openEditor();
    await waitFor(() =>
      expect((screen.getByLabelText('note body (Markdown)') as HTMLTextAreaElement).value).toBe(
        '| A | B |\n|---|---|\n| 9 | 2 |\n',
      ),
    );
  });

  // A pipe typed into a cell would break the table — so it's escaped, keeping the file a valid table.
  it('escapes a pipe typed into a table cell so the table cannot break', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    await runCommand('New note', 'create');
    const body = (await screen.findByLabelText('note body (Markdown)')) as HTMLTextAreaElement;
    await fireEvent.input(body, { target: { value: '| A |\n|---|\n| x |\n' } });
    await fireEvent.keyDown(body, { key: 's', ctrlKey: true });

    const cell = await waitFor(() => {
      const c = [...document.querySelectorAll('.read tbody td')].find(
        (td) => td.textContent?.trim() === 'x',
      );
      if (!c) throw new Error('table not rendered yet');
      return c as HTMLTableCellElement;
    });
    await fireEvent.click(cell);
    const input = (await screen.findByLabelText('edit cell')) as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'a|b' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    await openEditor();
    await waitFor(() =>
      expect((screen.getByLabelText('note body (Markdown)') as HTMLTextAreaElement).value).toBe(
        '| A |\n|---|\n| a\\|b |\n',
      ),
    );
  });
});
