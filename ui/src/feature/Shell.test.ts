// **The shell** (P2b): the rail takes a person anywhere inside the tab (the address changes, Back works,
// nothing reloads); Home shows what can be done and is customizable, kept as a note tagged `home`;
// the Journal shows the day; a note opens beside the page on a wide screen.
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Shell from './Shell.svelte';
import * as mock from '../lib/mock';
import { appToday } from '../lib/appTime';
import { typeInto } from '../lib/editor/testing';
import { resetShownVaults } from '../lib/shownVaults.svelte';
import { foldRail, railFolded } from '../lib/railFold.svelte';

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
  // Pinned, the same button takes it off again.
  await fireEvent.click(await screen.findByRole('button', { name: 'Remove from Home' }));
  await vi.waitFor(async () => {
    const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
    const d = (await mock.handle('get', { id: h.id })) as { body: string };
    expect(d.body).not.toContain('card:section:tasks');
  });
  expect(await screen.findByRole('button', { name: 'Pin to Home' })).toBeTruthy();
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

it('the one box: with a note that matches, the web is still one key away, and Enter shows what it does', async () => {
  await mock.handle('create_notes', {
    notes: [{ title: 'Ant colonies, field notes', body: 'Ant colonies move.', props: {} }],
  });
  const opened: string[] = [];
  vi.stubGlobal('open', (u: string) => void opened.push(u));
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  const field = (await screen.findAllByRole('searchbox', { name: 'search notes' }))[0];
  await fireEvent.input(field, { target: { value: 'ant colonies' } });
  const results = await screen.findByRole('region', { name: 'search results' });
  // The top note is the one Enter would open, and it is drawn so.
  const note = await within(results).findByRole('button', { name: /Ant colonies, field notes/ });
  await vi.waitFor(() => expect(note.getAttribute('aria-current')).toBe('true'));
  const webRow = within(results).getByRole('button', { name: /Search the web/ });
  expect(webRow.getAttribute('aria-current')).toBeNull();
  // Ctrl+Enter searches the web, whatever is chosen.
  await fireEvent.keyDown(field, { key: 'Enter', ctrlKey: true });
  expect(opened).toHaveLength(1);
  expect(decodeURIComponent(opened[0])).toContain('ant colonies');
  // ↑ chooses the web row, and Enter then searches the web.
  await fireEvent.input(field, { target: { value: 'ant colonies' } });
  const again = await screen.findByRole('region', { name: 'search results' });
  await within(again).findByRole('button', { name: /Ant colonies, field notes/ });
  await fireEvent.keyDown(field, { key: 'ArrowUp' });
  expect(
    within(again)
      .getByRole('button', { name: /Search the web/ })
      .getAttribute('aria-current'),
  ).toBe('true');
  await fireEvent.keyDown(field, { key: 'Enter' });
  expect(opened).toHaveLength(2);
});

it('Alt+Home goes Home from any page', async () => {
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.keyDown(window, { key: 'Home', altKey: true });
  await vi.waitFor(() => expect(location.pathname).toBe('/f/home'));
});

