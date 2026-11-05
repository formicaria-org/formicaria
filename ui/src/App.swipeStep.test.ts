// **A step between a group's views asks for that view and no more** (the owner, 2026-11-05: *"an
// awkward pause with the name of the next subview in the center before reaching it"*;
// `decisions.md#ui`). Every one of these was a read in the queue in front of the view being waited for.
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App.svelte';
import { clearFaults, reset, seedTrackerDemo } from './lib/mock';
import * as nav from './lib/nav.svelte';
import * as trackers from './lib/trackers.svelte';

const asks = vi.hoisted(() => [] as { cmd: string; args: Record<string, unknown> }[]);
vi.mock('./lib/mock', async (original) => {
  const real = await original<typeof import('./lib/mock')>();
  return {
    ...real,
    handle: (cmd: string, args: Record<string, unknown>) => {
      asks.push({ cmd, args });
      return real.handle(cmd, args);
    },
  };
});

const store = new Map<string, string>();
beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
    clear: () => store.clear(),
    key: () => null,
    length: 0,
  });
  clearFaults();
  nav.reset();
  store.set('fm-trackers', '1');
  trackers.reload();
  seedTrackerDemo();
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
  reset();
});

const openFood = async () => {
  const rail = await screen.findByRole('navigation', { name: 'views' });
  await fireEvent.click(await within(rail).findByRole('button', { name: 'open Food' }));
  return screen.findByRole('group', { name: 'Food' });
};
const tab = (name: string) =>
  within(screen.getByRole('group', { name: 'Food' })).getByRole('button', { name });
const trendsDrawn = () => screen.findAllByRole('button', { name: /^\w{3}, \w{3} \d+: / });

test('the view being left asks for nothing again, and nothing else is read on the way', async () => {
  render(App);
  await openFood();
  await waitFor(() => expect(asks.some((a) => a.cmd === 'log_days')).toBe(true));
  await new Promise((r) => setTimeout(r, 50));
  asks.length = 0;
  await fireEvent.click(tab('Trends'));
  await trendsDrawn();
  const said = asks.map((a) => a.cmd);
  expect(said).toContain('run_view');
  expect(said).not.toContain('activity');
  expect(said).not.toContain('thread_roots');
  const days = asks.filter((a) => a.cmd === 'log_days').map((a) => a.args.view);
  expect(days).not.toContain('Day');
});

test('a view just left is there at once on the way back, its feed read again behind it', async () => {
  render(App);
  await openFood();
  await fireEvent.click(tab('Trends'));
  await trendsDrawn();
  asks.length = 0;
  await fireEvent.click(tab('Day'));
  // No hold: what was kept is drawn in this very step, not the view being left.
  expect(document.querySelector<HTMLElement>('.tracker .screen')?.inert).toBe(false);
  expect(tab('Day').getAttribute('aria-current')).toBe('page');
  await waitFor(() =>
    expect(asks.some((a) => a.cmd === 'run_view' && a.args.name === 'Day')).toBe(true),
  );
});

// What a tracker hands across a change of view (`carried`) relies on its being built anew, and a
// feed already here means no hold for that to happen at the end of.
test('a tracker is built anew for each view, though its feed was already here', async () => {
  render(App);
  await openFood();
  await fireEvent.click(tab('Trends'));
  await trendsDrawn();
  await fireEvent.click(tab('Day'));
  await waitFor(() =>
    expect(document.querySelector('.tracker .screen')?.getAttribute('inert')).toBeNull(),
  );
  const before = document.querySelector('.tracker');
  await fireEvent.click(tab('Trends'));
  expect(document.querySelector('.tracker')).not.toBe(before);
});
