// **The conflict with nothing to open, and the notes nobody knew were missing.**
//
// Both are from one real incident (2026-07-31). A note deleted on the laptop and edited on the phone
// is a delete/modify conflict: git writes no markers, because one side has no file. The app listed
// conflicts by scanning bodies for `<<<<<<<`, told the user in every message to "open each one, both
// versions are marked in the text", and offered no other action — so the only resolutions that exist
// for that kind (keep theirs / keep mine) were unreachable. Meanwhile `commit_all` refuses while a
// vault is mid-merge, so that one unresolvable note froze every commit in the vault for a week, and
// 95 notes accumulated on disk that git had never seen.
//
// What is pinned here is that both are now *actionable from the UI*, because the owner works only
// through the UI: a fix that needs a terminal is not a fix.

import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults, setConflicts, setUnopenedVaults, setUnrecorded } from './lib/mock';
import type { ConflictInfo } from './lib/types';

const DELETE_MODIFY: ConflictInfo = {
  note: {
    id: 'M0CK000000000000000000CONF',
    type: 'note',
    title: 'Meeting',
    preview: 'a note deleted here and edited elsewhere',
    status: null,
    due: null,
    start: null,
    hard: false,
    created: '2026-07-21T07:52:36Z',
    updated: '2026-07-31T01:57:09Z',
    tags: ['template'],
    assets: [],
    props: {},
    vault: 'personal',
  },
  path: 'notes/M0CK000000000000000000CONF.md',
  vault: 'personal',
  code: 'DU',
  what: 'Deleted here, edited on the other device. There is nothing to merge — pick a side.',
  has_markers: false,
};

// jsdom here has no `localStorage` (which is why every read of it in the app is guarded), so the
// workspace seed below needs one. A Map is enough and keeps the test honest about what it relies on.
beforeEach(() => {
  const store = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
      clear: () => store.clear(),
      key: () => null,
      length: 0,
    },
  });
  clearFaults();
});
afterEach(() => clearFaults());

/// Open the app with a Collaboration pane already up.
///
/// Seeded through the persisted workspace rather than by clicking the view rotator five times: the
/// rotator is a ring, so "click until it says Collaboration" is a test that breaks the day someone
/// adds a view. The persistence format is the app's own (`fm-workspace`).
async function openCollaboration() {
  localStorage.setItem(
    'fm-workspace',
    JSON.stringify({
      cols: 1,
      layout: 'single',
      active: 0,
      panes: [{ id: 1, kind: 'collaboration', colSpan: 1, rowSpan: 1 }],
    }),
  );
  render(App);
  await vi.waitFor(() => expect(document.body.querySelector('header.topbar')).toBeTruthy());
}

