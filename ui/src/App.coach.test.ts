// **Fitness → Coach** (`decisions.md` 2026-11-20, *Fitness gets a Coach tab; the app still chooses no
// loads*): a plan is followed, a week is put on the calendar with the person's own numbers, the plan
// moves on only when Next week is pressed, a done session keeps what was planned, and a watch's
// session is matched only when the person says yes. The mock stands in for `coach.rs`, which is
// tested in `crates/fm-app/tests/coach.rs`.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, handle, reset } from './lib/mock';
import { today as todayNow } from './lib/units';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';
import { draftsFor, plannedCopies, rowsFor } from './lib/training/coach';
import type { CoachAnswer, CoachWeek, ObjectMeta } from './lib/types';

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
  clearFaults();
  nav.reset();
  store.set('fm-trackers', '1');
  trackers.reload();
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
  clearUndo();
});

async function openCoach() {
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  // The fourth tab, after Week, Exercises and Regime.
  const names = within(bar)
    .getAllByRole('button')
    .map((b) => b.textContent?.trim());
  expect(names).toEqual(['Week', 'Exercises', 'Regime', 'Coach']);
  await fireEvent.click(within(bar).getByRole('button', { name: 'Coach' }));
}

const coachNow = (day = todayNow()) => handle<CoachAnswer>('coach', { day, from: day, to: day });

test('a plan is followed, its week put on the calendar with your numbers, and moved on only by you', async () => {
  await openCoach();
  await fireEvent.click(
    await screen.findByRole('button', { name: /Endurance and strength in one week/ }),
  );
  expect(await screen.findByText(/the sessions carry no number of their own/)).toBeTruthy();
  expect(screen.getByText(/Anyone with an injury/)).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Follow' }));
  expect(await screen.findByRole('heading', { name: 'Week 1 of 2' })).toBeTruthy();
  // Nothing is filled in for the person: an assembled plan carries no number.
  const minutes = screen.getAllByLabelText('Minutes') as HTMLInputElement[];
  expect(minutes.length).toBe(2);
  expect(minutes.every((m) => m.value === '')).toBe(true);
  await fireEvent.input(minutes[1], { target: { value: '40' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Put this week on my calendar' }));
  expect(await screen.findByText(/This week is on your calendar: 0 of 2 done/)).toBeTruthy();
  const a = await handle<CoachAnswer>('coach', {
    day: todayNow(),
    from: '2000-01-01',
    to: '2000-01-02',
  });
  expect(a.followed[0].week).toBe(1);
  // The plan moves on only when the person presses Next week.
  await fireEvent.click(screen.getByRole('button', { name: 'Next week' }));
  expect(await screen.findByRole('heading', { name: 'Week 2 of 2' })).toBeTruthy();
  const b = await coachNow();
  expect(b.followed[0].week).toBe(2);
});

test('a planned session done today keeps what was planned', async () => {
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
          minutes: 40,
        },
      },
    ],
  });
  await openCoach();
  expect(await screen.findByRole('heading', { name: 'Today' })).toBeTruthy();
  await fireEvent.click(await screen.findByRole('button', { name: 'Done' }));
  await vi.waitFor(async () => {
    const e = (await coachNow()).days[0].training[0];
    expect(e.props.planned).toBeUndefined();
    expect(e.props.planned_minutes).toBe(40);
  });
});

test("a watch's session is matched only when the person says yes", async () => {
  const today = todayNow();
  await handle('create_notes', {
    notes: [
      {
        title: 'Coach plan',
        start: today,
        props: { program: 'coach', plan: 'hybrid-week', question: 'goal', week: 1 },
      },
      {
        title: 'Easy · Run',
        start: today,
        props: {
          log: 'workout',
          planned: 1,
          activity: 'Running',
          sport: 'run',
          counts_as: 'aerobic',
          plan: 'P',
          minutes: 30,
        },
      },
      {
        title: 'Running',
        start: `${today}T07:00`,
        props: {
          log: 'workout',
          activity: 'Running',
          sport: 'run',
          counts_as: 'aerobic',
          minutes: 33,
          day: today,
          source_key: 'fit:invented',
        },
      },
    ],
  });
  await openCoach();
  expect(await screen.findByText(/This was your planned/)).toBeTruthy();
  // Nothing is folded before the answer.
  expect((await coachNow()).days[0].training.length).toBe(2);
  await fireEvent.click(screen.getByRole('button', { name: 'Yes, it was' }));
  await vi.waitFor(async () => {
    const t = (await coachNow()).days[0].training;
    expect(t.length).toBe(1);
    expect(t[0].props.planned_minutes).toBe(30);
    expect(t[0].props.minutes).toBe(33);
    expect(t[0].props.source_key).toBe('fit:invented');
  });
});

describe('the Coach chooses no number', () => {
  const week: CoachWeek = {
    n: 1,
    note: '',
    sessions: [
      {
        id: 'a',
        day_hint: 'wed',
        sport: 'run',
        type: 'easy',
        said: 'Easy',
        steps: [{ part: 'work', said: 'Easy running' }],
        numbers_from: 'person',
      },
      {
        id: 'b',
        day_hint: 'sat',
        sport: 'run',
        type: 'long',
        said: 'Long',
        steps: [],
        numbers_from: 'source',
        minutes: 20,
      },
    ],
  };
  test("a row is blank for the person and carries a published plan's own figure", () => {
    const rows = rowsFor(week, '2026-11-16');
    expect(rows[0].day).toBe('2026-11-18');
    expect(rows[0].minutes).toBe('');
    expect(rows[1].minutes).toBe('20');
    const drafts = draftsFor(rows, 'P', 1);
    expect(drafts[0].props.minutes).toBeUndefined();
    expect(drafts[0].props).toMatchObject({
      log: 'workout',
      planned: 1,
      plan: 'P',
      plan_week: 1,
      session: 'a',
    });
    expect(drafts[1].props.minutes).toBe(20);
    expect(drafts[0].body).toBe('- Work: Easy running\n');
  });
  test('only an entry of a plan keeps what was planned', () => {
    const e = (props: Record<string, unknown>) => ({ props }) as unknown as ObjectMeta;
    expect(plannedCopies(e({ minutes: 30, planned: 1 }))).toEqual({});
    expect(plannedCopies(e({ minutes: 30, reps: 5, plan: 'P' }))).toEqual({
      planned_minutes: 30,
      planned_reps: 5,
    });
    expect(plannedCopies(e({ minutes: 30, planned_minutes: 25, plan: 'P' }))).toEqual({});
  });
});
