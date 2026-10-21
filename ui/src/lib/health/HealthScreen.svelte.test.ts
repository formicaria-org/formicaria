// **The Health diary** (`decisions.md#ui`, 2026-10-22): what a person tracks is keys on their own
// Health template; a day is one note written as the rows are touched; a row left alone is not
// recorded, and a recorded 0 is; the month counts and labels nothing.
import { cleanup, render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, expect, test, vi } from 'vitest';
import HealthScreen from './HealthScreen.svelte';
import { handle, reset } from '../mock';
import * as nav from '../nav.svelte';
import { clear as clearUndo } from '../undo.svelte';
import type { HealthDays, NoteDetail, TotalsDef, Tracker } from '../types';

const DIARY: TotalsDef = { log: 'health', add: 'health', span: 'diary', metrics: [], slots: [] };
/** Over time opens on its month's grid. */
const MONTH: TotalsDef = { ...DIARY, span: 'range' };
const TODAY = '2026-10-22';

afterEach(() => {
  reset();
  clearUndo();
  nav.reset();
});

const days = (from: string, to: string) => handle<HealthDays>('health_days', { from, to });

test('with nothing tracked, a starter writes its keys into one Health template note', async () => {
  render(HealthScreen, { def: DIARY, today: TODAY, signal: 0, onsaved: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: 'Track something' }));
  const sheet = await screen.findByRole('dialog', { name: 'track something' });
  await fireEvent.click(within(sheet).getByRole('button', { name: /^Headache/ }));

  // The rows appear, each the same every day.
  await screen.findByRole('group', { name: 'Headache' });
  expect(screen.getByRole('group', { name: 'Nausea' })).toBeTruthy();
  const keys = (await handle<Tracker[]>('health_trackers', {})).map((t) => t.key);
  expect(keys).toContain('headache_0to3');
  expect(keys).toContain('light_hurts_yes');
  expect((await days(TODAY, TODAY)).template).not.toBeNull();
});

test('touching a row makes the day note; a recorded no is kept apart from not recorded', async () => {
  await handle('create_notes', {
    notes: [
      {
        title: 'Health diary',
        tags: ['template'],
        props: { group: 'Health', headache_0to3: 0, bedding_washed_yes: 0 },
      },
    ],
  });
  render(HealthScreen, { def: DIARY, today: TODAY, signal: 0, onsaved: () => {} });
  const headache = await screen.findByRole('group', { name: 'Headache' });
  await fireEvent.click(within(headache).getByRole('button', { name: 'Moderate' }));
  await waitFor(async () =>
    expect((await days(TODAY, TODAY)).days[0].values).toEqual({ headache_0to3: 2 }),
  );

  const bedding = screen.getByRole('group', { name: 'Bedding washed' });
  await fireEvent.click(within(bedding).getByRole('button', { name: 'No' }));
  await waitFor(async () =>
    expect((await days(TODAY, TODAY)).days[0].values).toEqual({
      headache_0to3: 2,
      bedding_washed_yes: 0,
    }),
  );
  // One note for the day, however many rows were touched: the second row went into the first's.
  const id = (await days(TODAY, TODAY)).days[0].id;
  expect(id).toBeTruthy();

  // Pressing the chosen word again takes the value away: back to not recorded.
  await fireEvent.click(within(headache).getByRole('button', { name: 'Moderate' }));
  await waitFor(async () =>
    expect((await days(TODAY, TODAY)).days[0].values).toEqual({ bedding_washed_yes: 0 }),
  );
  expect((await days(TODAY, TODAY)).days[0].id).toBe(id);
});

test('the month shows one tracker and counts recorded days, days at 1 or more, and moderate or severe', async () => {
  await handle('create_notes', {
    notes: [
      { title: 'Health diary', tags: ['template'], props: { group: 'Health', headache_0to3: 0 } },
      { title: 'Health diary', start: '2026-10-01', props: { log: 'health', headache_0to3: 0 } },
      { title: 'Health diary', start: '2026-10-02', props: { log: 'health', headache_0to3: 1 } },
      { title: 'Health diary', start: '2026-10-03', props: { log: 'health', headache_0to3: 3 } },
    ],
  });
  render(HealthScreen, { def: MONTH, today: TODAY, signal: 0, onsaved: () => {} });
  await screen.findByRole('combobox');
  const counts = await screen.findByText('Recorded');
  const dl = counts.closest('dl')!;
  expect(within(dl).getByText('3 of 31 days')).toBeTruthy();
  expect(within(dl).getByText('2 days')).toBeTruthy();
  expect(within(dl).getByText('1 day')).toBeTruthy();
  // The other 28 days of October were not recorded, and say so.
  expect(screen.getAllByRole('img', { name: /not recorded/ })).toHaveLength(28);
  // It counts; it does not name a condition.
  expect(document.body.textContent).not.toMatch(/migraine|chronic|overuse|diagnos/i);
});

