// **Mail is your webmail** (`lib/webmail.ts`, `decisions.md` 2026-11-17): the views list's *Mail*
// opens it (Gmail unless changed); on the phone as a page inside formicaria, elsewhere in a tab.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import App from './App.svelte';
import * as mock from './lib/mock';
import { setWebmail, webmail, DEFAULT_WEBMAIL } from './lib/webmail';

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
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('Mail opens the webmail, Gmail unless another is set', async () => {
  const opened = vi.spyOn(window, 'open').mockImplementation(() => null);
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Mail' }));
  expect(opened).toHaveBeenCalledWith(DEFAULT_WEBMAIL, '_blank', 'noopener');
});

it('the webmail is an https address the person chose, or Gmail', () => {
  expect(webmail()).toBe(DEFAULT_WEBMAIL);
  expect(setWebmail('javascript:alert(1)')).toBe(false);
  expect(setWebmail('https://app.fastmail.com/')).toBe(true);
  expect(webmail()).toBe('https://app.fastmail.com/');
});
