/// **The same controls, placed by the space available.**
///
/// Wide: a panel down the left, holding everything, so the workspace gets back the height a top
/// bar used to take. Narrow: the same element as a bar along the bottom, because a top bar on a
/// phone holds actions the thumb cannot reach and nearly every app ships one anyway.
///
/// What can be tested here is the *state*, not the geometry — jsdom applies no CSS, so which side
/// of the screen the panel is on is a question only a browser can answer, and this file must not
/// pretend otherwise. What it does pin: the collapse is remembered, it survives a reload, and the
/// control describes what it will do rather than what it is.
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import App from './App.svelte';
import { clearFaults } from './lib/mock';

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
});
afterEach(() => {
  vi.unstubAllGlobals();
  clearFaults();
});

test('the panel starts open, and says what collapsing will do', async () => {
  render(App);
  const btn = await screen.findByRole('button', { name: 'collapse the panel' });
  expect(btn.getAttribute('aria-expanded')).toBe('true');
});

test('collapsing is remembered, so it is a preference and not a mood', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'collapse the panel' }));

  // The label flips to what the *next* click does — a control that says what it is rather than
  // what it does is the one people press twice to find out.
  await screen.findByRole('button', { name: 'expand the panel' });
  await waitFor(() => expect(store.get('fm-panel')).toBe('collapsed'));
});

test('a collapsed panel comes back collapsed', async () => {
  store.set('fm-panel', 'collapsed');
  render(App);
  const btn = await screen.findByRole('button', { name: 'expand the panel' });
  expect(btn.getAttribute('aria-expanded')).toBe('false');
});

test('storage that throws still gives a working panel', async () => {
  // A private window, or a WebView with site data blocked. The panel is a convenience; failing to
  // remember it must never be why the app will not start.
  vi.stubGlobal('localStorage', {
    getItem: () => {
      throw new Error('denied');
    },
    setItem: () => {
      throw new Error('denied');
    },
    removeItem: () => {
      throw new Error('denied');
    },
  });
  render(App);
  const btn = await screen.findByRole('button', { name: 'collapse the panel' });
  await fireEvent.click(btn);
  await screen.findByRole('button', { name: 'expand the panel' });
});

test('the panel offers every view you can open, and opening one shows it', async () => {
  render(App);
  const rail = (await screen.findByRole('navigation', { name: 'views' })) as HTMLElement;
  const names = Array.from(rail.querySelectorAll('button')).map((b) =>
    (b.textContent ?? '').trim(),
  );
  // The list is fixed so it can be learned.
  for (const v of ['Board', 'Agenda', 'Timeline', 'Collaboration', 'Discussions']) {
    expect(names, `${v} missing from the rail`).toContain(v);
  }
  // **And two absences that are decisions, not omissions.** `Search` is offered nowhere: an empty
  // search pane does nothing, and the search box makes one when you type. `Activity` is off the
  // rail — a column costs attention per entry — and stays in the palette, which is filterable and
  // can afford it.
  expect(names).not.toContain('Search');
  expect(names).not.toContain('Activity');
  // And the saved views, which is the half a fixed list of built-ins would miss.
  expect(names).toContain('Active');

  // Clicking opens it: there was no agenda, so a window appears.
  const before = document.querySelectorAll('.pane').length;
  await fireEvent.click(screen.getByRole('button', { name: 'open Agenda' }));
  await waitFor(() => expect(document.querySelectorAll('.pane').length).toBe(before + 1));

  // **And clicking it again shows that one rather than stacking another.** With one view at a
  // time, tapping a name means "show me that"; appending a second agenda answers a question
  // nobody asked and, on a phone, spends a feed and a scroll position. jsdom applies no CSS, so
  // "showing" is asserted structurally — which cell carries `.active` — not by visibility.
  await fireEvent.click(screen.getByRole('button', { name: 'open Agenda' }));
  await fireEvent.click(screen.getByRole('button', { name: 'open Agenda' }));
  expect(document.querySelectorAll('.pane').length).toBe(before + 1);
  expect(document.querySelectorAll('.cell.active').length, 'one active window').toBe(1);
  // The name is read off the bar that names it — a pane has no header of its own when one view
  // fills the window, which is the row this arrangement exists to reclaim.
  const bar = screen.getByRole('navigation', { name: 'open views' });
  expect(bar.querySelector('[aria-current="page"]')?.textContent).toMatch(/Agenda/);
});

