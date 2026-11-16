// **Placing a note, and handing it out** (`workspace-design` §13.4, §13.6): *Place on a shelf…* puts
// a note on a shelf; *Send by mail…* opens Gmail's compose with the
// note's title and text. Both in the note's ＋ (Options).
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

beforeEach(() => mock.reset());
afterEach(() => vi.restoreAllMocks());

async function openNew(title: string, body: string, props: Record<string, string> = {}) {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title, body, props, tags: ['web'] }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'note options' }));
  return made.id;
}

it('a note placed on a new shelf carries that shelf', async () => {
  const id = await openNew('Saved page', 'Text.', { source: 'web' });
  await fireEvent.click(screen.getByRole('button', { name: 'Place on a shelf…' }));
  const input = await screen.findByRole('textbox', { name: 'new shelf' });
  await fireEvent.input(input, { target: { value: 'Ants' } });
  await fireEvent.keyDown(input, { key: 'Enter' });
  await waitFor(async () => {
    const n = (await mock.handle('get', { id })) as {
      tags: string[];
      props: Record<string, unknown>;
    };
    expect(n.tags).toContain('shelf:Ants');
  });
});

it('Send by mail opens Gmail compose with the title and the text', async () => {
  const open = vi.spyOn(window, 'open').mockImplementation(() => null);
  await openNew('Lab visit', 'Tuesday works.');
  await fireEvent.click(screen.getByRole('button', { name: 'Send by mail…' }));
  expect(open).toHaveBeenCalledOnce();
  const u = new URL(open.mock.calls[0][0] as string);
  expect(u.host).toBe('mail.google.com');
  expect(u.searchParams.get('su')).toBe('Lab visit');
  expect(u.searchParams.get('body')).toContain('Tuesday works.');
});
