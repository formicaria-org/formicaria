// A form's unsaved work is kept on the device and is there when the form is opened again
// (`drafts.ts`; `decisions.md#ui`, 2026-10-14).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { draftOf, dropDraft, keepDraft } from './drafts';
import RoutineSheet from './training/RoutineSheet.svelte';
import WorkoutSheet from './training/WorkoutSheet.svelte';
import { reset } from './mock';
import type { ObjectMeta, TotalsDef } from './types';

const store = new Map<string, string>();
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
});
afterEach(() => {
  vi.unstubAllGlobals();
  reset();
});

const def = { log: 'training', metrics: [], slots: [] } as unknown as TotalsDef;
const aSet = {
  id: 's1',
  title: 'Squat',
  props: { exercise: 'Squat', reps: 5 },
} as unknown as ObjectMeta;

describe('what is typed and not yet saved', () => {
  it('is kept, read back and dropped by name, and a device that cannot store it still works', () => {
    expect(draftOf('x')).toBeNull();
    keepDraft('x', { a: 1 });
    expect(draftOf('x')).toEqual({ a: 1 });
    expect(draftOf('y')).toBeNull();
    dropDraft('x');
    expect(draftOf('x')).toBeNull();
    store.set('fm-draft:bad', '{not json');
    expect(draftOf('bad')).toBeNull();
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('refused');
      },
      setItem: () => {
        throw new Error('full');
      },
      removeItem: () => {
        throw new Error('refused');
      },
    });
    expect(() => keepDraft('x', 1)).not.toThrow();
    expect(() => dropDraft('x')).not.toThrow();
    expect(draftOf('x')).toBeNull();
  });

  it('a routine being named survives leaving the sheet, and starts afresh when asked', async () => {
    const props = {
      def,
      routine: null,
      sets: [aSet],
      routines: [],
      onclose: vi.fn(),
      onwrote: vi.fn(),
    };
    const first = render(RoutineSheet, props);
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Leg day' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Mon' }));
    await waitFor(() => expect(draftOf('routine')).toEqual({ name: 'Leg day', days: ['mon'] }));
    first.unmount();

    render(RoutineSheet, props);
    expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Leg day');
    expect(screen.getByRole('button', { name: 'Mon' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText(/Carried on from where you left it/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Start afresh' }));
    expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('');
    expect(screen.queryByText(/Carried on from where you left it/)).toBeNull();
  });

  it('an exercise half planned is there when the sheet is opened again', async () => {
    const props = {
      def,
      day: '2026-10-14',
      last: [],
      have: [],
      onclose: vi.fn(),
      onwrote: vi.fn(),
    };
    keepDraft('workout:training', {
      name: 'Plank',
      sets: 3,
      reps: 12,
      kg: 0,
      mode: 'sets',
      activity: '',
      countsAs: 'aerobic',
      minutes: 0,
      effort: 0,
      region: '',
      picked: true,
    });
    const first = render(WorkoutSheet, props);
    const name = (await screen.findByLabelText('Exercise')) as HTMLInputElement;
    expect(name.value).toBe('Plank');
    expect(screen.getByText(/Carried on from where you left it/)).toBeTruthy();
    // What is changed now is what is kept.
    await fireEvent.input(name, { target: { value: 'Side plank' } });
    await waitFor(() =>
      expect(draftOf<{ name: string }>('workout:training')?.name).toBe('Side plank'),
    );
    first.unmount();
    // Changing a set that exists never opens on the draft, and never overwrites it.
    render(WorkoutSheet, { ...props, set: aSet });
    expect(screen.queryByText(/Carried on from where you left it/)).toBeNull();
    expect(draftOf<{ name: string }>('workout:training')?.name).toBe('Side plank');
  });
});
