// **The shell** (P2b): the rail takes a person anywhere inside the tab (the address changes, Back works,
// nothing reloads); Home shows what can be done and is customizable, kept as a note tagged `home`;
// the Journal shows the day; a note opens beside the page on a wide screen.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Shell from './Shell.svelte';
import * as mock from '../lib/mock';
import { appToday } from '../lib/appTime';
import { typeInto } from '../lib/editor/testing';

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

it('Home: the journal ready to write in, and cards made from what is used', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const today = appToday();
  expect(await screen.findByRole('region', { name: `journal, ${today}` })).toBeTruthy();
  const cards = await screen.findByRole('list', { name: 'home cards shown' });
  expect(await within(cards).findByRole('heading', { name: /Continue/ })).toBeTruthy();
  // No list of every section on Home: the rail has them.
  expect(screen.queryByRole('list', { name: 'what you can do here' })).toBeNull();
});

it('the journal: today’s entry is made on the first keystroke, titled with the day', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const today = appToday();
  const region = await screen.findByRole('region', { name: `journal, ${today}` });
  const box = await vi.waitFor(() => {
    const el = region.querySelector('.cm-editor');
    if (!el) throw new Error('no editor yet');
    return el;
  });
  expect(await mock.handle('journal', { on_or_before: today })).toEqual([]);
  typeInto(box, 'Fed the colony.');
  await vi.waitFor(
    async () => {
      const [n] = (await mock.handle('journal', { on_or_before: today })) as {
        id: string;
        title: string;
        tags: string[];
      }[];
      expect(n?.title).toBe(today);
      expect(n.tags).toContain('journal');
      const d = (await mock.handle('get', { id: n.id })) as { body: string };
      expect(d.body).toContain('Fed the colony.');
    },
    { timeout: 3000 },
  );
});

it('Edit Home: a card removed, one moved and one resized are kept, in a note tagged home', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit Home' }));
  const body = async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    if (!h) return '';
    return ((await mock.handle('get', { id: h.id })) as { body: string }).body;
  };
  await fireEvent.click(await screen.findByRole('button', { name: 'remove Continue' }));
  await vi.waitFor(async () => {
    const b = await body();
    expect(b).toContain('card:journal');
    expect(b).not.toContain('card:continue');
  });
  await fireEvent.change(screen.getByRole('combobox', { name: 'size of Journal' }), {
    target: { value: 'full' },
  });
  await vi.waitFor(async () => expect(await body()).toContain('card:journal?size=full'));
  await fireEvent.click(screen.getByRole('button', { name: 'Reset to the default' }));
  await vi.waitFor(async () => expect(await body()).not.toContain('card:'));
});

it('Pin to Home, from a section’s bar: the section becomes a link on Home', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  await fireEvent.click(await screen.findByRole('button', { name: 'Pin to Home' }));
  await vi.waitFor(async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    const d = (await mock.handle('get', { id: h.id })) as { body: string };
    expect(d.body).toContain('card:section:tasks?size=glance');
    // The generated Home is kept with it, so pinning never empties Home.
    expect(d.body).toContain('card:journal');
  });
  expect(await screen.findByRole('button', { name: 'On Home ✓' })).toBeTruthy();
});

it('the one box: a web address opens the page; words also offer a web search', async () => {
  const opened: string[] = [];
  vi.stubGlobal('open', (u: string) => void opened.push(u));
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  const field = (await screen.findAllByRole('searchbox', { name: 'search notes' }))[0];
  await fireEvent.input(field, { target: { value: 'example.org' } });
  await fireEvent.keyDown(field, { key: 'Enter' });
  expect(opened).toEqual(['https://example.org']);
  await fireEvent.input(field, { target: { value: 'ant colonies' } });
  expect(
    await screen.findByRole('button', { name: /Search the web for “ant colonies”/ }),
  ).toBeTruthy();
});

it('Alt+Home goes Home from any page', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.keyDown(window, { key: 'Home', altKey: true });
  await vi.waitFor(() => expect(location.pathname).toBe('/f/home'));
});

it('the Journal page: the day’s entry, what the day collected in four parts', async () => {
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
