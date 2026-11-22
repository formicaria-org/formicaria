// **What the computer's pages did not do, until 2026-11-22** (found by the shell's parity list): a
// conflict's buttons were wired to nothing, and the program's reminder list was never handed over.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';
import type { ConflictInfo } from '../lib/types';

const { handed } = vi.hoisted(() => ({ handed: vi.fn() }));
vi.mock('../lib/ipc', async (importOriginal) => {
  const real = await importOriginal<typeof import('../lib/ipc')>();
  return {
    ...real,
    remindSet: (items: unknown[]) => {
      handed(items);
      return real.remindSet(items as never);
    },
  };
});

const CONFLICT: ConflictInfo = {
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
    tags: [],
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

const store = new Map<string, string>();
beforeEach(() => {
  mock.reset();
  mock.clearFaults();
  store.clear();
  handed.mockClear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
});
afterEach(() => {
  mock.clearFaults();
  vi.unstubAllGlobals();
  history.replaceState(null, '', '/');
});

it('a conflict’s “Keep the other device’s version” resolves it on the computer', async () => {
  mock.setConflicts([CONFLICT]);
  history.replaceState(null, '', '/f/collaboration');
  render(Feature);
  await fireEvent.click(
    await screen.findByRole('button', { name: /Keep the other device's version/i }),
  );
  await vi.waitFor(() =>
    expect(screen.queryByRole('button', { name: /Keep the other device's version/i })).toBeNull(),
  );
});

it('the page hands the program its reminders, so it can say them with no page open', async () => {
  store.set('fm-reminders', '1');
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  await vi.waitFor(() => expect(handed).toHaveBeenCalled(), { timeout: 4000 });
});