test('templates hold the trackers and nothing else is written to them by the diary', async () => {
  await handle('create_notes', {
    notes: [
      { title: 'Health diary', tags: ['template'], props: { group: 'Health', sleep_min: 0 } },
    ],
  });
  render(HealthScreen, { def: DIARY, today: TODAY, signal: 0, onsaved: () => {} });
  const input = await screen.findByRole('spinbutton', { name: 'Sleep' });
  await fireEvent.change(input, { target: { value: '425' } });
  await waitFor(async () =>
    expect((await days(TODAY, TODAY)).days[0].values).toEqual({ sleep_min: 425 }),
  );
  const template = (await days(TODAY, TODAY)).template!;
  expect((await handle<NoteDetail>('get', { id: template })).props.sleep_min).toBe(0);
});

const seedMonth = () =>
  handle('create_notes', {
    notes: [
      {
        title: 'Health diary',
        tags: ['template'],
        props: { group: 'Health', headache_0to3: 0, bedding_washed_yes: 0, ibuprofen_doses: 0 },
      },
      {
        title: 'Health diary',
        start: '2026-10-13',
        props: { log: 'health', headache_0to3: 2, ibuprofen_doses: 2, bedding_washed_yes: 1 },
      },
      { title: 'Health diary', start: '2026-10-14', props: { log: 'health', headache_0to3: 0 } },
      { title: 'Health diary', start: '2026-10-20', props: { log: 'health', headache_0to3: 1 } },
    ],
  });

test('the doctor’s page leads with the counts, lists every day, and leaves out what is unticked', async () => {
  await seedMonth();
  render(HealthScreen, { def: MONTH, today: TODAY, signal: 0, onsaved: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: 'For the doctor' }));
  const page = await screen.findByRole('dialog', { name: 'for the doctor' });
  const rows = await within(page).findAllByRole('row');
  // The summary's header and three trackers, then the day table's header and the 22 days to today.
  expect(rows).toHaveLength(1 + 3 + 1 + 22);
  const headache = within(page).getByRole('rowheader', { name: 'Headache' }).closest('tr')!;
  expect(within(headache).getByText('3 of 22 days')).toBeTruthy();
  expect(within(headache).getByText('1 day')).toBeTruthy();
  expect(page.textContent).toMatch(/A record, not a diagnosis/);
  expect(page.textContent).not.toMatch(/migraine|chronic|overuse|diagnosed/i);

  await fireEvent.click(within(page).getByRole('checkbox', { name: 'Ibuprofen' }));
  expect(within(page).queryByRole('rowheader', { name: 'Ibuprofen' })).toBeNull();
  expect(within(page).queryByRole('columnheader', { name: 'Ibuprofen' })).toBeNull();

  // The CSV is the same numbers: a row a day, a column a tracker, empty where not recorded.
  let saved: Blob | null = null;
  const created = vi.fn((b: Blob) => ((saved = b), 'blob:csv'));
  vi.stubGlobal('URL', Object.assign(URL, { createObjectURL: created, revokeObjectURL: () => {} }));
  await fireEvent.click(within(page).getByRole('button', { name: 'Save as CSV' }));
  const text = await (saved as unknown as Blob).text();
  const lines = text.trim().split('\n');
  expect(lines[0]).toBe('day,bedding_washed_yes,headache_0to3,written_later');
  expect(lines).toContain('2026-10-13,1,2,1');
  expect(lines).toContain('2026-10-15,,,');
  vi.unstubAllGlobals();

  // Eight weeks is NICE's least for a headache diary: 56 days to today.
  await fireEvent.click(within(page).getByRole('button', { name: 'Last 8 weeks' }));
  await waitFor(() =>
    expect(within(page).getByText('3 of 56 days', { exact: false })).toBeTruthy(),
  );
});

