// **A group of views**: one entry among the views, its views as tabs at the top of its window, a
// plus that adds to it (`decisions.md#ui`, 2026-10-12: a phone's bar is the app's own), and Back that
// leaves the group instead of walking back along its tabs — `decisions.md#ui`, 2026-10-11.
//
// The tracker's demo data is asked for here; the rest of the suite never sees it.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, seedTrackerDemo, setLabelReader } from './lib/mock';
import * as nav from './lib/nav.svelte';
import { clear as clearUndo } from './lib/undo.svelte';
import * as trackers from './lib/trackers.svelte';

// A browser draws the picture small before it is read; this suite has no canvas to draw on.
vi.mock('./lib/food/shot', () => ({
  shrink: async (_: Blob, turns = 0) => `data:image/jpeg;base64,TURN${turns}`,
}));

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

test('one entry comes out on its own from the day, the rest of the meal stays', async () => {
  // The owner, 2026-10-18: two coffees added by mistake, and the whole breakfast removed to fix it.
  seedTrackerDemo();
  showTrackers();
  render(App);
  await openFood();
  const oats = await screen.findByText('Oats, rolled');
  const meal = oats.closest('section') as HTMLElement;
  const before = within(meal).getAllByRole('button', { name: /^remove / }).length;
  expect(before).toBeGreaterThan(1);

  await fireEvent.click(within(meal).getByRole('button', { name: 'remove Oats, rolled' }));
  const said = await screen.findByText(/^Removed “Oats, rolled”$/);
  await waitFor(() => expect(screen.queryByText('Oats, rolled')).toBeNull());
  // Only that one: everything else in the meal is still there.
  await waitFor(() =>
    expect(within(meal).getAllByRole('button', { name: /^remove / })).toHaveLength(before - 1),
  );
  await fireEvent.click(
    within(said.parentElement as HTMLElement).getByRole('button', { name: 'Undo' }),
  );
  expect(await screen.findByText('Oats, rolled')).toBeTruthy();
});