/// **The collapsed rail hides words; it must never crop them.**
///
/// The first version set the rail's width with `overflow: hidden` and left every label in place, so
/// it showed a sliver of "Back up" and a sliver of the vault name. jsdom applies no CSS, so this
/// cannot check that a label is *invisible* — and asserting that would be a lie. What it can check
/// is the thing that makes hiding possible at all: that no label is a bare text node the stylesheet
/// has no handle on.
test('every label in the chrome is wrapped in something CSS can hide', async () => {
  render(App);
  const header = (await waitFor(() => {
    const h = document.querySelector('header.topbar');
    if (!h) throw new Error('no header');
    return h;
  })) as HTMLElement;

  for (const chip of Array.from(header.querySelectorAll('.vault-chip'))) {
    expect(
      chip.querySelector('.lbl'),
      `a filter chip's name is not wrapped: ${chip.textContent}`,
    ).toBeTruthy();
  }
  for (const item of Array.from(header.querySelectorAll('.view-item'))) {
    expect(
      item.querySelector('.lbl'),
      `a rail label is not wrapped: ${item.textContent}`,
    ).toBeTruthy();
  }
});

test('back up is an icon like the controls beside it, and says so only while it is working', async () => {
  render(App);
  const btn = await screen.findByRole('button', { name: 'back up' });
  // Help and Settings next to it carry no words; one labelling rule per state is what makes the
  // column read as a column. The meaning lives in the tooltip.
  expect((btn.textContent ?? '').trim()).toBe('');
  expect(btn.getAttribute('title')).toMatch(/back up/i);
});

test('a view can be opened when the chrome is a bar, where there is no rail', async () => {
  // Below 60rem the rail is hidden and the pane header no longer switches views, so without this
  // a narrow window has no way to open a view at all. jsdom applies no CSS, so this cannot check
  // *which* of the two is visible at a given width — only that the second way in exists and works.
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  const menu = await screen.findByRole('menu');
  const names = Array.from(menu.querySelectorAll('button')).map((b) =>
    (b.textContent ?? '').trim(),
  );
  expect(names).toContain('Timeline');
  expect(names).toContain('Active'); // a saved view, not just the built-ins
  expect(names).not.toContain('Search'); // nowhere offers an empty search pane

  const before = document.querySelectorAll('.pane').length;
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Agenda' }));
  await waitFor(() => expect(document.querySelectorAll('.pane').length).toBe(before + 1));

  // The bar's menu is the other consumer of the same list, so it re-points too — one edit
  // covers both, and this is what proves they did not drift.
  await fireEvent.click(await screen.findByRole('button', { name: 'open a view' }));
  await fireEvent.click(await screen.findByRole('menuitem', { name: 'Agenda' }));
  expect(document.querySelectorAll('.pane').length).toBe(before + 1);
});

/// **A second search must change what you are looking at.**
///
/// `onSearchInput` re-pointed the existing search pane but never made it the active one. While
/// every pane was on screen that was invisible; the moment one view at a time became the default
/// it meant a second query did nothing you could see.
test('typing a second query re-points the one search window, and shows it', async () => {
  vi.useFakeTimers();
  try {
    render(App);
    const box = await screen.findByRole('searchbox', { name: 'search notes' });

    await fireEvent.input(box, { target: { value: 'alpha' } });
    await vi.advanceTimersByTimeAsync(300);
    expect(document.querySelectorAll('.pane').length).toBeGreaterThan(1);

    const panes = document.querySelectorAll('.pane').length;
    await fireEvent.input(box, { target: { value: 'beta' } });
    await vi.advanceTimersByTimeAsync(300);
    // Still one search window...
    expect(document.querySelectorAll('.pane').length).toBe(panes);
    expect(document.querySelectorAll('.cell.active').length).toBe(1);
    // ...and it is the one being described: the bar carries the active view's own controls, so
    // the query box up there showing `beta` *is* the assertion that the search became visible.
    // It also pins the thing the chrome's field cannot do — show you what you are searching for.
    expect((screen.getByLabelText('search') as HTMLInputElement).value).toBe('beta');
  } finally {
    vi.useRealTimers();
  }
});