describe('a conflict with no markers', () => {
  it('says what happened and offers both sides, instead of telling you to edit text that does not exist', async () => {
    setConflicts([DELETE_MODIFY]);
    await openCollaboration();

    // The explanation, in plain words — this is what the user could not get anywhere before.
    expect(await screen.findByText(/nothing to merge/i)).toBeTruthy();
    // And the two resolutions that actually exist for this kind.
    expect(screen.getByRole('button', { name: /Keep the other device's version/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Keep this device's version/i })).toBeTruthy();
    // The old advice must not appear for a conflict that has no markers.
    expect(document.body.textContent).not.toMatch(/marked in the text/i);
  });

  it('resolves when a side is chosen, and the row goes away', async () => {
    setConflicts([DELETE_MODIFY]);
    await openCollaboration();

    await fireEvent.click(
      await screen.findByRole('button', { name: /Keep the other device's version/i }),
    );

    // The mock drops a resolved path, exactly as the server stops reporting it as unmerged — so an
    // emptying list is the real signal, not merely that a call was made.
    await vi.waitFor(() =>
      expect(
        screen.queryByRole('button', { name: /Keep the other device's version/i }),
      ).toBeNull(),
    );
  });

  it('offers "Mark resolved" for a marker conflict — editing alone settles nothing in git', async () => {
    setConflicts([
      { ...DELETE_MODIFY, code: 'UU', has_markers: true, what: 'Both sides edited this note; both versions are marked in the text.' },
    ]);
    await openCollaboration();

    // The action the product never had. Without it, a user who tidied the markers out of the note
    // left the vault permanently mid-merge, with nothing on screen saying so.
    await fireEvent.click(await screen.findByRole('button', { name: /Mark resolved/i }));
    await vi.waitFor(() =>
      expect(screen.queryByRole('button', { name: /Mark resolved/i })).toBeNull(),
    );
    expect(document.body.textContent).toMatch(/merge is finished/i);
  });

  it('still points a marker conflict at the note, where the markers are', async () => {
    setConflicts([
      { ...DELETE_MODIFY, code: 'UU', has_markers: true, what: 'Both sides edited this note; both versions are marked in the text.' },
    ]);
    await openCollaboration();

    expect(await screen.findByText(/marked in the text/i)).toBeTruthy();
    // No side-picking for a conflict a human can resolve properly by editing: choosing a side there
    // would silently discard the other one's edit. The way through is the note plus "Mark resolved".
    expect(screen.queryByRole('button', { name: /Keep the other device's version/i })).toBeNull();
    expect(screen.getByRole('button', { name: /Open the note/i })).toBeTruthy();
  });
});

describe('notes that are not in history', () => {
  it('are counted where you cannot miss them, and recorded in one click', async () => {
    setUnrecorded([{ vault: 'personal', count: 95, sample: ['M0CK000000000000000000AAAA'] }]);
    render(App);

    // A chip in the toolbar, not a dismissible banner: the condition persists until someone acts,
    // and its entire failure mode was being silent.
    const chip = await screen.findByRole('button', { name: /95 not in history/i });
    await fireEvent.click(chip);

    await vi.waitFor(() => expect(screen.queryByRole('button', { name: /not in history/i })).toBeNull());
    expect(document.body.textContent).toMatch(/Recorded 95 notes/i);
  });

  it('reports the total across vaults, not whichever vault came last', async () => {
    // Reported 2026-07-31: the chip said 146, the click committed them, and the message read
    // "Nothing left to record in 'notes'" — because `notice` was set inside the loop, so the second
    // vault (which had nothing) overwrote the first vault's result. The user was told their click did
    // nothing while 146 notes had just been committed.
    setUnrecorded([
      { vault: 'personal', count: 146, sample: [] },
      { vault: 'notes', count: 0, sample: [] },
    ]);
    render(App);
    await fireEvent.click(await screen.findByRole('button', { name: /146 not in history/i }));
    await vi.waitFor(() => expect(document.body.textContent).toMatch(/Recorded 146 notes/i));
    expect(document.body.textContent).not.toMatch(/Nothing left to record/i);
    // And it says what to do next: a commit is not a backup.
    expect(document.body.textContent).toMatch(/back up to send them to a remote/i);
  });

  it('shows no chip when git has everything', async () => {
    render(App);
    await vi.waitFor(() => expect(document.body.querySelector('header.topbar')).toBeTruthy());
    expect(screen.queryByRole('button', { name: /not in history/i })).toBeNull();
  });
});

describe('a vault that could not be opened', () => {
  it('is named on screen, because a silently absent vault reads as lost notes', async () => {
    setUnopenedVaults(['lab: io error: not a directory']);
    render(App);
    // The app must still work — the whole point is that one bad vault no longer refuses the rest —
    // and it must say which one is missing and why.
    await vi.waitFor(() => expect(document.body.querySelector('header.topbar')).toBeTruthy());
    await vi.waitFor(() => expect(document.body.textContent).toMatch(/could not be opened/i));
    expect(document.body.textContent).toMatch(/lab: io error: not a directory/);
    expect(document.body.textContent).toMatch(/Everything else is unaffected/i);
  });
});
