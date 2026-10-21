// **Health on a phone** (`decisions.md#ui`, 2026-10-22): the same screens, laid out for a narrow
// window by width, and one thing that differs by device. Android's WebView prints and downloads
// only when the shell handles them, and formicaria's does not, so the doctor's page offers to keep
// itself as a note there instead.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test } from 'vitest';
import HealthScreen from './HealthScreen.svelte';
import { asDesktop, asPhone } from '../harness';
import { handle, reset } from '../mock';
import * as nav from '../nav.svelte';
import { clear as clearUndo } from '../undo.svelte';
import type { ObjectMeta, TotalsDef } from '../types';

const OVER: TotalsDef = { log: 'health', add: 'health', span: 'range', metrics: [], slots: [] };
const TODAY = '2026-10-22';

beforeEach(() => void asPhone());
afterEach(() => {
  asDesktop();
  reset();
  clearUndo();
  nav.reset();
});

test('on a phone the page for the doctor is kept as a note, not printed or downloaded', async () => {
  await handle('create_notes', {
    notes: [
      { title: 'Health diary', tags: ['template'], props: { group: 'Health', headache_0to3: 0 } },
      { title: 'Health diary', start: '2026-10-13', props: { log: 'health', headache_0to3: 2 } },
    ],
  });
  render(HealthScreen, { def: OVER, today: TODAY, signal: 0, onsaved: () => {} });
  await fireEvent.click(await screen.findByRole('button', { name: 'For the doctor' }));
  const page = await screen.findByRole('dialog', { name: 'for the doctor' });
  await within(page).findByRole('rowheader', { name: 'Headache' });
  expect(within(page).queryByRole('button', { name: 'Print' })).toBeNull();
  expect(within(page).queryByRole('button', { name: 'Save as CSV' })).toBeNull();

  await fireEvent.click(within(page).getByRole('button', { name: 'Keep as a note' }));
  await within(page).findByRole('status');
  const notes = await handle<ObjectMeta[]>('recent', { limit: 50 });
  const kept = notes.find((n) => (n.title ?? '').startsWith('Health diary, '));
  expect(kept?.tags).toContain('health');
  const body = (await handle<{ body: string }>('get', { id: kept!.id })).body;
  expect(body).toMatch(/\| Headache \| 1 of 22 days \| 1 day \| 1 day \|/);
  expect(body).toMatch(/A record, not a diagnosis/);
  await waitFor(() =>
    expect(within(page).getByRole('status').textContent).toMatch(/Kept as a note/),
  );
});
