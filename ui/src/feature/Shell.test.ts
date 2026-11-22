// **The shell** (P2b): the rail takes a person anywhere inside the tab (the address changes, Back works,
// nothing reloads); Home shows what can be done and is customizable, kept as a note tagged `home`;
// Today shows the day; a note opens beside the page on a wide screen.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Shell from './Shell.svelte';
import * as mock from '../lib/mock';

const wide = (on: boolean) =>
  vi.stubGlobal('matchMedia', (q: string) => ({
    matches: on && q.includes('min-width'),
    addEventListener: () => {},
    removeEventListener: () => {},
  }));

beforeEach(() => {
  mock.reset();
  wide(true);
});
afterEach(() => {
  history.replaceState(null, '', '/');
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it('the rail goes from Home to a section and back, in the same tab', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const rail = await screen.findByRole('navigation', { name: 'formicaria' });
  expect(await screen.findByRole('heading', { name: 'Home' })).toBeTruthy();
  await fireEvent.click(within(rail).getByRole('button', { name: /^Tasks/ }));
  await vi.waitFor(() => expect(location.pathname).toBe('/f/group/Tasks'));
  // The same rail: nothing reloaded.
  expect(screen.getByRole('navigation', { name: 'formicaria' })).toBe(rail);
  expect(
    within(rail)
      .getByRole('button', { name: /^Tasks/ })
      .getAttribute('aria-current'),
  ).toBe('page');
  history.back();
  await vi.waitFor(() => expect(location.pathname).toBe('/f/home'));
});

it('Home lists the sections with ＋, and its blocks', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const kinds = await screen.findByRole('list', { name: 'what you can do here' });
  expect(within(kinds).getByText(/Food journal/)).toBeTruthy();
  expect(within(kinds).getByRole('button', { name: 'new in Music' })).toBeTruthy();
  const blocks = screen.getByRole('list', { name: 'home blocks' });
  for (const name of ['Getting started', 'Today', 'Just in', 'Recent', 'Pinned'])
    expect(within(blocks).getByRole('heading', { name: new RegExp(name) })).toBeTruthy();
});

it('Customize home: a block removed and one moved are kept, in a note tagged home', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  await fireEvent.click(await screen.findByRole('button', { name: 'Customize home' }));
  await fireEvent.click(screen.getByRole('button', { name: 'remove Pinned' }));
  await vi.waitFor(async () => {
    const home = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    expect(home.length).toBe(1);
    const d = (await mock.handle('get', { id: home[0].id })) as { body: string };
    expect(d.body).not.toContain('Pinned');
    expect(d.body).toContain('view:today');
  });
  await fireEvent.click(screen.getByRole('button', { name: 'move Today up' }));
  await vi.waitFor(async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    const d = (await mock.handle('get', { id: h.id })) as { body: string };
    expect(d.body.indexOf('view:today')).toBeLessThan(d.body.indexOf('view:start'));
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Reset to the default' }));
  await vi.waitFor(async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    const d = (await mock.handle('get', { id: h.id })) as { body: string };
    expect(d.body).toContain('Pinned');
  });
});

it('Today shows the day in its four parts', async () => {
  history.replaceState(null, '', '/f/today');
  render(Shell);
  for (const name of ['Planned and due', 'Logged', 'Came in', 'Written'])
    expect(await screen.findByRole('region', { name })).toBeTruthy();
});

it('a note opens beside the page on a wide screen', async () => {
  const [made] = (await mock.handle('create_notes', {
    notes: [{ title: 'Beside me', body: 'Words beside.', props: { status: 'todo' } }],
  })) as { id: string }[];
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  const card = await vi.waitFor(() => {
    const c = [...document.querySelectorAll('.card')].find((e) =>
      e.textContent?.includes('Beside me'),
    );
    if (!c) throw new Error('no card');
    return c as HTMLElement;
  });
  await fireEvent.click(card);
  const beside = await screen.findByRole('complementary', { name: 'the note beside' });
  expect(await within(beside).findByText('Words beside.')).toBeTruthy();
  void made;
});

it('＋ New in the rail makes a board, and opens it beside', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const rail = await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.click(within(rail).getByRole('button', { name: '＋ New' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'New board' }));
  await vi.waitFor(async () => {
    const all = (await mock.handle('recent', {})) as {
      title: string | null;
      props: Record<string, unknown>;
    }[];
    expect(all.some((n) => n.props?.view === 'board')).toBe(true);
  });
  expect(await screen.findByRole('complementary', { name: 'the note beside' })).toBeTruthy();
});

it('⋯ → Bring into another note: the passage goes into the note chosen', async () => {
  const [target] = (await mock.handle('create_notes', {
    notes: [{ title: 'Collected quotes', body: 'Start.\n', props: {} }],
  })) as { id: string }[];
  history.replaceState(null, '', '/f/home');
  render(Shell);
  await screen.findByRole('navigation', { name: 'formicaria' });
  window.dispatchEvent(
    new CustomEvent('fm-bring-to-room', {
      detail: { md: '> Ants farm fungus.', source: { title: 'Field notes' } },
    }),
  );
  const sheet = await screen.findByRole('dialog', { name: 'add what was shared to a note' });
  const list = await within(sheet).findByRole('list', { name: 'recent notes' });
  await fireEvent.click(await within(list).findByRole('button', { name: 'Collected quotes' }));
  await vi.waitFor(async () => {
    const d = (await mock.handle('get', { id: target.id })) as { body: string };
    expect(d.body).toContain('Ants farm fungus.');
  });
});

it('Settings in the shell is a place: no close of its own', async () => {
  history.replaceState(null, '', '/f/settings');
  render(Shell);
  await screen.findByRole('dialog', { name: 'settings' });
  expect(screen.queryByRole('button', { name: 'close' })).toBeNull();
});