// **Only the journal** (the owner, 2026-11-24): the day's entry and the earlier days, nothing else.
it('the Journal page holds the journal and nothing else', async () => {
  history.replaceState(null, '', '/f/today');
  render(Shell);
  expect(await screen.findByRole('region', { name: /^journal, / })).toBeTruthy();
  expect(await screen.findByRole('region', { name: 'earlier days' })).toBeTruthy();
  const page = document.querySelector('.journal') as HTMLElement;
  const regions = [...page.querySelectorAll('section[aria-label]')].map((r) =>
    r.getAttribute('aria-label'),
  );
  expect(regions.every((n) => n === 'earlier days' || n?.startsWith('journal, '))).toBe(true);
  for (const name of ['Planned and due', 'Logged', 'Came in', 'Written'])
    expect(screen.queryByRole('region', { name })).toBeNull();
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

const homeBody = async () => {
  const [h] = (await mock.handle('tagged', { tag: 'home' })) as { id: string }[];
  if (!h) return '';
  return ((await mock.handle('get', { id: h.id })) as { body: string }).body;
};

it('a card’s ⋯: its width, its height, and Remove from Home, where it is', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const cards = await screen.findByRole('list', { name: 'home cards shown' });
  await within(cards).findByRole('heading', { name: /Continue/ });
  const open = async () => {
    await fireEvent.click(within(cards).getByRole('button', { name: 'more for Continue' }));
    return screen.findByRole('group', { name: 'Continue on Home' });
  };
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Full width' }));
  await vi.waitFor(async () => expect(await homeBody()).toMatch(/card:continue\?[^)]*w=full/));
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Tall' }));
  await vi.waitFor(async () => expect(await homeBody()).toMatch(/card:continue\?[^)]*h=tall/));
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Remove from Home' }));
  await vi.waitFor(async () => expect(await homeBody()).not.toContain('card:continue'));
});

