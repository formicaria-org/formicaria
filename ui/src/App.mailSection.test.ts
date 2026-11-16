// **Mail as its own section, as a client** (workspace-design §12, #2): a conversation offers Reply in
// Gmail and *Bring into a study room*; the passage goes into the note chosen, quoted, with a link back
// to the conversation. The conversation itself stays where it is.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());

it('a passage of a conversation is brought into a study room, with a link back', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [
      {
        title: 'Lab visit',
        body: '### Ada, 2026-11-10 09:00\n\nShall we meet Tuesday?\n\n### Ben, 2026-11-11 10:00\n\nTuesday works.\n',
        props: {
          gmail_thread: 't-9',
          mail_link: 'https://mail.google.com/mail/u/0/#all/t-9',
          mail_last: '2026-11-11 10:00',
        },
      },
    ],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  const bring = await screen.findByRole('button', { name: 'bring into a study room' });
  expect(screen.getByRole('button', { name: 'reply in Gmail' })).toBeTruthy();
  await fireEvent.click(bring);
  const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
  await fireEvent.click(within(sheet).getByRole('button', { name: /New note/ }));
  await waitFor(
    async () => {
      const hits = (await mock.handle('recent', {})) as { id: string }[];
      const bodies = await Promise.all(
        hits.map(async (h) => ((await mock.handle('get', { id: h.id })) as { body: string }).body),
      );
      expect(
        bodies.some((b) => b.includes('> Tuesday works.') && b.includes(`(note:${made.id})`)),
      ).toBe(true);
    },
    { timeout: 4000 },
  );
});