test('a recipe asked for from its note opens in Food, on its ingredients', async () => {
  seedTrackerDemo();
  showTrackers();
  const { handle } = await import('./lib/mock');
  await handle('create_notes', {
    notes: [{ title: 'Tiramisù', tags: ['recipe'], props: { recipe: 'w8', servings: 6 } }],
  });
  render(App);
  trackers.askToOpen('recipe/w8');
  await openFood();
  expect(await screen.findByRole('heading', { name: 'Tiramisù' })).toBeTruthy();
  expect(screen.getByLabelText('Servings it makes')).toBeTruthy();
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
  // Fitness opens on its week; a day is opened from it.
  await fireEvent.click(await screen.findByRole('button', { name: 'open today' }));
  expect(await screen.findByText(/^0 of 9 sets/)).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.click(
    within(sheet).getByRole('button', { name: 'Not in the list: type a name' }),
  );
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
  // Fitness opens on its week; a day is opened from it.
  await fireEvent.click(await screen.findByRole('button', { name: 'open today' }));
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.click(
    within(sheet).getByRole('button', { name: 'Not in the list: type a name' }),
  );
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
  expect(await screen.findByRole('heading', { name: 'Choose what to follow' })).toBeTruthy();
  // The "worth watching" set is there before anything is followed (2026-10-16).
  expect(screen.getByRole('heading', { name: 'Worth watching' })).toBeTruthy();
  // Shelved by kind; WHO is the card to start from.
  const who = await screen.findByRole('button', { name: /WHO weekly activity/ });
  expect(who.textContent).toMatch(/Start here/);
  expect(who.textContent).toMatch(/Guideline/);
  await fireEvent.click(who);
  // What it would hold you to is shown before anything is written, and each number says whose it is.
  expect(await screen.findByRole('heading', { name: 'What you will be held to' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Follow this' }));
  expect(await screen.findByRole('button', { name: 'Stop following' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'What you follow' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'WHO weekly activity' })).toBeTruthy();
  // Another card is added from the library, where the followed one is marked.
  await fireEvent.click(screen.getByRole('button', { name: 'Add a card, or change one' }));
  expect((await screen.findByRole('button', { name: /WHO weekly activity/ })).textContent).toMatch(
    /Following/,
  );
  await fireEvent.click(screen.getByRole('button', { name: /Back to what you follow/ }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Stop following' }));
  expect(await screen.findByRole('heading', { name: 'Choose what to follow' })).toBeTruthy();
});

test('a flag set in "Anything we should know?" leaves out the cards not meant for it', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Diet' }));
  await fireEvent.click(await screen.findByRole('button', { name: /WHO healthy diet/ }));
  await fireEvent.click(await screen.findByText(/Anything we should know/));
  await fireEvent.click(screen.getByLabelText('Breastfeeding'));
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await fireEvent.click(await screen.findByRole('button', { name: /All diets/ }));
  // Time-restricted eating is not for breastfeeding: left out, and said so.
  expect(await screen.findByText(/1 card is left out because of what you told us/)).toBeTruthy();
  expect(screen.queryByRole('button', { name: /Time-restricted eating/ })).toBeNull();
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
  // Fitness opens on its week; a day is opened from it.
  await fireEvent.click(await screen.findByRole('button', { name: 'open today' }));
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const add = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.click(within(add).getByRole('button', { name: 'Not in the list: type a name' }));
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

test('a diet being fine-tuned is still open, as typed, after another view and after the app is closed', async () => {
  showTrackers();
  const app = render(App);
  const toDiet = async () => {
    const rail = await screen.findByRole('navigation', { name: 'views' });
    await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
    const bar = await screen.findByRole('group', { name: 'Food' });
    await fireEvent.click(within(bar).getByRole('button', { name: 'Diet' }));
    return bar;
  };
  let bar = await toDiet();
  await fireEvent.click(await screen.findByRole('button', { name: /WHO healthy diet/ }));
  await screen.findByRole('button', { name: 'Follow this' });
  const bound = () => screen.queryAllByRole('spinbutton')[0] as HTMLInputElement | undefined;
  await waitFor(() => expect(bound()).toBeTruthy());
  await fireEvent.input(bound()!, { target: { value: '7' } });
  // Away to the day, and back: it is as it was.
  await fireEvent.click(within(bar).getByRole('button', { name: 'Day' }));
  bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Diet' }));
  expect(await screen.findByRole('button', { name: 'Follow this' })).toBeTruthy();
  await waitFor(() => expect(bound()?.value).toBe('7'));
  // The app closed and opened again: it opens on the same option, with what was typed.
  app.unmount();
  render(App);
  bar = await toDiet();
  expect(await screen.findByText(/Carried on from where you left it/)).toBeTruthy();
  expect(await screen.findByRole('button', { name: 'Follow this' })).toBeTruthy();
  await waitFor(() => expect(bound()?.value).toBe('7'));
  // Its own Back is leaving it: the library is shown the next time.
  await fireEvent.click(screen.getByRole('button', { name: /All diets/ }));
  expect(await screen.findByRole('button', { name: /WHO healthy diet/ })).toBeTruthy();
  expect(store.has('fm-draft:plan:food')).toBe(false);
});

test('a label half typed, and a food half typed in, are there when the sheet is opened again', async () => {
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  let sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Type in a label' }));
  let form = await within(sheet).findByRole('region', { name: 'a packaged food, from its label' });
  await fireEvent.input(within(form).getByLabelText('Name'), { target: { value: 'Penne' } });
  await fireEvent.input(within(form).getByLabelText('Fat (g)'), { target: { value: '1.5' } });
  // Closed without saving.
  await fireEvent.click(within(sheet).getByRole('button', { name: 'close' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'add to Food' }));
  sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Type in a label' }));
  form = await within(sheet).findByRole('region', { name: 'a packaged food, from its label' });
  expect((within(form).getByLabelText('Name') as HTMLInputElement).value).toBe('Penne');
  expect((within(form).getByLabelText('Fat (g)') as HTMLInputElement).value).toBe('1.5');
  expect(within(form).getByText(/Carried on from where you left it/)).toBeTruthy();
  await fireEvent.click(within(form).getByRole('button', { name: 'Start afresh' }));
  await waitFor(() =>
    expect((within(sheet).getByLabelText('Name') as HTMLInputElement).value).toBe(''),
  );
  // Quick add keeps its name and figures the same way.
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Back' }));
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Quick add' }));
  await fireEvent.input(within(sheet).getByLabelText('Name'), { target: { value: 'Soup' } });
  await fireEvent.click(within(sheet).getByRole('button', { name: 'close' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'add to Food' }));
  sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Quick add' }));
  expect((within(sheet).getByLabelText('Name') as HTMLInputElement).value).toBe('Soup');
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

test('scanning a barcode is offered the moment the add sheet opens', async () => {
  // It was two screens away, behind small print at the foot of Search, and was not found.
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  expect(within(sheet).getByRole('button', { name: 'Scan a barcode' })).toBeTruthy();
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Type in a label' }));
  const form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  // And the one picker serves the form's own button too.
  expect(within(form).getByRole('button', { name: 'Photograph the barcode' })).toBeTruthy();
  expect(document.querySelectorAll('.log-overlay input[type="file"]').length).toBe(1);
});

test('a label is photographed, read into the form, and saved only by the person', async () => {
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  // Beside the barcode, on the first screen; pressing it opens the one picker.
  expect(within(sheet).getByRole('button', { name: 'Photograph a label' })).toBeTruthy();
  const picker = document.querySelector('.log-overlay input[type="file"]') as HTMLInputElement;
  const clicked = vi.spyOn(picker, 'click').mockImplementation(() => {});
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Photograph a label' }));
  expect(clicked).toHaveBeenCalledTimes(1);
  Object.defineProperty(picker, 'files', {
    configurable: true,
    value: [new File(['x'], 'label.jpg', { type: 'image/jpeg' })],
  });
  await fireEvent.change(picker);
  const form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  // What the stand-in reader returns is a can of cola: per 100 ml, the can as its serving.
  expect(await within(form).findByText(/Read by stand-in: 7 of the label's figures/)).toBeTruthy();
  expect((within(form).getByLabelText('per 100 ml') as HTMLInputElement).checked).toBe(true);
  expect((within(form).getByLabelText('Energy') as HTMLInputElement).value).toBe('42');
  expect((within(form).getByLabelText('Carbohydrate (g)') as HTMLInputElement).value).toBe('10.6');
  expect((within(form).getByLabelText('A serving is (optional)') as HTMLInputElement).value).toBe(
    '330',
  );
  // The picture is shown beside the figures, and what was read can be opened.
  expect(within(form).getByAltText('the label, as photographed')).toBeTruthy();
  expect(within(form).getByText('What was read')).toBeTruthy();
  // Nothing is saved by the reading: it has no name yet, and the person presses Save.
  const save = within(form).getByRole('button', { name: 'Save it' }) as HTMLButtonElement;
  expect(save.disabled).toBe(true);
  // Turning the picture reads it again, a quarter turn on.
  await fireEvent.click(
    within(form).getByRole('button', { name: 'Turn the picture and read it again' }),
  );
  await waitFor(() =>
    expect(
      (within(form).getByAltText('the label, as photographed') as HTMLImageElement).src,
    ).toContain('TURN1'),
  );
  await fireEvent.input(within(form).getByLabelText('Name'), { target: { value: 'Cola' } });
  await waitFor(() => expect(save.disabled).toBe(false));
  await fireEvent.click(save);
  expect(await within(sheet).findByRole('heading', { name: 'Cola' })).toBeTruthy();
  expect(within(sheet).getByLabelText('amount in millilitres')).toBeTruthy();
});

test('where no model can read a label the picture is kept, and the label is settled later', async () => {
  const why = 'The model on this device (tiny) misreads nutrition labels too often.';
  setLabelReader(why);
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  let sheet = await screen.findByRole('dialog');
  const picker = document.querySelector('.log-overlay input[type="file"]') as HTMLInputElement;
  vi.spyOn(picker, 'click').mockImplementation(() => {});
  const take = async () => {
    Object.defineProperty(picker, 'files', {
      configurable: true,
      value: [new File(['x'], 'label.jpg', { type: 'image/jpeg' })],
    });
    await fireEvent.change(picker);
  };
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Photograph a label' }));
  await take();
  let form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  // It is not read here, and says why; the picture is kept, and that is enough to save on.
  expect(
    within(form).getByText(/misreads nutrition labels too often\. The picture is kept/),
  ).toBeTruthy();
  expect(within(form).getByRole('list', { name: 'pictures of the pack' }).children.length).toBe(1);
  expect(within(form).queryByRole('button', { name: /read the label from picture/ })).toBeNull();
  const save = () => within(form).getByRole('button', { name: 'Save it' }) as HTMLButtonElement;
  expect(save().disabled).toBe(true); // it still needs a name
  await fireEvent.input(within(form).getByLabelText('Name'), { target: { value: 'Oat bar' } });
  await waitFor(() => expect(save().disabled).toBe(false));
  await fireEvent.click(save());
  // It opens at an amount, says it is not counted yet, and can be logged all the same.
  expect(await within(sheet).findByRole('heading', { name: 'Oat bar' })).toBeTruthy();
  expect(within(sheet).getByText(/Its label has not been read yet/)).toBeTruthy();
  await fireEvent.change(within(sheet).getByLabelText('amount in grams'), {
    target: { value: '50' },
  });
  await fireEvent.change(within(sheet).getByRole('combobox'), { target: { value: 'lunch' } });
  await fireEvent.click(within(sheet).getByRole('button', { name: /^Add to / }));
  // It is in the day with its amount and no energy. Find it again by name.
  await fireEvent.click(await screen.findByRole('button', { name: 'add to Food' }));
  sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Search' }));
  await fireEvent.input(within(sheet).getByRole('searchbox'), { target: { value: 'Oat bar' } });
  await fireEvent.click(
    await within(sheet).findByRole('button', { name: /Oat bar.*label not read yet/ }),
  );
  // Later, where a model can read: its label is opened again and read from the kept picture.
  setLabelReader('');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Fill in its label' }));
  form = await within(sheet).findByRole('region', { name: 'a packaged food, from its label' });
  expect((within(form).getByLabelText('Name') as HTMLInputElement).value).toBe('Oat bar');
  expect(within(form).getByRole('list', { name: 'pictures of the pack' }).children.length).toBe(1);
  // Typed here; reading the kept picture is the same reading as a new one.
  await fireEvent.input(within(form).getByLabelText('Energy'), { target: { value: '400' } });
  await waitFor(() => expect(save().disabled).toBe(false));
  await fireEvent.click(save());
  expect(await within(sheet).findByRole('button', { name: 'Change its label' })).toBeTruthy();
  // And the 50 g logged before now has its share: 400 kcal in 100 g is 200.
  expect(
    within(sheet).getByText('Filled in 1 entry logged before its label was read.'),
  ).toBeTruthy();
  expect(within(sheet).queryByText(/Its label has not been read yet/)).toBeNull();
  // Its label opened again to put a figure right, and left half-way: the change is kept.
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Change its label' }));
  form = await within(sheet).findByRole('region', { name: 'a packaged food, from its label' });
  expect(within(form).queryByText(/Carried on from where you left it/)).toBeNull();
  await fireEvent.input(within(form).getByLabelText('Energy'), { target: { value: '410' } });
  await fireEvent.click(within(form).getByRole('button', { name: 'Back' }));
  await fireEvent.click(await within(sheet).findByRole('button', { name: /Oat bar/ }));
  await fireEvent.click(await within(sheet).findByRole('button', { name: 'Change its label' }));
  form = await within(sheet).findByRole('region', { name: 'a packaged food, from its label' });
  expect((within(form).getByLabelText('Energy') as HTMLInputElement).value).toBe('410');
  expect(within(form).getByText(/Carried on from where you left it/)).toBeTruthy();
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
  await fireEvent.input(within(form).getByLabelText('Energy'), { target: { value: '359' } });
  await fireEvent.input(within(form).getByLabelText('Salt'), { target: { value: '0.5' } });
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

test('a drink is typed in as its label prints it, and is logged in millilitres', async () => {
  // Per serving, in kJ, with sodium in milligrams: a label that is none of the European defaults.
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Type in a label' }));
  const form = await within(sheet).findByRole('region', {
    name: 'a packaged food, from its label',
  });
  const save = () => within(form).getByRole('button', { name: 'Save it' }) as HTMLButtonElement;
  await fireEvent.input(within(form).getByLabelText('Name'), { target: { value: 'Oat drink' } });
  await fireEvent.click(within(form).getByLabelText('per serving'));
  await fireEvent.change(within(form).getByLabelText('energy given in'), {
    target: { value: 'kJ' },
  });
  await fireEvent.input(within(form).getByLabelText('Energy'), { target: { value: '481' } });
  // Figures per serving cannot be kept until the serving has a size.
  expect(save().disabled).toBe(true);
  await fireEvent.input(within(form).getByLabelText('A serving is'), { target: { value: '250' } });
  await fireEvent.change(within(form).getByLabelText('serving measured in'), {
    target: { value: 'ml' },
  });
  await fireEvent.change(within(form).getByLabelText('salt or sodium'), {
    target: { value: 'sodium' },
  });
  await fireEvent.input(within(form).getByLabelText('Sodium'), { target: { value: '100' } });
  expect(save().disabled).toBe(false);
  await fireEvent.click(save());
  // It opens at its serving, in its own unit, and is never offered a range in grams.
  expect(await within(sheet).findByRole('heading', { name: 'Oat drink' })).toBeTruthy();
  const amount = within(sheet).getByLabelText('amount in millilitres') as HTMLInputElement;
  expect(amount.value).toBe('250');
  expect(within(sheet).queryByLabelText('amount in grams')).toBeNull();
  expect(within(sheet).queryByRole('button', { name: 'Not sure of the amount?' })).toBeNull();
  // 481 kJ a serving is 115 kcal; 250 ml of it is that again.
  expect(within(sheet).getByText(/115 kcal/)).toBeTruthy();
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
  expect((within(form).getByLabelText('Energy') as HTMLInputElement).value).toBe('');
  expect(within(form).getByText(/sends the barcode number, and nothing else/)).toBeTruthy();
  await fireEvent.click(
    within(form).getByRole('button', { name: 'Look it up at Open Food Facts' }),
  );
  await waitFor(() =>
    expect((within(form).getByLabelText('Energy') as HTMLInputElement).value).toBe('539'),
  );
  expect((within(form).getByLabelText('Name') as HTMLInputElement).value).toBe('Nutella (Ferrero)');
  // Sodium came back; the form shows it as the salt a label states.
  expect((within(form).getByLabelText('Salt') as HTMLInputElement).value).toBe('0.11');
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

test('a recipe carried in the app is shown as published and logged by its source’s own figures', async () => {
  showTrackers();
  render(App);
  await openFood();
  await fireEvent.click(screen.getByRole('button', { name: 'add to Food' }));
  const sheet = await screen.findByRole('dialog');
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Recipes' }));
  // They are searched by dish or by ingredient, in the one search every recipe is in.
  const seek = await within(sheet).findByLabelText('search recipes');
  await fireEvent.input(seek, { target: { value: 'lasagne' } });
  expect(await within(sheet).findByText('No recipe names that.')).toBeTruthy();
  await fireEvent.input(seek, { target: { value: 'diced tomatoes' } });
  await fireEvent.click(await within(sheet).findByRole('button', { name: /Vegetable Stew/ }));
  const recipe = await within(sheet).findByRole('region', { name: 'a recipe, as published' });
  expect(within(recipe).getByText('3 C water')).toBeTruthy();
  expect(within(recipe).getByText('Heat water in a large pot.')).toBeTruthy();
  expect(within(recipe).getByText('Tip: Good with bread.')).toBeTruthy();
  expect(within(recipe).getByText(/National Heart, Lung, and Blood Institute/)).toBeTruthy();
  expect(within(recipe).getByRole('link', { name: "recipe's own page" })).toBeTruthy();
});

test('fitness is the week: what is meant for each day, and a day opens in its place', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  expect(await screen.findByRole('heading', { name: 'This week' })).toBeTruthy();
  // Seven days, and with no routine yet the screen says where a timetable comes from.
  await waitFor(() =>
    expect(screen.getAllByRole('button', { name: /^open (today|[A-Z][a-z]{2}, )/ }).length).toBe(7),
  );
  expect(screen.getByText(/A timetable comes from routines/)).toBeTruthy();
  expect(screen.getByText(/Nothing is followed/)).toBeTruthy();
  // The week before has no "today"; its last day opens as a session of that day.
  await fireEvent.click(screen.getByRole('button', { name: 'the week before' }));
  await screen.findByRole('button', { name: 'Back to this week' });
  await waitFor(() =>
    expect(screen.getAllByRole('button', { name: /^open (today|[A-Z][a-z]{2}, )/ }).length).toBe(7),
  );
  await fireEvent.click(
    screen.getAllByRole('button', { name: /^open (today|[A-Z][a-z]{2}, )/ })[6],
  );
  // The day is shown in the week's place, named, and Back returns to the week.
  expect(await screen.findByRole('button', { name: 'Back to the week' })).toBeTruthy();
  expect(screen.queryByRole('heading', { name: 'Today' })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Back to the week' }));
  expect(await screen.findByRole('heading', { name: 'This week' })).toBeTruthy();
});

test('an exercise card shows how it is done in its source’s words, credited, and adds to the session', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Exercises' }));
  // No equipment first: the NHS's; the gym's are behind their own choice.
  expect(await screen.findByRole('button', { name: /Wall press-up/ })).toBeTruthy();
  expect(screen.queryByRole('button', { name: /^Deadlift/ })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Free weights' }));
  await fireEvent.click(await screen.findByRole('button', { name: /^Deadlift/ }));
  const card = await screen.findByRole('article', { name: 'Deadlift' });
  expect(
    within(card).getByText('From the starting position, extend the hips and knees.'),
  ).toBeTruthy();
  expect(within(card).getByText(/Written for soldiers, and shown as written/)).toBeTruthy();
  // A trainer's video is said to be a trainer's, with its views.
  expect(
    within(card).getByText(/a trainer’s channel — no known body’s film was found/),
  ).toBeTruthy();
  await fireEvent.click(within(card).getByRole('button', { name: 'Add to today’s session' }));
  // The sheet opens on that exercise with a place to start: the card gives no figure of its
  // own, so it is the general one, and the sheet says whose.
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  expect((within(sheet).getByLabelText('Exercise') as HTMLInputElement).value).toBe('Deadlift');
  expect(await within(sheet).findByRole('button', { name: 'Add 2 × 8' })).toBeTruthy();
  expect(within(sheet).getByRole('note').textContent).toMatch(/from the NHS/);
});

test('with no timetable yet, the week shows example weeks as a public body published them', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  const examples = await screen.findByRole('region', { name: 'example weeks' });
  expect(within(examples).getByText(/Example 1: Moderate-intensity activity/)).toBeTruthy();
  expect(
    within(examples).getByText(/Centers for Disease Control and Prevention \(seen/),
  ).toBeTruthy();
});

test('an exercise in a session that has a card says how it is done, over the session', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  // Fitness opens on its week; a day is opened from it.
  await fireEvent.click(await screen.findByRole('button', { name: 'open today' }));
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const add = await screen.findByRole('dialog', { name: 'add to the session' });
  await fireEvent.click(within(add).getByRole('button', { name: 'Not in the list: type a name' }));
  await fireEvent.input(within(add).getByLabelText('Exercise'), { target: { value: 'deadlift' } });
  await fireEvent.input(within(add).getByLabelText('Reps'), { target: { value: '5' } });
  await fireEvent.click(within(add).getByRole('button', { name: /^Add \d+ × 5$/ }));
  // Named as the card is, case aside: the session offers "How".
  await fireEvent.click(await screen.findByRole('button', { name: 'How' }));
  const over = await screen.findByRole('dialog', { name: 'how it is done' });
  expect(await within(over).findByRole('article', { name: 'Deadlift' })).toBeTruthy();
  expect(within(over).queryByRole('button', { name: 'Add to today’s session' })).toBeNull();
  await fireEvent.click(within(over).getByRole('button', { name: 'close' }));
  expect(await screen.findByText(/^0 of 1 sets/)).toBeTruthy();
});

test('what is added to a day is picked from a list and arrives with numbers to change', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(await screen.findByRole('button', { name: 'open today' }));
  await fireEvent.click(screen.getByRole('button', { name: 'add to Fitness' }));
  const sheet = await screen.findByRole('dialog', { name: 'add to the session' });
  // An exercise card with a line of its own: its numbers, and the sheet quotes the line.
  await fireEvent.click(await within(sheet).findByRole('button', { name: /^Wall press-up/ }));
  expect(await within(sheet).findByRole('button', { name: 'Add 3 × 5' })).toBeTruthy();
  expect(within(sheet).getByRole('note').textContent).toMatch(
    /Its card says: “Attempt 3 sets of 5 to 10/,
  );
  // Back to the list: an activity is by the minute.
  await fireEvent.click(within(sheet).getByRole('button', { name: 'Choose another' }));
  await fireEvent.click(await within(sheet).findByRole('button', { name: /^Brisk walking/ }));
  expect(within(sheet).getByRole('button', { name: /^Log \d+ min$/ })).toBeTruthy();
});

test('following a regime puts a week to start from on the first screen, and using it makes routines', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Fitness' }));
  const bar = await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Regime' }));
  await fireEvent.click(await screen.findByRole('button', { name: /WHO weekly activity/ }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Follow this' }));
  await screen.findByRole('button', { name: 'Stop following' });
  // Back on the week — the first screen — the regime's example is there, said to be only a start.
  const again = await screen.findByRole('group', { name: 'Fitness' });
  await fireEvent.click(within(again).getByRole('button', { name: 'Week' }));
  const example = await screen.findByRole('region', { name: 'a week to start from' });
  expect(within(example).getByText(/only a simple indication/)).toBeTruthy();
  expect(within(example).getByText(/Brisk walking · 30 min/)).toBeTruthy();
  await fireEvent.click(within(example).getByRole('button', { name: 'Use it as my week' }));
  // They are routines now: the example steps back, folded, and the week is the person's own.
  expect(await screen.findByText('A week to start from, for WHO weekly activity')).toBeTruthy();
  expect((await screen.findAllByText(/Meant for it: (Brisk walking|Weight training)/)).length).toBe(
    2,
  );
});

test('a diet followed offers a day to start from, said to be only an indication, and plans it', async () => {
  showTrackers();
  render(App);
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  const bar = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(bar).getByRole('button', { name: 'Diet' }));
  await fireEvent.click(await screen.findByRole('button', { name: /WHO healthy diet/ }));
  // Before following: the example day, with what it falls short of.
  const example = await screen.findByRole('region', { name: 'a day to start from' });
  expect(within(example).getByText(/only an indication/)).toBeTruthy();
  expect(within(example).getByText(/short by 21 g/)).toBeTruthy();
  expect(await within(example).findByText(/Vegetable Stew/)).toBeTruthy();
  await fireEvent.click(within(example).getByRole('button', { name: 'Plan it for today' }));
  expect(await within(example).findByText(/Planned for today/)).toBeTruthy();
  // It is a planned meal of today, to confirm or change on the Day tab.
  const again = await screen.findByRole('group', { name: 'Food' });
  await fireEvent.click(within(again).getByRole('button', { name: 'Day' }));
  expect(await screen.findByRole('button', { name: 'confirm Vegetable Stew' })).toBeTruthy();
});