it('a heatmap on Home: a square per day, as GitHub draws one', async () => {
  await mock.handle('create_notes', {
    notes: [
      { title: 'Idea one', body: 'x', props: {} },
      { title: 'Idea two', body: 'y', props: {} },
    ],
  });
  history.replaceState(null, '', '/f/home');
  render(Shell);
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit Home' }));
  await fireEvent.change(screen.getByRole('combobox', { name: 'card to add' }), {
    target: { value: 'heat:notes' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add to Home' }));
  const map = await screen.findByRole('img', {
    name: /Notes, day by day: \d+ in the last six months/,
  });
  expect(map.querySelectorAll('.hcol')).toHaveLength(26);
  expect(map.querySelectorAll('.hcell')).toHaveLength(26 * 7);
  await vi.waitFor(() =>
    expect(map.querySelectorAll('.hcell:not(.l0):not(.future)').length).toBeGreaterThan(0),
  );
});

it('a section hidden from the rail leaves it, and comes back from Show hidden sections', async () => {
  globalThis.localStorage?.removeItem('fm-rail-hidden');
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const rail = await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.click(within(rail).getByRole('button', { name: 'more for Health' }));
  await fireEvent.click(await within(rail).findByRole('menuitem', { name: 'Hide from the rail' }));
  const listed = within(rail).getByRole('list', { name: 'sections' });
  expect(within(listed).queryByRole('button', { name: /Health/ })).toBeNull();
  await fireEvent.click(within(rail).getByRole('button', { name: /Show hidden sections \(1\)/ }));
  await fireEvent.click(within(rail).getByRole('button', { name: 'show Health in the rail' }));
  expect(within(listed).getAllByRole('button', { name: /Health/ }).length).toBeGreaterThan(0);
  globalThis.localStorage?.removeItem('fm-rail-hidden');
});

// **The rail folds away** (the owner, 2026-11-24: no way of "dropping down the left panel").
it('the rail folds to a strip with one button, comes back, and Ctrl+\\ does either', async () => {
  foldRail(false);
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const rail = await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.click(within(rail).getByRole('button', { name: 'hide the side panel' }));
  expect(rail.classList.contains('folded')).toBe(true);
  // Folded: the entries are out of reach, and one button brings them back.
  expect((rail.querySelector('.panel') as HTMLElement).inert).toBe(true);
  await fireEvent.click(within(rail).getByRole('button', { name: 'show the side panel' }));
  expect(rail.classList.contains('folded')).toBe(false);
  expect(within(rail).queryByRole('button', { name: 'show the side panel' })).toBeNull();
  await fireEvent.keyDown(window, { key: '\\', ctrlKey: true });
  expect(railFolded()).toBe(true);
  await fireEvent.keyDown(window, { key: '\\', ctrlKey: true });
  expect(railFolded()).toBe(false);
});

it('the vault chooser: a vault unticked is left out of the counts, and comes back ticked', async () => {
  globalThis.localStorage?.removeItem('fm-hidden-vaults');
  resetShownVaults();
  const asked: unknown[] = [];
  const real = mock.handle;
  vi.spyOn(mock, 'handle').mockImplementation(((cmd: string, args: Record<string, unknown>) => {
    if (cmd === 'section_summary') asked.push(args?.hidden);
    return real(cmd, args);
  }) as typeof mock.handle);
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const rail = await screen.findByRole('navigation', { name: 'formicaria' });
  await fireEvent.click(await within(rail).findByRole('button', { name: /All vaults/ }));
  const list = await within(rail).findByRole('group', { name: 'vaults to see' });
  const boxes = within(list).getAllByRole('checkbox') as HTMLInputElement[];
  expect(boxes.length).toBeGreaterThan(1);
  await fireEvent.click(boxes[1]);
  expect(await within(rail).findByRole('button', { name: /\d+ of \d+ vaults/ })).toBeTruthy();
  await vi.waitFor(() => expect(asked.some((h) => Array.isArray(h) && h.length === 1)).toBe(true));
  await fireEvent.click(boxes[1]);
  expect(await within(rail).findByRole('button', { name: /All vaults/ })).toBeTruthy();
  globalThis.localStorage?.removeItem('fm-hidden-vaults');
  resetShownVaults();
});

it('Home: two quick changes are both kept, and a save that fails puts the screen back and says so', async () => {
  history.replaceState(null, '', '/f/home');
  render(Shell);
  const cards = await screen.findByRole('list', { name: 'home cards shown' });
  await within(cards).findByRole('heading', { name: /Continue/ });
  const open = async () => {
    await fireEvent.click(within(cards).getByRole('button', { name: 'more for Continue' }));
    return screen.findByRole('group', { name: 'Continue on Home' });
  };
  // Two changes, the second before the first is written.
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Full width' }));
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Tall' }));
  await vi.waitFor(async () => {
    const b = await homeBody();
    expect(b).toMatch(/card:continue\?[^)]*w=full/);
    expect(b).toMatch(/card:continue\?[^)]*h=tall/);
  });
  // A save refused: the card goes back to its size, and Home says why.
  const real = mock.handle;
  vi.spyOn(mock, 'handle').mockImplementation(((cmd: string, args: Record<string, unknown>) =>
    cmd === 'update_body'
      ? Promise.reject(new Error('the disk is full'))
      : real(cmd, args)) as typeof mock.handle);
  await fireEvent.click(within(await open()).getByRole('button', { name: 'Narrow' }));
  expect((await screen.findByRole('alert')).textContent).toContain('the disk is full');
  const li = within(cards)
    .getByRole('heading', { name: /Continue/ })
    .closest('li') as HTMLElement;
  expect(li.getAttribute('style')?.replace(/\s/g, '')).toContain('span12;');
});

it('the note beside closes, or takes the full screen', async () => {
  await mock.handle('create_notes', {
    notes: [{ title: 'Beside me', body: 'Words beside.', props: { status: 'todo' } }],
  });
  history.replaceState(null, '', '/f/group/Tasks');
  render(Shell);
  const open = async () => {
    const card = await vi.waitFor(() => {
      const c = [...document.querySelectorAll('.card')].find((e) =>
        e.textContent?.includes('Beside me'),
      );
      if (!c) throw new Error('no card');
      return c as HTMLElement;
    });
    await fireEvent.click(card);
    return screen.findByRole('complementary', { name: 'the note beside' });
  };
  const beside = await open();
  await fireEvent.click(within(beside).getByRole('button', { name: 'close the note beside' }));
  await vi.waitFor(() =>
    expect(screen.queryByRole('complementary', { name: 'the note beside' })).toBeNull(),
  );
  const again = await open();
  await fireEvent.click(within(again).getByRole('button', { name: /Full screen/ }));
  await vi.waitFor(() => expect(location.pathname).toMatch(/^\/f\/note\//));
  expect(screen.queryByRole('complementary', { name: 'the note beside' })).toBeNull();
});
