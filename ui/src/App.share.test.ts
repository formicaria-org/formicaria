// A picture shared from another app on the phone (Android's Share → formicaria) is placed in a
// note the person chooses: the note open now, a new note, or any other (`decisions.md#ui`,
// 2026-10-20). The shell is stood in for by a fake `__fmShare`, shaped as `MainActivity.kt`'s.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

const PNG = btoa(String.fromCharCode(137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3));
let held: { name: string; mime: string }[] = [];
let refused: { name: string; why: string }[] = [];
let doneCalls = 0;

function share(...names: string[]) {
  held = names.map((name) => ({ name, mime: 'image/png' }));
  window.dispatchEvent(new Event('fm-share'));
}

beforeEach(() => {
  mock.reset();
  held = [];
  refused = [];
  doneCalls = 0;
  (globalThis as { __fmShare?: unknown }).__fmShare = {
    pending: () => JSON.stringify({ files: held, refused }),
    read: () => PNG,
    done: (_f: number, _r: number) => {
      held = [];
      refused = [];
      doneCalls++;
    },
  };
});
afterEach(() => {
  delete (globalThis as { __fmShare?: unknown }).__fmShare;
});

/** Some open editor holds exactly this (the test app may have other notes open). */
const holds = (re: RegExp) =>
  waitFor(() => {
    return expect(
      screen
        .getAllByLabelText('note body (Markdown)')
        .some((el) => re.test((el as HTMLTextAreaElement).value)),
    ).toBe(true);
  });

describe('a picture shared from another app', () => {
  // Seen on the emulator: the sharing app gave no permission, so nothing was taken. The sheet says
  // so, under a heading that does not offer to add nothing.
  it('says plainly when nothing could be taken', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    refused = [{ name: 'IMG_1.png', why: 'could not be read' }];
    share();
    const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
    expect(within(sheet).getByRole('heading').textContent).toBe('Nothing could be added');
    expect(within(sheet).getByRole('alert').textContent).toContain('IMG_1.png (could not be read)');
    expect(within(sheet).queryByRole('button', { name: /New note/ })).toBeNull();
    await fireEvent.click(within(sheet).getByRole('button', { name: 'Close' }));
  });

  it('goes into a new note when that is chosen', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    share('Screenshot_20261020.png');
    const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
    expect(within(sheet).getByRole('heading').textContent).toBe('Add the picture to…');
    expect(doneCalls).toBe(1);
    await fireEvent.click(within(sheet).getByRole('button', { name: /New note/ }));
    // The test backend gives a blank new note a sample text; the picture goes at its end.
    await holds(/\n!\[Screenshot_20261020\.png\]\(asset:sha256-[0-9a-f]+\)\n$/);
  });

  it('offers the note open now first, and adds at its end', async () => {
    const ids = mock.seed({ notes: 2 });
    await mock.handle('update_body', { id: ids[0], body: 'my notes', base: '' });
    render(App);
    await fireEvent.click(await screen.findByText(/Seeded 0/));
    share('a.png', 'b.png');
    const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
    expect(within(sheet).getByRole('heading').textContent).toBe('Add 2 pictures to…');
    const first = await within(sheet).findByText('the note open now');
    await fireEvent.click(first.closest('button')!);
    await holds(
      /^my notes\n!\[a\.png\]\(asset:sha256-[0-9a-f]+\)\n!\[b\.png\]\(asset:sha256-[0-9a-f]+\)\n$/,
    );
  });

  it('says why one was not taken, and Cancel adds nothing', async () => {
    render(App);
    await screen.findByRole('button', { name: 'make something new' });
    refused = [{ name: 'film.png', why: 'too large' }];
    share('small.png');
    const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
    expect(within(sheet).getByRole('alert').textContent).toContain('film.png (too large)');
    await fireEvent.click(within(sheet).getByRole('button', { name: 'Cancel' }));
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'add what was shared to a note' })).toBeNull(),
    );
    expect(
      screen
        .queryAllByLabelText('note body (Markdown)')
        .some((el) => (el as HTMLTextAreaElement).value.includes('small.png')),
    ).toBe(false);
  });
});