/// **The lens must not make search disappear.**
///
/// In a collapsed rail the field is hidden by `.app.panel-collapsed .topbar .searchfield` (four
/// classes), which outranks `.search-slot.open .searchfield` (three) — while `.search-slot.open
/// .search-btn` hides the lens that was just pressed. So opening search on a collapsed panel hid
/// both halves and the chrome had no search in it at all. jsdom applies no CSS, so what is pinned
/// here is the state that CSS reads: the panel ends up expanded, which is where the field is
/// visible.
test('searching from a collapsed rail expands it instead of hiding the field', async () => {
  store.set('fm-panel', 'collapsed');
  render(App);
  const toggle = await screen.findByRole('button', { name: 'expand the panel' });
  expect(toggle.getAttribute('aria-expanded')).toBe('false');

  await fireEvent.click(screen.getByRole('button', { name: 'search notes' }));
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'collapse the panel' }).getAttribute('aria-expanded'),
    ).toBe('true'),
  );
  expect(store.get('fm-panel')).toBe('open');
});

/// The open-windows strip used to carry its own gear, because it was the phone's only way into
/// anything that was not a view. It is not on the phone any more, and two buttons with one name
/// were something two other test files had to work around.
test('there is exactly one way into Settings from the chrome', async () => {
  render(App);
  await screen.findByRole('button', { name: 'settings' });
  expect(screen.getAllByRole('button', { name: 'settings' })).toHaveLength(1);
});

test('a dropdown opens at the button that opened it, not at a fixed corner', async () => {
  // These menus are `position: fixed` because the chrome scrolls. Their coordinates used to be
  // hardcoded to the corners of a horizontal top bar — so once the chrome became a left column,
  // the backup menu opened in the opposite corner from its own button.
  render(App);

  // **Driven by the ＋ menu since 2026-09-09.** This used the backup chevron, which is gone: the
  // split Back up button became one control that opens the panel. The rule under test is about
  // *any* of these menus, so it moves to one that still exists — with a faked rect low on screen,
  // which is what it was really testing all along.
  const low = await screen.findByRole('button', { name: 'make something new' });
  low.getBoundingClientRect = () =>
    ({ left: 12, top: 700, bottom: 728, right: 40, width: 28, height: 28 }) as DOMRect;
  await fireEvent.click(low);

  const menu = await screen.findByRole('menu');
  const style = menu.getAttribute('style') ?? '';
  expect(style, 'the menu must carry measured coordinates').toMatch(/left:\s*12px/);
  // Low on screen, so it opens upward — anchored by its bottom, which needs no guess at its height.
  expect(style).toMatch(/bottom:/);
  expect(style).toMatch(/top:\s*auto/);
});

test('a dropdown from the top of the panel opens downward', async () => {
  render(App);
  const plus = await screen.findByRole('button', { name: 'make something new' });
  plus.getBoundingClientRect = () =>
    ({ left: 12, top: 20, bottom: 48, right: 40, width: 28, height: 28 }) as DOMRect;
  await fireEvent.click(plus);

  const style = (await screen.findByRole('menu')).getAttribute('style') ?? '';
  expect(style).toMatch(/top:\s*52px/);
  expect(style).not.toMatch(/bottom:/);
});

