import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { applyTheme, cannotStart, toggledTheme } from './start';

// jsdom here has no `localStorage` (why every read of it in the app is guarded): a small one in memory,
// and one that refuses, as private mode does.
const kept = new Map<string, string>();
const memory = {
  getItem: (k: string) => kept.get(k) ?? null,
  setItem: (k: string, v: string) => void kept.set(k, String(v)),
  removeItem: (k: string) => void kept.delete(k),
  clear: () => kept.clear(),
};
const refusing = {
  getItem: () => {
    throw new Error('private mode');
  },
  setItem: () => {
    throw new Error('quota');
  },
};

beforeEach(() => {
  kept.clear();
  vi.stubGlobal('localStorage', memory);
});
afterEach(() => vi.unstubAllGlobals());

describe("a page's start", () => {
  it('paints the saved theme before anything is drawn', () => {
    kept.set('fm-theme', 'light');
    applyTheme();
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('falls back to dark when storage refuses', () => {
    vi.stubGlobal('localStorage', refusing);
    applyTheme();
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('says why it could not start, in words on the screen', () => {
    const box = document.createElement('div');
    box.textContent = 'half-drawn';
    cannotStart(box, new Error('no vaults'));
    expect(box.querySelector('[role=alert]')?.textContent).toBe(
      'formicaria could not start: no vaults',
    );
  });
});

it('toggles the theme, keeping it, and still wears it when storage refuses', () => {
  expect(toggledTheme('dark')).toBe('light');
  expect(kept.get('fm-theme')).toBe('light');
  vi.stubGlobal('localStorage', refusing);
  expect(toggledTheme('light')).toBe('dark');
  expect(document.documentElement.dataset.theme).toBe('dark');
});
