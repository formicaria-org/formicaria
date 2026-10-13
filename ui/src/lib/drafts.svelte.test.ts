// A form's unsaved work is kept on the device and is there when the form is opened again
// (`drafts.ts`; `decisions.md#ui`, 2026-10-14).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { draftOf, dropDraft, keepDraft } from './drafts';
import RoutineSheet from './training/RoutineSheet.svelte';
import WorkoutSheet from './training/WorkoutSheet.svelte';
import LogSheet from './food/LogSheet.svelte';
import { handle, reset, seedTrackerDemo } from './mock';
import { today } from './units';
import type { LogDays, ObjectMeta, Routine, TotalsDef, ViewResult } from './types';

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
    // A set that exists has a draft of its own: looking at it leaves none…
    const second = render(WorkoutSheet, { ...props, set: aSet });
    expect(screen.queryByText(/Carried on from where you left it/)).toBeNull();
    expect(draftOf('workout:training:s1')).toBeNull();
    expect(draftOf<{ name: string }>('workout:training')?.name).toBe('Side plank');
    // …and changing it and leaving keeps the change, for that set alone.
    const reps = screen.getAllByRole('spinbutton')[0] as HTMLInputElement;
    expect(reps.value).toBe('5');
    await fireEvent.input(reps, { target: { value: '8' } });
    await waitFor(() => expect(draftOf<{ reps: number }>('workout:training:s1')?.reps).toBe(8));
    second.unmount();
    render(WorkoutSheet, { ...props, set: aSet });
    expect((screen.getAllByRole('spinbutton')[0] as HTMLInputElement).value).toBe('8');
    await fireEvent.click(screen.getByRole('button', { name: 'Start afresh' }));
    await waitFor(() =>
      expect((screen.getAllByRole('spinbutton')[0] as HTMLInputElement).value).toBe('5'),
    );
    await waitFor(() => expect(draftOf('workout:training:s1')).toBeNull());
  });

  it('a routine being changed keeps the change, and one only looked at keeps nothing', async () => {
    const routine = {
      id: 'r1',
      title: 'Leg day',
      weekdays: ['mon'],
      sets: [],
    } as unknown as Routine;
    const props = {
      def,
      routine,
      sets: [],
      routines: [routine],
      onclose: vi.fn(),
      onwrote: vi.fn(),
    };
    const first = render(RoutineSheet, props);
    await waitFor(() => expect(draftOf('routine:r1')).toBeNull());
    await fireEvent.click(screen.getByRole('button', { name: 'Thu' }));
    await waitFor(() =>
      expect(draftOf<{ days: string[] }>('routine:r1')?.days).toEqual(['mon', 'thu']),
    );
    first.unmount();
    render(RoutineSheet, props);
    expect(screen.getByRole('button', { name: 'Thu' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText(/Carried on from where you left it/)).toBeTruthy();
    // Pressed back to what it was, it is no longer a change.
    await fireEvent.click(screen.getByRole('button', { name: 'Thu' }));
    await waitFor(() => expect(draftOf('routine:r1')).toBeNull());
  });

  it('a logged entry being changed keeps its new amount until it is saved', async () => {
    seedTrackerDemo();
    const food = (await handle<ViewResult>('run_view', { name: 'Day' })).totals!;
    const days = await handle<LogDays>('log_days', {
      view: 'Day',
      from: today(),
      to: today(),
      rows: true,
    });
    const entry = (days.days[0].rows ?? []).find((r) => r.props.amount_g !== undefined)!;
    const props = {
      view: 'Day',
      def: food,
      day: today(),
      slot: '',
      entry,
      onclose: vi.fn(),
      onwrote: vi.fn(),
    };
    const amount = async () =>
      (await screen.findByLabelText('amount in grams')) as HTMLInputElement;
    const first = render(LogSheet, props);
    const was = (await amount()).value;
    expect(screen.queryByText(/Carried on from where you left it/)).toBeNull();
    await fireEvent.input(await amount(), { target: { value: '77' } });
    await waitFor(() => expect(draftOf<{ grams: number }>(`entry:${entry.id}`)?.grams).toBe(77));
    first.unmount();
    render(LogSheet, props);
    expect(await screen.findByText(/Carried on from where you left it/)).toBeTruthy();
    expect((await amount()).value).toBe('77');
    await fireEvent.click(screen.getByRole('button', { name: 'Start afresh' }));
    await waitFor(async () => expect((await amount()).value).toBe(was));
    await waitFor(() => expect(draftOf(`entry:${entry.id}`)).toBeNull());
  });
});