/// **Where a new note lands is asked where you decide it.**
///
/// This was a permanent `in <select>` in the chrome — a control on screen at all times for a
/// choice made only while creating something, and a whole row of a narrow bar. It moved into the
/// ＋ menu on 2026-08-31. It had no test at all before, which is part of why it was easy to leave
/// sitting there: nothing said what it was for.
test('the ＋ menu chooses where a new note lands, and stays open while you choose', async () => {
  render(App);
  await fireEvent.click(await screen.findByRole('button', { name: 'make something new' }));

  // The things you can make are still the first thing in the menu.
  expect(await screen.findByRole('menuitem', { name: 'New note' })).toBeTruthy();

  const targets = await screen.findAllByRole('menuitemradio');
  expect(targets.length, 'one row per vault').toBeGreaterThan(1);
  expect(targets.filter((t) => t.getAttribute('aria-checked') === 'true')).toHaveLength(1);

  // Picking a destination does NOT close the menu: you almost always pick where, then pick what,
  // and closing here would mean opening the ＋ twice for one note.
  const other = targets.find((t) => t.getAttribute('aria-checked') === 'false')!;
  await fireEvent.click(other);
  await waitFor(() => expect(other.getAttribute('aria-checked')).toBe('true'));
  expect(screen.queryByRole('menuitem', { name: 'New note' }), 'menu still open').not.toBeNull();
  // Exactly one destination at a time — it is a radio, unlike the vault *filter* beside it.
  expect(
    screen.getAllByRole('menuitemradio').filter((t) => t.getAttribute('aria-checked') === 'true'),
  ).toHaveLength(1);
  // And it is remembered, the way it always was.
  expect(store.get('fm-create-vault')).toBeTruthy();
});

/// **A withdrawn surface, not a deleted feature.**
///
/// Save/Rename/Delete view left the UI on 2026-08-31 — *"views are basically fixed for now and view
/// customization will need its own design plan"*. The commands, their `ipc.ts` wrappers and every
/// Rust test stayed, and this is the guard that the *reading* half survived the cut: a `.view` file
/// that arrives by hand or over git must still list in the rail and still open. If this ever fails,
/// the withdrawal has quietly become a removal.
test('a saved view still lists and opens, with no way to author one anywhere', async () => {
  render(App);
  const rail = (await screen.findByRole('navigation', { name: 'views' })) as HTMLElement;
  const names = Array.from(rail.querySelectorAll('button')).map((b) =>
    (b.textContent ?? '').trim(),
  );
  expect(names, 'a .view in the vault is still offered').toContain('Active');

  const before = document.querySelectorAll('.pane').length;
  await fireEvent.click(screen.getByRole('button', { name: 'open Active' }));
  await waitFor(() => expect(document.querySelectorAll('.pane').length).toBe(before + 1));

  // And nothing anywhere in the chrome offers to make, rename or remove one.
  for (const gone of [
    /save this view/i,
    /rename this view/i,
    /delete this view/i,
    /keep this arrangement/i,
  ]) {
    expect(screen.queryByRole('button', { name: gone }), `${gone} should be gone`).toBeNull();
  }
  expect(screen.queryByLabelText('group by'), 'grouping is no longer editable').toBeNull();
});

/// **Every alert chip names itself, so it can stop showing its words.**
///
/// At narrow widths the chips shed their labels and keep their icon and count — five chips of
/// prose wrapped the toolbar on the owner's phone (2026-09-08) and pushed the board down, the same
/// failure the four-row measurement records. The words moving out is only safe because each chip
/// carries an explicit `aria-label`: without one, hiding the text leaves a button a screen reader
/// cannot name and a test cannot find.
///
/// **This is the only half that can be tested here**, and the header above says why: jsdom applies
/// no CSS, so nothing in this file can observe the label actually disappearing. What it pins is the
/// thing that makes the disappearing safe.
///
/// Proven red by deleting any of the five `aria-label` attributes: the chip's name falls back to
/// its visible text, which is exactly the dependency this removes.
test('every alert chip carries a name of its own, not one borrowed from its label', async () => {
  render(App);
  // Anchored on a chip that must appear, so this cannot pass before the toolbar has rendered.
  await screen.findByRole('button', { name: /days since a save/i });

  const chips = [...document.querySelectorAll('.tb-chip.alert')];
  expect(chips.length).toBeGreaterThan(0);
  for (const chip of chips) {
    const name = chip.getAttribute('aria-label');
    expect(name, `${chip.textContent?.trim()} has no aria-label`).toBeTruthy();
    // And it says something, rather than repeating the bare count the chip still shows.
    expect(name!.trim().length).toBeGreaterThan(3);
  }
});