test('over time draws a week at a time, says how many days each had, and marks a yes/no tracker', async () => {
  await seedMonth();
  const OVER: TotalsDef = { ...DIARY, span: 'range' };
  render(HealthScreen, { def: OVER, today: TODAY, signal: 0, onsaved: () => {} });
  // One tab to look back in: a month's grid first, a bar a week on the switch.
  await fireEvent.click(await screen.findByRole('button', { name: 'Weeks' }));
  await fireEvent.click(await screen.findByRole('button', { name: '4 weeks' }));
  // Weeks from Monday 28 Sep to the week of 19 Oct.
  await waitFor(() => expect(screen.getAllByRole('button', { name: /^Week of/ })).toHaveLength(4));
  // The first tracker in key order is shown first; a yes/no is the days it was answered yes.
  expect(screen.getByText('Days in the week answered yes')).toBeTruthy();
  await fireEvent.change(screen.getByRole('combobox', { name: 'Showing' }), {
    target: { value: 'headache_0to3' },
  });
  expect(await screen.findByText('Days in the week at mild or worse')).toBeTruthy();
  const w12 = screen.getByRole('button', { name: /^Week of .*12.*: 1, recorded on 2 of 7 days/ });
  expect(w12).toBeTruthy();
  expect(screen.getByRole('button', { name: /^Week of .*28.*: nothing recorded/ })).toBeTruthy();

  await fireEvent.change(screen.getByRole('combobox', { name: 'Mark' }), {
    target: { value: 'bedding_washed_yes' },
  });
  await fireEvent.click(w12);
  expect(screen.getByText(/bedding washed on 1 day/)).toBeTruthy();

  // A count is the times in the week.
  await fireEvent.change(screen.getByRole('combobox', { name: 'Showing' }), {
    target: { value: 'ibuprofen_doses' },
  });
  expect(await screen.findByText('Times in the week')).toBeTruthy();
});

