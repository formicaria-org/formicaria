// **A piece for a note that is not on screen goes in by `put`** (`actions.rs`; `decisions.md`
// 2026-11-17): chosen by the icon's chooser on a page, it lands in the note's file without opening it,
// so the person stays on the page; a note on screen still takes it through its pane.
import { afterEach, beforeEach, expect, it } from 'vitest';
import * as mock from '../mock';
import { listen, toast } from './bridge.svelte';

let opened: string[] = [];
let stop: () => void = () => {};
const replies: unknown[] = [];

beforeEach(() => {
  mock.reset();
  opened = [];
  replies.length = 0;
  addEventListener('message', (e) => {
    const r = (e.data as { formicaria_reply?: unknown })?.formicaria_reply;
    if (r) replies.push(r);
  });
});
afterEach(() => stop());

function send(m: Record<string, unknown>) {
  window.dispatchEvent(
    new MessageEvent('message', {
      data: { formicaria_bridge: m },
      origin: location.origin,
      source: window,
    }),
  );
}

it('goes into a note open nowhere, without opening it', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Ants', body: 'Mine.', props: {} }],
  })) as { id: string }[];
  stop = listen({
    open: (id) => opened.push(id),
    target: () => null,
    onScreen: () => false,
    create: async () => null,
    ask: async () => null,
  });
  send({
    n: 1,
    type: 'insert',
    text: '> Leaves are carried home.',
    what: 'the passage',
    page: { title: 'A walk', url: 'https://example.org/a' },
    quiet: false,
    note: made.id,
    noteTitle: 'Ants',
  });
  await new Promise((r) => setTimeout(r, 50));
  const n = (await mock.handle('get', { id: made.id })) as { body: string };
  expect(n.body).toBe('Mine.\n\n> Leaves are carried home.\n');
  expect(opened).toEqual([]);
  expect(toast.now?.text).toBe('Brought the passage into “Ants”');
  expect(replies).toContainEqual(expect.objectContaining({ n: 1, ok: true, note: made.id }));
});
