// **A group of views**: one entry among the views, its views as tabs at the top of its window, a
// plus that adds to it (`decisions.md#ui`, 2026-10-12: a phone's bar is the app's own), and Back that
// leaves the group instead of walking back along its tabs — `decisions.md#ui`, 2026-10-11.
//
// The tracker's demo data is asked for here; the rest of the suite never sees it.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, seedTrackerDemo } from './lib/mock';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';

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
  trackers.reload();
});
/** Turn the shipped groups on for this device, as Settings will. */
const showTrackers = () => {
  store.set('fm-trackers', '1');
  trackers.reload();
};
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
  clearUndo();
});

const openFood = async () => {
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  return screen.findByRole('group', { name: 'Food' });
};

test('the groups that come with the app are shown unless this device hides them', async () => {
  store.set('fm-trackers', '0');
  trackers.reload();
  const first = render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  expect(screen.queryByRole('button', { name: 'open Food' })).toBeNull();
  first.unmount();

  store.delete('fm-trackers');
  trackers.reload();
  render(App);
  expect(await screen.findByRole('button', { name: 'open Food' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'open Fitness' })).toBeTruthy();
});

test('a group is one entry among the views, and opens on its first view', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  // One entry for the group; none for the views inside it.
  await within(rail).findByRole('button', { name: 'open Food' });
  expect(within(rail).queryByRole('button', { name: 'open Trends' })).toBeNull();

  const bar = await openFood();
  expect(within(bar).getByRole('button', { name: 'Day' }).getAttribute('aria-current')).toBe(
    'page',
  );
  // Half a day is logged in the demo: something remains.
  expect(await screen.findByText('left')).toBeTruthy();
});

test('stepping along a group’s views is not a move: one Back leaves the group', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  const bar = await openFood();
  await screen.findByText('left');

  const depth = history.length;
  await fireEvent.click(within(bar).getByRole('button', { name: 'Trends' }));
  expect(await screen.findByText(/^Logged \d+ of 7 days$/)).toBeTruthy();
  // The strip is drawn again with the view it steps to, so it is found again.
  await fireEvent.click(
    within(await screen.findByRole('group', { name: 'Food' })).getByRole('button', { name: 'Day' }),
  );
  await screen.findByText('left');
  expect(history.length).toBe(depth);

  history.back();
  // The window in front is no longer the group's (its own stays open behind, tabs and all).
  await waitFor(() =>
    expect(
      within(document.querySelector('.cell.active') as HTMLElement).queryByRole('group', {
        name: 'Food',
      }),
    ).toBeNull(),
  );
});

test('over a view that logs, the plus opens its sheet, and Back closes the sheet before it leaves', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  const bar = await openFood();
  await screen.findByText('left');

  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  expect(await screen.findByRole('dialog', { name: 'log a food' })).toBeTruthy();

  history.back();
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'log a food' })).toBeNull());
  // Still in the group.
  expect(screen.getByRole('group', { name: 'Food' })).toBeTruthy();
});

test('removing an entry takes it out of the day and offers to put it back', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(await screen.findByText('Oats, rolled'));
  await fireEvent.click(await screen.findByRole('button', { name: 'Remove' }));

  const said = await screen.findByText(/^Removed “Oats, rolled”$/);
  expect(screen.queryByText('Oats, rolled')).toBeNull();
  await fireEvent.click(
    within(said.parentElement as HTMLElement).getByRole('button', { name: 'Undo' }),
  );
  expect(await screen.findByText('Oats, rolled')).toBeTruthy();
});

test('logged entries stay out of the board', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await screen.findByText(/GAE lambda interacts badly/);
  expect(screen.queryByText('Oats, rolled')).toBeNull();
});

test('a target set from the day screen is what the day is then measured against', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  await screen.findByText(/of 2,100 kcal/);
  await fireEvent.click(await screen.findByRole('button', { name: 'Change' }));
  const sheet = await screen.findByRole('dialog', { name: 'Food targets' });
  await fireEvent.input(within(sheet).getByLabelText('Energy (kcal)'), {
    target: { value: '1800' },
  });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Save targets' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Food targets' })).toBeNull());
  expect(await screen.findByText(/of 1,800 kcal/)).toBeTruthy();
});

