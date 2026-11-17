// **The one frame** (`decisions.md` 2026-11-17): a tab's line names its feature as a chooser that
// moves *this* tab; a new tab suggests the features, a new note and the web; Settings is a page.
import { render, screen, fireEvent } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import NewTab from './NewTab.svelte';
import * as mock from '../lib/mock';

beforeEach(() => mock.reset());
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
});

it('the feature’s name is a chooser that moves this tab', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  const moved: string[] = [];
  render(Feature, { go: (u: string) => void moved.push(u) });
  const name = await screen.findByRole('button', { name: 'switch this tab to another feature' });
  expect(name.textContent).toContain('Tasks');
  await fireEvent.click(name);
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Food journal' }));
  expect(moved).toEqual(['/f/group/Food']);
});

it('a new tab suggests the features, a new note and the web', async () => {
  const went: string[] = [];
  render(NewTab, { go: (u: string) => void went.push(u) });
  const features = screen.getByRole('list', { name: 'features' });
  const names = [...features.querySelectorAll('button')].map((b) => b.textContent?.trim());
  expect(names).toEqual([
    'New note',
    'Notes',
    'Tasks',
    'Agenda',
    'Food journal',
    'Fitness',
    'Health',
    'Music',
    'Videos',
    'Discussions',
    'Collaboration',
  ]);
  await fireEvent.click(screen.getByRole('button', { name: 'Notes' }));
  expect(went).toEqual(['/f/group/Library']);
  const field = screen.getByRole('searchbox', { name: 'a web address or a search' });
  await fireEvent.input(field, { target: { value: 'example.org' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Go to example.org' }));
  expect(went.at(-1)).toBe('https://example.org');
  await fireEvent.input(field, { target: { value: 'leafcutter ants' } });
  await fireEvent.click(
    screen.getByRole('button', { name: 'Search the web for “leafcutter ants”' }),
  );
  expect(went.at(-1)).toBe('https://duckduckgo.com/?q=leafcutter%20ants');
  await fireEvent.click(screen.getByRole('button', { name: 'New note' }));
  await vi.waitFor(() => expect(went.at(-1)).toMatch(/^\/f\/note\/[0-9A-Z]{26}$/));
});

it('Settings is a page of its own', async () => {
  history.replaceState(null, '', '/f/settings');
  render(Feature);
  expect(await screen.findByRole('heading', { name: 'Settings' }, { timeout: 4000 })).toBeTruthy();
});

it('+ offers Discussions and Collaboration among the features', async () => {
  const { render, screen } = await import('@testing-library/svelte');
  const { default: NewTab } = await import('./NewTab.svelte');
  let went = '';
  render(NewTab, { go: (u: string) => (went = u) });
  const d = screen.getByRole('button', { name: 'Discussions' });
  expect(screen.getByRole('button', { name: 'Collaboration' })).toBeTruthy();
  d.click();
  expect(went).toBe('/f/discussions');
});

it('a feature page offers a newer version, as the old whole-app page did', async () => {
  const { render, screen } = await import('@testing-library/svelte');
  const { default: Feature } = await import('./Feature.svelte');
  const mock = await import('../lib/mock');
  mock.reset();
  history.replaceState(null, '', '/f/group/Library');
  render(Feature);
  // The stand-in's server has v0.6.1 waiting.
  expect(await screen.findByRole('button', { name: 'Update' }, { timeout: 4000 })).toBeTruthy();
  expect(screen.getAllByText(/v0\.6\.1/).length).toBeGreaterThan(0);
});
