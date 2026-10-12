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
  await waitFor(() => expect(screen.queryByText('Oats, rolled')).toBeNull());
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

test('a weight is written down from the trends, where it is one of the values to pick', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Trends' }));
  const pick = (await screen.findByLabelText('what to show')) as HTMLSelectElement;
  await fireEvent.change(pick, { target: { value: 'weight_g' } });
  await fireEvent.click(await screen.findByRole('button', { name: 'add Weight' }));
  const sheet = await screen.findByRole('dialog', { name: 'add a measurement' });
  await fireEvent.input(within(sheet).getByLabelText('Weight (kg)'), { target: { value: '80.5' } });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Save' }));
  // Nothing was eaten, and the day still counts as one with a weight.
  expect(await screen.findByText(/Logged 1 of 7 days/)).toBeTruthy();
  expect(screen.getByText('80.5')).toBeTruthy();
});

test('a session is kept as a routine for some weekdays, and planned again from it', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const add = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.input(within(add).getByLabelText('Exercise'), { target: { value: 'Lunge' } });
  await fireEvent.input(within(add).getByLabelText('Reps'), { target: { value: '8' } });
  await fireEvent.click(within(add).getByRole('button', { name: /^Add \d+ × 8$/ }));
  await screen.findByText(/^0 of 1 sets/);

  await fireEvent.click(screen.getByRole('button', { name: 'Keep this session as a routine' }));
  const sheet = await screen.findByRole('dialog', { name: 'keep a routine' });
  await fireEvent.input(within(sheet).getByLabelText('Name'), { target: { value: 'Legs' } });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Mon' }));
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Keep it' }));
  // It is listed with its days, and is in no day: today still has its one set.
  expect(await screen.findByRole('button', { name: 'change the routine Legs' })).toHaveProperty(
    'textContent',
    expect.stringMatching(/Legs · Mon · 1\s+set/),
  );
  expect(screen.getByText(/^0 of 1 sets/)).toBeTruthy();

  // Removing it is one press in its own sheet.
  await fireEvent.click(screen.getByRole('button', { name: 'change the routine Legs' }));
  const change = await screen.findByRole('dialog', { name: 'change a routine' });
  await fireEvent.click(within(change).getByRole('button', { name: 'Remove this routine' }));
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: 'change the routine Legs' })).toBeNull(),
  );
});

test('the day says whether it follows the diet, and what is off', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  // Follow the WHO option from the Diet tab…
  await fireEvent.click(within(bar).getByRole('button', { name: 'Diet' }));
  await fireEvent.click(await screen.findByRole('button', { name: /WHO healthy diet/ }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Follow this' }));
  await screen.findByRole('button', { name: 'Stop following' });
  // …and the Day screen names it, with nothing to say until something is logged.
  const again = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(again).getByRole('button', { name: 'Day' }));
  const diet = await screen.findByRole('region', { name: 'today against what you follow' });
  expect(within(diet).getByRole('heading', { name: 'WHO healthy diet' })).toBeTruthy();
  expect(within(diet).getByText('nothing logged yet')).toBeTruthy();
});

test('every nutrient of the day is one press from the day screen, each beside its reference', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(await screen.findByRole('button', { name: 'All nutrients' }));
  const sheet = await screen.findByRole('dialog', { name: 'all nutrients' });
  expect(await within(sheet).findByText('Calcium')).toBeTruthy();
  // The sex is not known, so what differs by it is left out and the sheet says why.
  expect(within(sheet).getByText(/differ for men and women/)).toBeTruthy();
  expect(within(sheet).getByText(/European Food Safety Authority/)).toBeTruthy();
});