test('a meal added to a day still to come is a plan, in no total until it is confirmed', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  await screen.findByText('left');
  await fireEvent.click(screen.getByRole('button', { name: 'the day after' }));
  expect(await screen.findByText('Planned')).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: 'add to Dinner' }));
  const again = await screen.findAllByRole('button', { name: /^add .* again$/ });
  const what = again[0].getAttribute('aria-label')!.replace(/^add (.*) again$/, '$1');
  await fireEvent.click(again[0]);
  await fireEvent.click(screen.getAllByRole('button', { name: 'close' }).at(-1)!);

  // On the day, waiting to be confirmed — and the day still has everything left.
  const confirm = await screen.findByRole('button', { name: `confirm ${what}` });
  expect(screen.getByText(/^2,100$/, { selector: '.big' })).toBeTruthy();
  await fireEvent.click(confirm);
  await waitFor(() => expect(screen.queryByRole('button', { name: `confirm ${what}` })).toBeNull());
  expect(screen.queryByText(/^2,100$/, { selector: '.big' })).toBeNull();
});

test('training: an exercise is planned as sets, and ticking one counts it', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  expect(await screen.findByText(/^0 of 9 sets/)).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.input(within(sheet).getByLabelText('Exercise'), { target: { value: 'Lunge' } });
  // Nothing is pre-filled with a number of the app's own: sets, reps and load are the person's.
  const add = () =>
    within(sheet).getByRole('button', { name: /^Add \d+ × \d+$/ }) as HTMLButtonElement;
  expect(add().disabled).toBe(true);
  await fireEvent.click(within(sheet).getByRole('button', { name: 'more Sets' }));
  await fireEvent.click(within(sheet).getByRole('button', { name: 'more Sets' }));
  await fireEvent.input(within(sheet).getByLabelText('Reps'), { target: { value: '8' } });
  await fireEvent.input(within(sheet).getByLabelText('Load (kg)'), { target: { value: '30' } });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Add 3 × 8' }));
  expect(await screen.findByText(/^0 of 12 sets/)).toBeTruthy();
  expect(screen.getByText('Lunge')).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: 'set 1 of Lunge' }));
  expect(await screen.findByText(/^1 of 12 sets/)).toBeTruthy();
  // A clock of what has passed since, and no prescribed rest.
  expect(await screen.findByRole('timer', { name: 'since the last set' })).toBeTruthy();
});

test('training: an activity is logged in minutes and effort, with nothing to tick', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Activity, in minutes' }));
  // The activities offered are the library's, each from a named source.
  await fireEvent.click(await within(sheet).findByRole('button', { name: 'Brisk walking' }));
  const log = () =>
    within(sheet).getByRole('button', { name: /^Log \d+ min$/ }) as HTMLButtonElement;
  expect(log().disabled).toBe(true);
  for (let i = 0; i < 6; i++)
    await fireEvent.click(within(sheet).getByRole('button', { name: 'more Minutes' }));
  for (let i = 0; i < 5; i++)
    await fireEvent.click(within(sheet).getByRole('button', { name: 'more Effort (0–10)' }));
  expect(within(sheet).getByText(/^Moderate/)).toBeTruthy();
  await fireEvent.click(log());
  expect(await screen.findByText(/30 min of activity/)).toBeTruthy();
  expect(screen.getByText(/Brisk walking · 30 min · effort 5/)).toBeTruthy();
  expect(screen.queryByRole('button', { name: /set 1 of Brisk walking/ })).toBeNull();
});

test('a regime is chosen from the library, followed, measured, and stopped', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Regime' }));
  expect(await screen.findByRole('heading', { name: 'Choose a regime to follow' })).toBeTruthy();
  await fireEvent.click(await screen.findByRole('button', { name: /WHO weekly activity/ }));
  // What it would hold you to is shown before anything is written, and each number says whose it is.
  expect(await screen.findByRole('heading', { name: 'What you will be held to' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Follow this' }));
  expect(await screen.findByRole('button', { name: 'Stop following' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'WHO weekly activity' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Stop following' }));
  expect(await screen.findByRole('heading', { name: 'Choose a regime to follow' })).toBeTruthy();
});
