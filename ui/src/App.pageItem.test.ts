// **A web page is an item of the collection on the phone** (`workspace-design` §13.7): a web link
// opens a page item, which the shell draws over the item's box (`__fmPage.show`); the bar's switch
// goes back to the note before it; ⇥ brings the page's selection into that note, quoted with the
// page's exact address; closing the item closes the shell's page. The shell is stood in for by fake
// `__fmPage` and `__fmShare`, shaped as `MainActivity.kt`'s.
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';

let calls: string[][] = [];
let brought: unknown[] = [];

beforeEach(() => {
  mock.reset();
  calls = [];
  brought = [];
  const g = globalThis as Record<string, unknown>;
  g.__fmPage = {
    show: (id: string, url: string) => calls.push(['show', id, url]),
    hide: (id: string) => calls.push(['hide', id]),
    close: (id: string) => calls.push(['close', id]),
    back: (id: string) => calls.push(['back', id]),
    bring: (id: string, note: string) => calls.push(['bring', id, note]),
  };
  g.__fmShare = {
    pending: () => JSON.stringify({ files: [], refused: [], texts: [], brought }),
    read: () => '',
    done: () => {},
    doneTexts: () => {},
    doneBrought: (n: number) => (brought = brought.slice(n)),
  };
  // jsdom lays nothing out: the page's box is given a size, as a phone's screen would.
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    return this.classList?.contains('page-box')
      ? ({
          left: 0,
          top: 40,
          width: 360,
          height: 600,
          right: 360,
          bottom: 640,
          x: 0,
          y: 40,
        } as DOMRect)
      : ({ left: 0, top: 0, width: 0, height: 0, right: 0, bottom: 0, x: 0, y: 0 } as DOMRect);
  });
});
afterEach(() => {
  const g = globalThis as Record<string, unknown>;
  delete g.__fmPage;
  delete g.__fmShare;
  vi.restoreAllMocks();
});

it('a link opens a page item drawn by the shell; the switch goes back; ⇥ brings into the note', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Reading', body: 'Start. [Ants](https://example.org/ants)', props: {} }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  (await screen.findByRole('link', { name: 'Ants' })).click();

  await waitFor(() =>
    expect(calls.some((c) => c[0] === 'show' && c[2] === 'https://example.org/ants')).toBe(true),
  );
  const pageId = calls.find((c) => c[0] === 'show')![1];

  // The page names its item once it has loaded.
  window.dispatchEvent(
    new CustomEvent('fm-page', {
      detail: { id: pageId, title: 'All about ants', url: 'https://example.org/ants' },
    }),
  );
  expect((await screen.findAllByText('All about ants')).length).toBeGreaterThan(0);

  // ⇥ asks the shell for the selection, for the note before the page.
  await fireEvent.click(screen.getByRole('button', { name: 'bring the selection into a note' }));
  expect(calls).toContainEqual(['bring', pageId, made.id]);

  // What the shell brings back lands in that note, with the page's exact address.
  brought = [
    {
      kind: 'selection',
      html: '<p>Ants farm fungus.</p>',
      text: 'Ants farm fungus.',
      pageUrl: 'https://example.org/ants',
      pageTitle: 'All about ants',
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

it('the bar switches between the page and the note before it, and closing the page closes it', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Reading', body: '[Ants](https://example.org/ants)', props: {} }],
  })) as { id: string }[];
  location.hash = `#note=${made.id}`;
  render(App);
  (await screen.findByRole('link', { name: 'Ants' })).click();
  await waitFor(() => expect(calls.some((c) => c[0] === 'show')).toBe(true));
  const pageId = calls.find((c) => c[0] === 'show')![1];

  await fireEvent.click(await screen.findByRole('button', { name: 'switch to Reading' }));
  expect(await screen.findByRole('button', { name: /^switch to example\.org/ })).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: /^switch to example\.org/ }));
  await fireEvent.click(await screen.findByRole('button', { name: /open notes/ }));
  await fireEvent.click(screen.getByRole('menuitem', { name: 'close example.org' }));
  expect(calls).toContainEqual(['close', pageId]);
});
