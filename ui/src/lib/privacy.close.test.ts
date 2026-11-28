// **Forget when closing, on the phone** (2026-11-28): when the app starts again, and when it comes back
// after five minutes or more away; not after a shorter look elsewhere, and never while it is off.
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
type Mock = typeof import('./mock');
let mock: Mock;

const g = globalThis as Record<string, unknown>;
let told: { fired: number }[] = [];
const fired = async () => ((await mock.handle('privacy', {})) as { fired: number }).fired;
function visibility(state: 'hidden' | 'visible') {
  Object.defineProperty(document, 'visibilityState', { value: state, configurable: true });
  document.dispatchEvent(new Event('visibilitychange'));
}

beforeEach(async () => {
  // A fresh module each time (it watches once), and the mock it reaches.
  vi.resetModules();
  mock = await import('./mock');
  mock.reset();
  told = [];
  g.__fmPage = { privacy: (j: string) => told.push(JSON.parse(j)) };
});
afterEach(() => {
  delete g.__fmPage;
  visibility('visible');
});

it('off (the default): nothing is forgotten, on starting or coming back', async () => {
  const { forgetOnClosing } = await import('./privacy.svelte');
  let t = 0;
  forgetOnClosing(() => t);
  await vi.waitFor(() => expect(told.length).toBe(1));
  visibility('hidden');
  t += 60 * 60 * 1000;
  visibility('visible');
  await new Promise((r) => setTimeout(r, 20));
  expect(await fired()).toBe(0);
});

it('on: forgotten when the app starts, again after five minutes away, not after one', async () => {
  await mock.handle('set_privacy', { fire_on_close: true });
  const { forgetOnClosing, AWAY_MS } = await import('./privacy.svelte');
  let t = 0;
  forgetOnClosing(() => t);
  await vi.waitFor(async () => expect(await fired()).toBe(1));
  // The pages were told the first reading before the press, so they act on it.
  await vi.waitFor(() => expect(told.map((x) => x.fired)).toEqual([0, 1]));
  visibility('hidden');
  t += 60 * 1000;
  visibility('visible');
  await new Promise((r) => setTimeout(r, 20));
  expect(await fired()).toBe(1);
  visibility('hidden');
  t += AWAY_MS;
  visibility('visible');
  await vi.waitFor(async () => expect(await fired()).toBe(2));
});
