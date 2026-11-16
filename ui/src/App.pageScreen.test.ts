// **The phone's browser, phase A** (`workspace-design` §12): a web link opens the page screen with
// the note in front as where *Bring in* goes, and what the page screen brings back lands in that
// note, quoted with the page's exact address. The shell is stood in for by fake `__fmPage` and
// `__fmShare`, shaped as `MainActivity.kt`'s.
import { render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

let opened: string[][] = [];
let brought: unknown[] = [];

beforeEach(() => {
  mock.reset();
  opened = [];
  brought = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = { open: (u: string, n: string, t: string) => opened.push([u, n, t]) };
  g.__fmShare = {
    pending: () => JSON.stringify({ files: [], refused: [], texts: [], brought }),
    read: () => '',
    done: () => {},
    doneTexts: () => {},
    doneBrought: (n: number) => (brought = brought.slice(n)),
  };
});
afterEach(() => {
  const g = globalThis as Record<string, unknown>;
  delete g.__fmPage;
  delete g.__fmShare;
});

it('a link opens the page screen, and a selection brought back lands in the note', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Reading', body: 'Start. [Ants](https://example.org/ants)', props: {} }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  const link = await screen.findByRole('link', { name: 'Ants' });
  link.click();
  expect(opened[0]?.[0]).toBe('https://example.org/ants');

  brought = [
    {
      kind: 'selection',
      html: '<p>Ants farm fungus.</p>',
      text: 'Ants farm fungus.',
      pageUrl: 'https://example.org/ants',
      pageTitle: 'Ants',
      note: made.id,
    },
  ];
  window.dispatchEvent(new Event('fm-share'));
  await waitFor(
    async () => {
      const n = (await mock.handle('get', { id: made.id })) as { body: string };
      expect(n.body).toContain('> Ants farm fungus.');
      expect(n.body).toContain('(https://example.org/ants#:~:text=Ants%20farm%20fungus.)');
    },
    { timeout: 4000 },
  );
});
