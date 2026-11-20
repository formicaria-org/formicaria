// **A watch's files on the phone, and workouts out** (`decisions.md` 2026-11-20, *The phone takes
// watch files* and the Coach's item 6). The shell is stood in for by fakes shaped as
// `MainActivity.kt`'s `__fmShare` and `__fmWatch`; the mock stands in for `import_fit_files` and
// `workout_fit`, which `crates/fm-app/tests/import_fit.rs` and `src/workout.rs` test for real.
import { render, screen, fireEvent, within, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, handle, reset } from './lib/mock';
import { today as todayNow } from './lib/units';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';
import { toast } from './lib/web/bridge.svelte';
import type { ObjectMeta } from './lib/types';

const FIT = btoa('\x0e\x10\x00\x00\x00\x00\x00\x00.FIT');
const store = new Map<string, string>();
let held: { name: string; mime: string }[] = [];
let sent: string[] = [];
let saved: string[] = [];

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
  nav.reset();
  store.set('fm-trackers', '1');
  trackers.reload();
  held = [];
  sent = [];
  saved = [];
  (globalThis as { __fmShare?: unknown }).__fmShare = {
    pending: () => JSON.stringify({ files: held, refused: [] }),
    read: () => FIT,
    done: () => void (held = []),
  };
  (globalThis as { __fmWatch?: unknown }).__fmWatch = {
    state: () =>
      JSON.stringify({
        folder: '',
        readable: true,
        looking: false,
        held: 0,
        waiting: 0,
        tooLarge: 0,
      }),
    choose: () => {},
    forget: () => {},
    look: () => {},
    pending: () => '[]',
    read: () => '',
    done: () => {},
    send: (name: string) => void sent.push(name),
    save: (name: string) => void saved.push(name),
  };
});
afterEach(() => {
  delete (globalThis as { __fmShare?: unknown }).__fmShare;
  delete (globalThis as { __fmWatch?: unknown }).__fmWatch;
  toast.now = null;
  vi.unstubAllGlobals();
  clearFaults();
  reset();
  clearUndo();
});

test('a shared .fit goes into Fitness, never into a note', async () => {
  render(App);
  await screen.findByRole('button', { name: 'make something new' });
  held = [{ name: 'run-1.fit', mime: 'application/vnd.ant.fit' }];
  window.dispatchEvent(new Event('fm-share'));
  await waitFor(() => expect(toast.now?.text).toMatch(/Added 1 activity from the watch/));
  // No sheet asking which note: a watch's file is not an attachment.
  expect(screen.queryByRole('dialog', { name: 'add what was shared to a note' })).toBeNull();
});

test('a planned session is saved as a .fit file and sent to the watch app, said plainly', async () => {
  const today = todayNow();
  const [program] = await handle<ObjectMeta[]>('create_notes', {
    notes: [
      {
        title: 'Endurance and strength in one week, assembled',
        start: today,
        props: { program: 'coach', plan: 'hybrid-week', question: 'goal', week: 1 },
      },
    ],
  });
  await handle('create_notes', {
    notes: [
      {
        title: 'Easy · Run',
        start: today,
        body: '- Work: Your sport, time and effort\n',
        props: {
          log: 'workout',
          planned: 1,
          activity: 'Running',
          sport: 'run',
          counts_as: 'aerobic',
          plan: program.id,
          plan_week: 1,
          session: 'endurance',
        },
      },
    ],
  });
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Coach' }));
  const todayCard = await screen.findByRole('region', { name: 'Today' });
  expect(within(todayCard).getByText(/Workouts list, not its calendar/)).toBeTruthy();
  expect(within(todayCard).getByText(/Not yet tried on every watch/)).toBeTruthy();
  await fireEvent.click(within(todayCard).getByRole('button', { name: 'Save .fit file' }));
  await waitFor(() => expect(saved).toEqual([`${today}-workout.fit`]));
  // No numbers yet: the steps are open, and that is said.
  expect(await within(todayCard).findByText(/lap key/)).toBeTruthy();
  await fireEvent.click(within(todayCard).getByRole('button', { name: 'Send to watch app' }));
  await waitFor(() => expect(sent).toEqual([`${today}-workout.fit`]));
});
