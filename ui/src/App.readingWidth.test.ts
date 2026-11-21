// **Reading width** (`decisions.md` 2026-11-21 (evening)): Settings → Look → Reading width sets how
// wide a note's text runs, in characters of text; a note's ＋ → Full width gives that note alone the
// pane's width; the choice is this browser's, and an open note follows it at once.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import {
  WIDTHS,
  fullWidth,
  measureFor,
  readingWidth,
  resetReadingWidth,
  setFullWidth,
  setReadingWidth,
} from './lib/readingWidth.svelte';

const store = new Map<string, string>();
beforeEach(() => {
  mock.reset();
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  resetReadingWidth();
});
afterEach(() => vi.unstubAllGlobals());

it('each step is a number of characters of text, Comfortable 75 by default, Full none', () => {
  expect(readingWidth()).toBe('comfortable');
  expect(measureFor('n1')).toBe('75ch');
  const got = WIDTHS.map((w) => {
    setReadingWidth(w.value);
    return measureFor('n1');
  });
  expect(got).toEqual(['60ch', '75ch', '100ch', 'none']);
  expect(store.get('fm-reading-width')).toBe('full');
});

it('one note at the full width leaves the others at the setting, and is kept on the device', () => {
  setFullWidth('n1', true);
  expect(measureFor('n1')).toBe('none');
  expect(measureFor('n2')).toBe('75ch');
  expect(JSON.parse(store.get('fm-full-width')!)).toEqual(['n1']);
  setFullWidth('n1', false);
  expect(fullWidth('n1')).toBe(false);
  expect(measureFor('n1')).toBe('75ch');
});

async function openANote(): Promise<HTMLElement> {
  await fireEvent.click(await screen.findByRole('button', { name: /make something new/i }));
  await fireEvent.click(await screen.findByText('New note'));
  await screen.findByRole('button', { name: 'close note' });
  return document.querySelector('article.panel') as HTMLElement;
}

it('a note’s ＋ → Full width widens that note, and Readable width puts it back', async () => {
  render(App);
  const pane = await openANote();
  expect(pane.style.getPropertyValue('--read-max')).toBe('calc(75ch + 2 * var(--space-5))');
  await fireEvent.click(screen.getByRole('button', { name: 'note options' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Full width' }));
  await vi.waitFor(() => expect(pane.style.getPropertyValue('--read-max')).toBe('none'));
  await fireEvent.click(screen.getByRole('button', { name: 'note options' }));
  const back = await screen.findByRole('button', { name: 'Readable width' });
  expect(back.getAttribute('aria-pressed')).toBe('true');
  await fireEvent.click(back);
  await vi.waitFor(() =>
    expect(pane.style.getPropertyValue('--read-max')).toBe('calc(75ch + 2 * var(--space-5))'),
  );
});

it('Settings → Look → Reading width changes an open note at once', async () => {
  render(App);
  const pane = await openANote();
  setReadingWidth('wide'); // what the radio does; the radio itself is pressed below
  await vi.waitFor(() =>
    expect(pane.style.getPropertyValue('--read-max')).toBe('calc(100ch + 2 * var(--space-5))'),
  );
  await fireEvent.click(await screen.findByRole('button', { name: /settings/i }));
  const list = await screen.findByLabelText('reading width', {}, { timeout: 4000 });
  const narrow = within(list).getByLabelText(/Narrow/);
  await fireEvent.click(narrow);
  expect(readingWidth()).toBe('narrow');
  expect(store.get('fm-reading-width')).toBe('narrow');
});