test('guidance: a card is its sources’ words; following counts the diary beside the body’s number', async () => {
  await handle('create_notes', {
    notes: [
      {
        title: 'Health diary',
        tags: ['template'],
        props: { group: 'Health', paracetamol_doses: 0, bedding_washed_yes: 0 },
      },
      {
        title: 'Health diary',
        start: '2026-10-19',
        props: { log: 'health', paracetamol_doses: 1 },
      },
      {
        title: 'Health diary',
        start: '2026-10-20',
        props: { log: 'health', paracetamol_doses: 2 },
      },
    ],
  });
  const GUIDE: TotalsDef = { ...DIARY, span: 'plan' };
  render(HealthScreen, { def: GUIDE, today: TODAY, signal: 0, onsaved: () => {} });

  // Shelved, and badged by what each rests on.
  const shelf = await screen.findByRole('region', { name: 'Keeping a diary' });
  await fireEvent.click(within(shelf).getByRole('button', { name: /Painkiller days/ }));
  const card = await screen.findByRole('article', { name: 'Painkiller days' });
  expect(within(card).getByText('Guideline')).toBeTruthy();
  expect(card.textContent).toMatch(/Open Government Licence v3\.0/);
  expect(within(card).getByText(/does not say you have medication overuse headache/)).toBeTruthy();

  // The class of a medicine is the person's to say.
  const simple = within(card).getByRole('group', { name: /Paracetamol, aspirin or an NSAID/ });
  await fireEvent.click(within(simple).getByRole('checkbox', { name: 'Paracetamol' }));
  await fireEvent.click(within(card).getByRole('button', { name: 'Follow' }));

  const followed = await screen.findByRole('region', { name: 'followed' });
  await waitFor(() =>
    expect(within(followed).getByText('Days this week with any painkiller')).toBeTruthy(),
  );
  // Two days this week, beside the NHS's 2: within it, and said as a count, not a verdict.
  const week = within(followed).getByText('Days this week with any painkiller').closest('.check')!;
  expect(week.textContent).toMatch(/2/);
  expect(week.textContent).toMatch(/at most 2/);
  expect(within(week as HTMLElement).getByLabelText('within the number')).toBeTruthy();
  expect(followed.textContent).not.toMatch(/overuse|you have/i);

  // Stopping gives the note a last day; the card is no longer followed.
  await fireEvent.click(within(followed).getByRole('button', { name: 'Painkiller days' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Stop following' }));
  await waitFor(() => expect(screen.queryByRole('region', { name: 'followed' })).toBeNull());
});

test('the cycle: a starter that says where its notes go, lengths that only count, a gap left out', async () => {
  // The starter says, before it is added, that backups and remotes are copies.
  render(HealthScreen, { def: DIARY, today: TODAY, signal: 0, onsaved: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: 'Track something' }));
  const sheet = await screen.findByRole('dialog', { name: 'track something' });
  const cycle = within(sheet).getByRole('button', { name: /^Cycle/ });
  expect(cycle.textContent).toMatch(/backed up/);
  expect(cycle.textContent).toMatch(/Nothing is predicted/);
  await fireEvent.click(cycle);
  // Flow in the words of Apple Health and Health Connect, and a context row.
  const flow = await screen.findByRole('group', { name: 'Period flow' });
  expect(within(flow).getByRole('button', { name: 'Heavy' })).toBeTruthy();
  expect(screen.getByText('Cycle context')).toBeTruthy();
  cleanup();

  await handle('create_notes', {
    notes: [
      ['2026-07-01', 2],
      ['2026-07-29', 2],
      ['2026-08-30', 2],
      ['2026-09-28', 2],
    ].map(([start]) => ({
      title: 'Health diary',
      start,
      props: { log: 'health', period_start_yes: 1, period_flow_0to3: 2 },
    })),
  });
  await handle('create_notes', {
    notes: [
      {
        title: 'Your cycle',
        start: '2026-07-01',
        props: { program: 'health', plan: 'your-cycle' },
      },
    ],
  });
  render(HealthScreen, {
    def: { ...DIARY, span: 'plan' },
    today: TODAY,
    signal: 0,
    onsaved: () => {},
  });
  const followed = await screen.findByRole('region', { name: 'followed' });
  await waitFor(() => expect(within(followed).getByText('28, 32, 29')).toBeTruthy());
  expect(followed.textContent).toMatch(/24 to 38/);
  expect(followed.textContent).not.toMatch(/irregular|fertile|ovulat|predict/i);

  // The person says the 32-day cycle was a gap in their logging: it is left out, and says so.
  const rows = within(followed).getByRole('list', { name: 'cycles' });
  const second = within(rows).getAllByRole('listitem')[1];
  await fireEvent.click(within(second).getByRole('button', { name: 'Missed logging' }));
  await waitFor(() => expect(within(followed).getByText('28, 29')).toBeTruthy());
  expect(followed.textContent).toMatch(/left out: marked as missed logging/);
});

test('yours: a card the person writes counts their tracker against their own number, badged as theirs', async () => {
  await handle('create_notes', {
    notes: [
      {
        title: 'Health diary',
        tags: ['template'],
        props: { group: 'Health', bedding_washed_yes: 0 },
      },
      {
        title: 'Health diary',
        start: '2026-10-20',
        props: { log: 'health', bedding_washed_yes: 1 },
      },
    ],
  });
  render(HealthScreen, {
    def: { ...DIARY, span: 'plan' },
    today: TODAY,
    signal: 0,
    onsaved: () => {},
  });
  const yours = await screen.findByRole('region', { name: 'Yours' });
  await fireEvent.click(within(yours).getByRole('button', { name: /Write your own/ }));
  const form = await screen.findByRole('form', { name: 'your own card' });
  await fireEvent.input(within(form).getByRole('textbox', { name: 'Name' }), {
    target: { value: 'Wash the bedding' },
  });
  await fireEvent.change(within(form).getByRole('combobox', { name: 'Your aim (optional)' }), {
    target: { value: 'min' },
  });
  await fireEvent.click(within(form).getByRole('button', { name: 'Follow it' }));

  const followed = await screen.findByRole('region', { name: 'followed' });
  await waitFor(() =>
    expect(within(followed).getByRole('button', { name: 'Wash the bedding' })).toBeTruthy(),
  );
  expect(followed.textContent).toMatch(/Your own number/);
  expect(
    within(screen.getByRole('region', { name: 'Yours' })).getByText('Wash the bedding'),
  ).toBeTruthy();
});