test('an amount known only roughly is kept as a range, and the day says between what and what', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(await screen.findByText('Oats, rolled'));
  await fireEvent.click(await screen.findByRole('button', { name: 'Not sure of the amount?' }));
  const grams = Number((screen.getByLabelText('amount in grams') as HTMLInputElement).value);
  await fireEvent.input(screen.getByLabelText('At least (g)'), { target: { value: grams - 10 } });
  await fireEvent.input(screen.getByLabelText('At most (g)'), { target: { value: grams + 10 } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(await screen.findByText(/^between .* and .* kcal$/)).toBeTruthy();
  expect(screen.getByText(new RegExp(`^${grams - 10}–${grams + 10} g$`))).toBeTruthy();
});

test('the menu is the days ahead by meal, for as many days as asked, and says what they need', async () => {
  seedTrackerDemo();
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Menu' }));
  expect(await screen.findByRole('heading', { name: 'Menu' })).toBeTruthy();
  const days = () => screen.getAllByRole('button', { name: /^plan Breakfast for / }).length;
  await waitFor(() => expect(days()).toBe(7));
  await fireEvent.click(screen.getByRole('button', { name: 'one day fewer' }));
  await waitFor(() => expect(days()).toBe(6));
  // Planning a meal uses the sheet the day screen uses.
  await fireEvent.click(screen.getAllByRole('button', { name: /^plan Lunch for / })[1]);
  expect(await screen.findByRole('dialog')).toBeTruthy();
});

test('a packaged food is written down once from its label, and is then one of your foods', async () => {
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Search' }));
  await fireEvent.click(
    await within(sheet).findByRole('button', { name: /A packaged food\? Write it down/ }),
  );
  const form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  const save = () => within(form).getByRole('button', { name: 'Save it' }) as HTMLButtonElement;
  await fireEvent.input(within(form).getByLabelText('Name'), { target: { value: 'Penne' } });
  expect(save().disabled).toBe(true); // energy is needed
  await fireEvent.input(within(form).getByLabelText('Energy (kcal)'), { target: { value: '359' } });
  await fireEvent.input(within(form).getByLabelText('Salt (g)'), { target: { value: '0.5' } });
  // A number that is not a barcode is said to be none, and is not kept.
  await fireEvent.input(within(form).getByLabelText('Barcode number (optional)'), {
    target: { value: '6291041500214' },
  });
  expect(within(form).getByText(/its last digit does not check/)).toBeTruthy();
  expect(save().disabled).toBe(true);
  await fireEvent.input(within(form).getByLabelText('Barcode number (optional)'), {
    target: { value: '6291041500213' },
  });
  await fireEvent.click(save());
  // It opens at an amount, ready to log, with the label's energy for 100 g.
  expect(await within(sheet).findByRole('heading', { name: 'Penne' })).toBeTruthy();
  expect((within(sheet).getByLabelText('amount in grams') as HTMLInputElement).value).toBe('100');
});

test('a barcode is looked up only when asked, fills the label, and the note says where it came from', async () => {
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Search' }));
  await fireEvent.input(within(sheet).getByLabelText('search foods'), {
    target: { value: '3017624010701' },
  });
  await fireEvent.click(
    await within(sheet).findByRole('button', {
      name: 'A barcode you have not written down: add it',
    }),
  );
  const form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  // Nothing is asked until the press; the notice says what is sent and to whom.
  expect((within(form).getByLabelText('Energy (kcal)') as HTMLInputElement).value).toBe('');
  expect(within(form).getByText(/sends the barcode number, and nothing else/)).toBeTruthy();
  await fireEvent.click(
    within(form).getByRole('button', { name: 'Look it up at Open Food Facts' }),
  );
  await waitFor(() =>
    expect((within(form).getByLabelText('Energy (kcal)') as HTMLInputElement).value).toBe('539'),
  );
  expect((within(form).getByLabelText('Name') as HTMLInputElement).value).toBe('Nutella (Ferrero)');
  // Sodium came back; the form shows it as the salt a label states.
  expect((within(form).getByLabelText('Salt (g)') as HTMLInputElement).value).toBe('0.11');
  expect(within(form).getByText(/Check it against the pack/)).toBeTruthy();
  await fireEvent.click(within(form).getByRole('button', { name: 'Save it' }));
  expect(await within(sheet).findByRole('heading', { name: 'Nutella (Ferrero)' })).toBeTruthy();
});

test('what you say you have in your kitchen is a list of names, kept from the menu', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Menu' }));
  const kitchen = await screen.findByRole('region', { name: 'in your kitchen' });
  await fireEvent.input(within(kitchen).getByLabelText('something you have'), {
    target: { value: 'Rice' },
  });
  await fireEvent.click(within(kitchen).getByRole('button', { name: 'Add' }));
  expect(await within(kitchen).findByText('Rice')).toBeTruthy();
  await fireEvent.click(within(kitchen).getByRole('button', { name: 'no longer have Rice' }));
  await waitFor(() => expect(within(kitchen).queryByText('Rice')).toBeNull());
});
