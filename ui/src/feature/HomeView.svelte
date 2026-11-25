<script lang="ts">
  // **Home: a glance, a journal, cards** (`unifying-home-research-2026-11-22.md` §12, 2026-11-23).
  // The owner: Home should give *"a quick glance on the current status, and the user selects what
  // to see"*; a board as its figures and what needs a look, or the whole board, beside anything
  // else chosen; a journal entry for the day, ready to write in, as in Logseq; calm on a small
  // screen, with quick links to what is kept an eye on.
  //
  // - **The glance row**: figures (a section this week, what is due, a board's open cards).
  // - **The watch row**: links to notes, views and sections kept an eye on.
  // - **The journal**: today's entry, ready to write in.
  // - **Cards** at their size: summary (what needs a look) or full (the whole thing).
  //
  // What is on Home and how is kept in the Home note (`lib/home.ts`); with nothing set up, Home is
  // generated from what the person uses. **Edit Home** lists the cards (size, on the phone, order)
  // and what can be added; **Pin to Home** works from a note's ⋯ and a section's bar.
  import { onMount } from 'svelte';
  import { hiddenVaults, shownVault } from '../lib/shownVaults.svelte';
  import * as ipc from '../lib/ipc';
  import sectionsJson from '../lib/sections.json';
  import type { ObjectMeta, Section, SectionSummary, ViewInfo } from '../lib/types';
  import { appToday, dayOfInstant } from '../lib/appTime';
  import { addDays, mondayOf } from '../lib/civil';
  import { isDeadline, isOverdue } from '../lib/urgency';
  import { dayOf } from '../lib/stamp';
  import {
    BUILTIN,
    COLS,
    GAP_REM,
    MAX_ROWS,
    MIN_COLS,
    MIN_ROWS,
    ROW_REM,
    arrange,
    defaultRoom,
    generated,
    labelOf,
    layout,
    placeAll,
    readHome,
    rowsRem,
    saveHome,
    sizesOf,
    tidy,
    HEIGHTS,
    WIDTHS,
    type Box,
    type Card,
    type CardHeight,
    type CardSize,
    type CardWidth,
    type HomeDoc,
  } from '../lib/home';
  import { webTarget } from '../lib/features';
  import { searchUrl } from '../lib/privacy.svelte';
  import JournalEntry from './JournalEntry.svelte';
  import { popup } from '../lib/popup';
  import { KEYBOARD_EVENT } from '../lib/keyboard';

  let { go, onopen }: { go: (url: string) => void; onopen: (id: string) => void } = $props();

  const sections = sectionsJson as Section[];
  const sectionOf = (key: string) => sections.find((s) => s.key === key);

  let summary = $state<SectionSummary[]>([]);
  let recent = $state<ObjectMeta[]>([]);
  let agenda = $state<ObjectMeta[]>([]);
  let pinned = $state<ObjectMeta[]>([]);
  let views = $state<ViewInfo[]>([]);
  let paired = $state(false);
  /** What each card that reads for itself has read, by its ref. */
  let rows = $state<Record<string, ObjectMeta[]>>({});
  let notes = $state<Record<string, { id: string; title: string; body: string } | null>>({});
  let home = $state<{ id: string; version: string } | null>(null);
  let doc = $state<HomeDoc>({ cards: [], text: '' });
  let editing = $state(false);
  let saveError = $state('');
  let narrow = $state(false);
  let root = $state<HTMLDivElement | undefined>(undefined);
  let entry = $state<{ focus: () => void } | undefined>(undefined);
  /** **A card being moved or resized** (2026-11-25): where it would go, the others making way while
   *  the pointer is down; written when it is let go. */
  let moving = $state<Box | null>(null);
  let resizing = $state<Box | null>(null);
  /** On the phone: the card being moved in the list, and the card it would go before. */
  let lifted = $state<string | null>(null);
  let dropOn = $state<string | null>(null);
  /** **Writing on the phone, the journal has the screen** (2026-11-23; the owner: Home's top, a
   *  floating jot bar and a sliver of the entry, with the keyboard up). Home glides until the entry
   *  is at the top, the entry takes the height above the keyboard (`--app-h`), the jot bar fades;
   *  leaving the entry glides back to where Home was. No movement when motion is to be reduced. */
  let inEntry = $state(false);
  let wasAt = 0;
  const motion = () =>
    typeof matchMedia === 'undefined' || !matchMedia('(prefers-reduced-motion: reduce)').matches;
  let entryCard: HTMLElement | null = null;
  const toEntry = () =>
    entryCard?.scrollIntoView?.({ block: 'start', behavior: motion() ? 'smooth' : 'auto' });
  function enterWriting(e: FocusEvent) {
    if (!narrow || inEntry) return;
    inEntry = true;
    wasAt = root?.scrollTop ?? 0;
    entryCard = e.currentTarget as HTMLElement;
    requestAnimationFrame(toEntry);
  }
  // The keyboard coming up resizes Home and cuts a glide short: once it has settled, again.
  onMount(() => {
    const settled = () => inEntry && requestAnimationFrame(toEntry);
    addEventListener(KEYBOARD_EVENT, settled);
    return () => removeEventListener(KEYBOARD_EVENT, settled);
  });
  function leaveWriting(e: FocusEvent) {
    const card = e.currentTarget as HTMLElement;
    if (!inEntry || card.contains(e.relatedTarget as Node | null)) return;
    inEntry = false;
    root?.scrollTo?.({ top: wasAt, behavior: motion() ? 'smooth' : 'auto' });
  }
  /** The card or figure whose ⋯ is open. */
  let menuRef = $state<string | null>(null);
  /** Where a card's ⋯ is, for its menu to open beside (`popup` keeps it on the screen). */
  function moreBox(ref: string) {
    const b = [...document.querySelectorAll<HTMLElement>('button.more-btn')].find(
      (x) => x.dataset.ref === ref,
    );
    const r = b?.getBoundingClientRect();
    return r
      ? { top: r.top, bottom: r.bottom, left: r.left, right: r.right }
      : { top: 0, bottom: 0, left: 0, right: 0 };
  }
  /** A year of each section's notes, by when they were made: for heatmaps only. */
  let year = $state<SectionSummary[]>([]);

  /** The cards shown: the person's, or the generated ones while nothing is set up. */
  const cards = $derived(doc.cards.length ? doc.cards : generated(summary));
  const set = $derived(doc.cards.length > 0);
  const placed = $derived(layout(cards, narrow));
  /** The wide grid's boxes, with what is being moved or resized where it would go. */
  const boxes = $derived(arrange(cards, moving ?? resizing ?? undefined));
  const boxOf = (ref: string) => boxes.find((b) => b.ref === ref);
  const today = $derived(appToday());

  /** **Only the newest read lands** (2026-11-23): a read begun before a save and answered after it
   *  put the older Home back on screen, so an edit seemed to snap back. A save, and each newer
   *  read, makes the older ones stale. */
  let readGen = 0;
  async function read() {
    const gen = ++readGen;
    const [s, r, a, p, v, h, alive] = await Promise.all([
      ipc.sectionSummary(hiddenVaults()).catch(() => []),
      ipc.recent().catch(() => []),
      ipc.getAgenda().catch(() => []),
      ipc.tagged('pinned').catch(() => []),
      ipc.listViews().catch(() => []),
      readHome(),
      ipc.alive().catch(() => null),
    ]);
    if (gen !== readGen) return;
    // The vaults chosen not to see leave Home too (the rail's vault chooser).
    const seen = (n: ObjectMeta) => shownVault(n.vault);
    summary = s;
    recent = r.filter(seen);
    agenda = a.filter(seen);
    pinned = p.filter(seen);
    views = v.filter((x) => !x.error);
    paired = !!alive && 'devices' in alive && (alive.devices ?? 0) > 0;
    home = h ? { id: h.id, version: h.version } : null;
    doc = h?.doc ?? { cards: [], text: '' };
    await fill();
  }
  /** Each card that shows notes of its own reads them: a board, a section, a view, a note. */
  async function fill() {
    const next: Record<string, ObjectMeta[]> = {};
    const nextNotes: typeof notes = {};
    await Promise.all(
      cards.map(async (c) => {
        const [fam, arg] = [c.ref.split(':')[0], c.ref.slice(c.ref.indexOf(':') + 1)];
        if (fam === 'board') next[c.ref] = await ipc.sectionNotes(arg, 500).catch(() => []);
        else if (fam === 'section') next[c.ref] = await ipc.sectionNotes(arg, 20).catch(() => []);
        else if (fam === 'view') {
          const res = await ipc.runView(arg).catch(() => null);
          next[c.ref] = res?.rows ?? res?.board?.columns.flatMap((col) => col.cards) ?? [];
        } else if (fam === 'note') {
          const n = await ipc.getNote(arg).catch(() => null);
          nextNotes[c.ref] = n ? { id: n.id, title: n.title ?? '', body: n.body } : null;
        }
      }),
    );
    for (const k of Object.keys(next)) next[k] = next[k].filter((n) => shownVault(n.vault));
    rows = next;
    notes = nextNotes;
    if (cards.some((c) => c.ref.startsWith('heat:')))
      year = await ipc.sectionSummary(hiddenVaults(), 371).catch(() => []);
  }

  // Read now, and again when the vaults shown change.
  $effect(() => {
    void hiddenVaults().join('\n');
    void read();
  });
  onMount(() => {
    const stop = ipc.listenChanges(() => void read());
    const again = () => {
      if (placed.journal) entry?.focus();
      else go('/f/today');
    };
    addEventListener('fm-home-again', again);
    const away = (e: MouseEvent) => {
      const t = e.target as HTMLElement | null;
      if (!t?.closest?.('.cmenu, .more-btn')) menuRef = null;
    };
    addEventListener('click', away);
    // Narrow by Home's own width (a phone, or a pane on a tablet), not the window's.
    const measure = () => (narrow = !!root && root.clientWidth > 0 && root.clientWidth < 700);
    const ro = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    if (root) ro?.observe(root);
    measure();
    return () => {
      stop();
      removeEventListener('fm-home-again', again);
      removeEventListener('click', away);
      ro?.disconnect();
    };
  });

  /** **A change shows at once**, and is written after (2026-11-23): waiting for the write left a
   *  card at its old size for seconds on a busy computer. Writes go one after another, each on
   *  the version the one before left, and a write that fails puts the screen back and says so. */
  let writing: Promise<void> = Promise.resolve();
  let edits = 0;
  function save(next: HomeDoc): Promise<void> {
    readGen++;
    saveError = '';
    const before = $state.snapshot(doc) as HomeDoc;
    const mine = ++edits;
    doc = next;
    writing = writing.then(async () => {
      try {
        home = await saveHome(next, home);
        await fill();
      } catch (e) {
        // Back to what was shown, unless a newer change has replaced it since.
        if (mine === edits) doc = before;
        saveError = `Home was not saved: ${String(e)}`;
      }
    });
    return writing;
  }
  /** The cards as they stand, made the person's own on the first edit. */
  const own = (): Card[] =>
    (doc.cards.length ? doc.cards : generated(summary)).map((c) => ({ ...c }));
  /** **A card changed on the phone is wanted on the phone** (2026-11-23: a board set to *summary*
   *  on the phone stayed off it, unticked). Unless the change is that very tick. */
  const change = (i: number, patch: Partial<Card>) => {
    const list = own();
    const here = narrow && !('phone' in patch) ? { phone: true } : {};
    // A card made a figure (or back) takes that kind's room, unless the room is what changes.
    const room =
      patch.size && patch.size !== list[i].size && !('cols' in patch)
        ? defaultRoom(list[i].ref, patch.size)
        : {};
    list[i] = { ...list[i], ...room, ...patch, ...here };
    void save({ ...doc, cards: list });
  };
  const move = (i: number, by: number) => {
    const list = own();
    const j = i + by;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    void save({ ...doc, cards: list });
  };
  const remove = (i: number) => void save({ ...doc, cards: own().filter((_, k) => k !== i) });
  /** A card's place in the list, by what it shows. */
  const at = (ref: string) => cards.findIndex((c) => c.ref === ref);
  const changeRef = (ref: string, patch: Partial<Card>) => {
    menuRef = null;
    const i = at(ref);
    if (i >= 0) change(i, patch);
  };
  function removeRef(ref: string) {
    menuRef = null;
    const i = at(ref);
    if (i >= 0) remove(i);
  }
  /** The places as they are now, written into every card: a move, a resize or a tidy saved. */
  const savePlaces = (b: Box[]) => void save({ ...doc, cards: placeAll(own(), b) });
  /** The grid's measures in pixels: a column's step, a row's step, and its corner. */
  function measures(el: Element | null) {
    const grid = el?.closest('ol.grid') as HTMLElement | null;
    if (!grid) return null;
    const g = grid.getBoundingClientRect();
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const gap = GAP_REM * rem;
    return { left: g.left, top: g.top, col: (g.width + gap) / COLS, row: ROW_REM * rem + gap };
  }
  /** **Moving a card** (2026-11-25): its ⠿ grip, held and dragged with a mouse, a pen or a finger.
   *  On the wide grid it goes to the column and row under it and the others make way; on the phone
   *  (after a short hold, so a swipe still scrolls) it goes before the card it is let go over. */
  function lift(e: PointerEvent, c: Card) {
    if (e.button > 0) return;
    const handle = e.currentTarget as HTMLElement;
    const m = measures(handle);
    const start = boxOf(c.ref);
    const sx = e.clientX;
    const sy = e.clientY;
    const touch = e.pointerType === 'touch';
    let live = !touch;
    let hold = touch
      ? setTimeout(() => {
          live = true;
          if (narrow) lifted = c.ref;
          navigator.vibrate?.(15);
        }, 300)
      : 0;
    handle.setPointerCapture?.(e.pointerId);
    const go = (ev: PointerEvent) => {
      if (!live) {
        if (Math.hypot(ev.clientX - sx, ev.clientY - sy) > 8) end();
        return;
      }
      ev.preventDefault();
      if (narrow) {
        lifted = c.ref;
        const over = [...(root?.querySelectorAll<HTMLElement>('ol.grid > li.card') ?? [])].find(
          (li) => {
            const r = li.getBoundingClientRect();
            return ev.clientY >= r.top && ev.clientY < r.bottom;
          },
        );
        dropOn = over?.dataset.ref && over.dataset.ref !== c.ref ? over.dataset.ref : null;
        return;
      }
      if (!m || !start) return;
      moving = {
        ...start,
        x: Math.round(start.x + (ev.clientX - sx) / m.col),
        y: Math.round(start.y + (ev.clientY - sy) / m.row),
      };
    };
    const end = (ev?: PointerEvent) => {
      clearTimeout(hold);
      hold = 0;
      handle.removeEventListener('pointermove', go);
      handle.removeEventListener('pointerup', end);
      handle.removeEventListener('pointercancel', end);
      if (narrow) {
        const to = ev?.type === 'pointerup' ? dropOn : null;
        lifted = null;
        dropOn = null;
        if (to) moveBefore(c.ref, to);
        return;
      }
      const m2 = moving;
      moving = null;
      if (ev?.type === 'pointerup' && m2 && (m2.x !== start?.x || m2.y !== start?.y))
        savePlaces(arrange(cards, m2));
    };
    handle.addEventListener('pointermove', go);
    handle.addEventListener('pointerup', end);
    handle.addEventListener('pointercancel', end);
  }
  /** In the list (the phone's order): `ref` takes the place of `over` — dragged up it goes
   *  before it, dragged down after it. */
  function moveBefore(ref: string, over: string) {
    const list = own();
    const from = list.findIndex((c) => c.ref === ref);
    const target = list.findIndex((c) => c.ref === over);
    if (from < 0 || target < 0) return;
    const [c] = list.splice(from, 1);
    // Removed first: below it, `target` is now just after the card it was over.
    list.splice(target, 0, c);
    void save({ ...doc, cards: list });
  }
  /** **The keys move it too**: the arrows on its grip, or ⋯ → Move (a column, or a row, at a
   *  time). On the phone up and down move it in the list. */
  function nudge(c: Card, dx: number, dy: number) {
    if (narrow) {
      const i = own().findIndex((x) => x.ref === c.ref);
      if (dy) move(i, dy);
      return;
    }
    const b = boxOf(c.ref);
    if (!b) return;
    // Up past the card above: its place is taken (that card makes way below).
    const above =
      dy < 0
        ? boxes.filter(
            (o) =>
              o.ref !== b.ref && o.y + o.rows <= b.y && o.x < b.x + b.cols && b.x < o.x + o.cols,
          )
        : [];
    const y =
      dy < 0 && above.length
        ? Math.max(...above.map((o) => o.y))
        : b.y + dy * (dy > 0 ? b.rows : 1);
    savePlaces(arrange(cards, { ...b, x: b.x + dx, y }));
  }
  /** A width or a height from the ⋯ or Edit Home: on the wide grid the others make way. */
  function resize(c: Card, room: { cols?: number; rows?: number }) {
    menuRef = null;
    const b = boxOf(c.ref);
    if (narrow || !b) return changeRef(c.ref, room);
    const cols = Math.min(room.cols ?? b.cols, COLS);
    savePlaces(
      arrange(cards, { ...b, cols, rows: room.rows ?? b.rows, x: Math.min(b.x, COLS - cols) }),
    );
  }
  function onGripKey(e: KeyboardEvent, c: Card) {
    const by: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const d = by[e.key];
    if (!d) return;
    e.preventDefault();
    nudge(c, d[0], d[1]);
  }
  /** **Resizing**: the right edge sets the width, the bottom one the height, the corner both — a
   *  column or a row at a time, the others making way as it grows. */
  function grip(e: PointerEvent, c: Card, axis: 'x' | 'y' | 'xy') {
    e.preventDefault();
    const handle = e.currentTarget as HTMLElement;
    const m = measures(handle);
    const start = boxOf(c.ref);
    if (!m || !start) return;
    const li = handle.closest('li.card') as HTMLElement | null;
    const box = li?.getBoundingClientRect();
    handle.setPointerCapture?.(e.pointerId);
    resizing = { ...start };
    const go = (ev: PointerEvent) => {
      if (!resizing || !box) return;
      const cols = Math.round((ev.clientX - box.left) / m.col);
      const rows = Math.round((ev.clientY - box.top) / m.row);
      resizing = {
        ...start,
        cols: axis === 'y' ? start.cols : Math.min(COLS - start.x, Math.max(MIN_COLS, cols)),
        rows: axis === 'x' ? start.rows : Math.min(MAX_ROWS, Math.max(MIN_ROWS, rows)),
      };
    };
    const up = (ev: PointerEvent) => {
      handle.removeEventListener('pointermove', go);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      const r = resizing;
      resizing = null;
      if (ev.type === 'pointerup' && r && (r.cols !== start.cols || r.rows !== start.rows))
        savePlaces(arrange(cards, r));
    };
    handle.addEventListener('pointermove', go);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  }
  const SIZE_NAMES: Record<CardSize, string> = {
    glance: 'Glance',
    summary: 'Summary',
    full: 'Full',
  };
  const WIDTH_NAMES: Record<CardWidth, string> = {
    narrow: 'Narrow',
    half: 'Half',
    wide: 'Wide',
    full: 'Full width',
  };
  const HEIGHT_NAMES: Record<CardHeight, string> = {
    short: 'Short',
    medium: 'Medium',
    tall: 'Tall',
  };
  function add(ref: string, size?: CardSize) {
    if (!ref || cards.some((c) => c.ref === ref)) return;
    const sizes = sizesOf(ref);
    const card: Card = {
      ref,
      label: '',
      size: size && sizes.includes(size) ? size : sizes.includes('summary') ? 'summary' : sizes[0],
      // Added on the phone, it is for the phone.
      phone: narrow || (BUILTIN[ref]?.phone ?? false),
      ...defaultRoom(ref, size && sizes.includes(size) ? size : 'summary'),
    };
    void save({ ...doc, cards: [...own(), card] });
  }
  /** Reset: nothing set up, so Home is generated from use again. */
  const reset = () => void save({ cards: [], text: doc.text });
  let words = $state('');
  $effect(() => {
    if (editing) words = doc.text;
  });
  let dragFrom = -1;
  function drop(to: number) {
    if (dragFrom < 0 || dragFrom === to) return;
    const list = own();
    const [c] = list.splice(dragFrom, 1);
    list.splice(to, 0, c);
    dragFrom = -1;
    void save({ ...doc, cards: list });
  }

  // **What can be added**, as groups of choices (a list of toggles, never a blank canvas).
  const catalogue = $derived([
    {
      group: 'Today',
      items: Object.keys(BUILTIN).map((k) => ({ ref: k, label: BUILTIN[k].label })),
    },
    {
      group: 'Figures',
      items: sections
        .filter((s) => s.key !== 'discussions' && s.key !== 'collaboration')
        .map((s) => ({ ref: `stat:${s.key}`, label: `${s.label} this week`, size: 'glance' })),
    },
    {
      group: 'Heatmaps',
      items: sections
        .filter((s) => s.key !== 'discussions' && s.key !== 'collaboration')
        .map((s) => ({ ref: `heat:${s.key}`, label: `${s.label}, day by day` })),
    },
    {
      group: 'Boards',
      items: [{ ref: 'board:tasks', label: 'Tasks board' }],
    },
    {
      group: 'Sections',
      items: sections.map((s) => ({ ref: `section:${s.key}`, label: s.label })),
    },
    { group: 'Views', items: views.map((v) => ({ ref: `view:${v.name}`, label: v.name })) },
    {
      group: 'Notes',
      items: recent
        .filter((n) => !n.tags.includes('home') && !n.tags.includes('journal'))
        .slice(0, 15)
        .map((n) => ({ ref: `note:${n.id}`, label: n.title || n.preview })),
    },
  ] as { group: string; items: { ref: string; label: string; size?: CardSize }[] }[]);
  let adding = $state('');

  // ---- what the cards show ----
  const count = (key: string) => summary.find((s) => s.key === key)?.count ?? 0;
  /** `YYYY-MM-DD`, `n` days before today, by the calendar (`civil.ts`). */
  const back = (n: number): string => addDays(today, -n);
  /** Notes made per day in a section, over the last `days` days, oldest first. */
  function perDay(key: string, days: number): number[] {
    const made = (summary.find((s) => s.key === key)?.recent ?? []).map((t) => dayOfInstant(t));
    return Array.from(
      { length: days },
      (_, i) => made.filter((d) => d === back(days - 1 - i)).length,
    );
  }
  const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
  /** **A heatmap**, as GitHub draws one: a column per week (Monday at the top), a square per day,
   *  darker for more notes made that day, over the last `weeks` weeks. */
  function heat(key: string, weeks: number) {
    const made = new Map<string, number>();
    for (const t of year.find((s) => s.key === key)?.recent ?? []) {
      const d = dayOfInstant(t);
      made.set(d, (made.get(d) ?? 0) + 1);
    }
    const first = addDays(mondayOf(today), -7 * (weeks - 1));
    const level = (n: number) => (n === 0 ? 0 : n === 1 ? 1 : n <= 3 ? 2 : n <= 6 ? 3 : 4);
    let total = 0;
    const cols = Array.from({ length: weeks }, (_, w) =>
      Array.from({ length: 7 }, (_, d) => {
        const day = addDays(first, w * 7 + d);
        const n = day > today ? 0 : (made.get(day) ?? 0);
        total += n;
        return { day, n, level: level(n), future: day > today };
      }),
    );
    return { cols, total };
  }
  function stat(key: string) {
    const fortnight = perDay(key, 14);
    const week = sum(fortnight.slice(7));
    const before = sum(fortnight.slice(0, 7));
    return { week, trend: week > before ? '↑' : week < before ? '↓' : '' };
  }
  // Today's meetings and spans, and the deadlines due today or overdue. A meeting or a plan that
  // ended earlier is over, not overdue (`isDeadline`, 2026-11-23).
  const due = $derived(
    agenda.filter(
      (n) =>
        dayOfInstant(n.start ?? '') === today ||
        (isDeadline(n) && n.status !== 'done' && dayOf(n.due ?? '') <= today),
    ),
  );
  const overdueDue = $derived(due.filter((n) => isOverdue(n)));
  const justIn = $derived(
    recent.filter(
      (n) =>
        ['web', 'song', 'video', 'meeting', 'paper'].includes(n.kind_key ?? '') &&
        dayOfInstant(n.created) >= back(6),
    ),
  );
  const carryOn = $derived(
    recent.find(
      (n) => !n.tags.some((t) => ['home', 'journal', 'tutorial'].includes(t)) && n.title !== today,
    ) ?? null,
  );
  /** A board's state: its columns, and what needs a look (overdue, blocked, unmoved a week). */
  function board(ref: string) {
    const all = rows[ref] ?? [];
    const order = ['todo', 'doing', 'blocked', 'done'];
    const statuses = [...new Set(all.map((n) => n.status || 'none'))].sort(
      (a, b) => (order.indexOf(a) + 1 || 99) - (order.indexOf(b) + 1 || 99),
    );
    const open = all.filter((n) => n.status !== 'done');
    const week = back(7);
    const look = open
      .map((n) => ({
        n,
        why: isOverdue(n)
          ? 'overdue'
          : n.status === 'blocked'
            ? 'blocked'
            : dayOfInstant(n.updated) < week
              ? 'unmoved a week'
              : '',
      }))
      .filter((x) => x.why);
    return {
      columns: statuses.map((s) => ({
        status: s,
        cards: all.filter((n) => (n.status || 'none') === s),
      })),
      open: open.length,
      overdue: look.filter((x) => x.why === 'overdue').length,
      look,
    };
  }
  const steps = $derived([
    {
      label: 'Write in today’s journal',
      done: recent.some((n) => n.tags.includes('journal')),
      go: () => go('/f/today'),
    },
    { label: 'Keep a web page', done: count('web') > 0, go: () => go('/f/group/Web') },
    {
      label: 'Log a meal or a workout',
      done: count('food') + count('fitness') > 0,
      go: () => go('/f/group/Food'),
    },
    {
      label: 'Add a song or a video',
      done: count('music') + count('videos') > 0,
      go: () => go('/f/group/Music'),
    },
    { label: 'Connect your phone', done: paired, go: () => go('/f/settings') },
  ]);

  /** Where a card's title goes: its whole place. */
  function whole(c: Card): (() => void) | null {
    const [fam, arg] = [c.ref.split(':')[0], c.ref.slice(c.ref.indexOf(':') + 1)];
    if (fam === 'stat' || fam === 'board' || fam === 'section') {
      const s = sectionOf(arg);
      return s ? () => go(s.page) : null;
    }
    if (fam === 'view') return () => go(`/f/view/${encodeURIComponent(arg)}`);
    if (fam === 'note') return () => onopen(arg);
    if (c.ref === 'due') return () => go('/f/agenda');
    if (c.ref === 'journal') return () => go('/f/today');
    if (c.ref === 'justin' || c.ref === 'recent') return () => go('/f/timeline');
    return null;
  }
  const title = (n: ObjectMeta) => n.title || n.preview || 'Untitled';

  // ---- the capture bar (a narrow screen): a line into today's entry, or a page to open ----
  let captured = $state('');
  let captureNote = $state('');
  async function capture() {
    const t = captured.trim();
    if (!t) return;
    const web = webTarget(t, searchUrl);
    if (web && !/\s/.test(t) && !web.label.startsWith('Search')) {
      captured = '';
      go(web.url);
      return;
    }
    try {
      const [n] = await ipc.journal(today, 1);
      if (n && n.title === today) {
        const d = await ipc.getNote(n.id);
        if (!d) throw new Error('today’s entry could not be read');
        const body = d.body.trimEnd();
        await ipc.updateBody(d.id, `${body}${body ? '\n' : ''}- ${t}\n`, d.version);
      } else {
        await ipc.createNotes([
          { title: today, body: `- ${t}\n`, tags: ['journal'], start: '', props: {} },
        ]);
      }
      captured = '';
      captureNote = 'Added to today’s journal.';
    } catch (e) {
      captureNote = `Not added: ${String(e)}`;
    }
  }
</script>

{#snippet tile(c: Card)}
  {@const fam = c.ref.split(':')[0]}
  {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
  <button type="button" class="tile-btn" onclick={() => whole(c)?.()}>
    {#if fam === 'stat'}
      {@const st = stat(arg)}
      <b>{st.week}{st.trend ? ` ${st.trend}` : ''}</b><span>{labelOf(c, sections)}</span>
    {:else if fam === 'board'}
      {@const b = board(c.ref)}
      <b>{b.open}</b><span
        >open in {sectionOf(arg)?.label ?? arg}{b.overdue ? ` · ${b.overdue} overdue` : ''}</span
      >
    {:else if c.ref === 'due'}
      <b>{due.length}</b><span
        >due today{overdueDue.length ? ` · ${overdueDue.length} overdue` : ''}</span
      >
    {:else if c.ref === 'justin'}
      <b>{justIn.length}</b><span>came in this week</span>
    {:else if fam === 'note'}<b class="link-name">{c.label || notes[c.ref]?.title || 'A note'}</b>
    {:else if fam === 'section'}<b class="link-name">{labelOf(c, sections)}</b><span
        >{count(arg)} notes</span
      >
    {:else}<b class="link-name">{labelOf(c, sections)}</b>{/if}
  </button>
{/snippet}

{#snippet more(c: Card)}
  {@const name = labelOf(c, sections)}
  <button
    type="button"
    class="more-btn"
    data-ref={c.ref}
    aria-label={`more for ${name}`}
    aria-expanded={menuRef === c.ref}
    title="Size, room, phone, remove"
    onclick={() => (menuRef = menuRef === c.ref ? null : c.ref)}>⋯</button
  >
  {#if menuRef === c.ref}
    <div
      class="cmenu"
      role="group"
      aria-label={`${name} on Home`}
      use:popup={{ anchor: () => moreBox(c.ref), align: 'end' }}
    >
      {#if sizesOf(c.ref).length > 1}
        <div class="opt-row">
          <span>Show</span>
          {#each sizesOf(c.ref) as z (z)}
            <button
              type="button"
              aria-pressed={c.size === z}
              onclick={() => changeRef(c.ref, { size: z })}>{SIZE_NAMES[z]}</button
            >
          {/each}
        </div>
      {/if}
      {#if !narrow}
        <div class="opt-row">
          <span>Width</span>
          {#each Object.keys(WIDTHS) as CardWidth[] as w (w)}
            <button
              type="button"
              aria-pressed={c.cols === WIDTHS[w]}
              onclick={() => resize(c, { cols: WIDTHS[w] })}>{WIDTH_NAMES[w]}</button
            >
          {/each}
        </div>
      {/if}
      {#if !narrow || c.size !== 'glance'}
        <div class="opt-row">
          <span>Height</span>
          {#each Object.keys(HEIGHTS) as CardHeight[] as h (h)}
            <button
              type="button"
              aria-pressed={c.rows === HEIGHTS[h]}
              onclick={() => resize(c, { rows: HEIGHTS[h] })}>{HEIGHT_NAMES[h]}</button
            >
          {/each}
        </div>
      {/if}
      <div class="opt-row">
        <span>Move</span>
        {#if !narrow}
          <button type="button" aria-label={`move ${name} left`} onclick={() => nudge(c, -1, 0)}
            >←</button
          >
        {/if}
        <button type="button" aria-label={`move ${name} up`} onclick={() => nudge(c, 0, -1)}
          >↑</button
        >
        <button type="button" aria-label={`move ${name} down`} onclick={() => nudge(c, 0, 1)}
          >↓</button
        >
        {#if !narrow}
          <button type="button" aria-label={`move ${name} right`} onclick={() => nudge(c, 1, 0)}
            >→</button
          >
        {/if}
      </div>
      <label class="opt-row"
        ><input
          type="checkbox"
          checked={c.phone}
          onchange={(e) =>
            changeRef(c.ref, { phone: (e.currentTarget as HTMLInputElement).checked })}
        /> On the phone</label
      >
      <button type="button" class="remove" onclick={() => removeRef(c.ref)}>Remove from Home</button
      >
    </div>
  {/if}
{/snippet}

<div class="home" class:narrow class:writing={inEntry} bind:this={root}>
  <header class="top">
    <h1>Home</h1>
    <button
      type="button"
      class="customize"
      aria-pressed={editing}
      onclick={() => (editing = !editing)}
    >
      {editing ? 'Done' : 'Edit Home'}
    </button>
  </header>

  {#if narrow}
    <form
      class="capture"
      onsubmit={(e) => {
        e.preventDefault();
        void capture();
      }}
    >
      <input
        type="text"
        bind:value={captured}
        placeholder="Jot into today, or a web address"
        aria-label="jot into today or open a web address"
        oninput={() => (captureNote = '')}
      />
      <button type="submit" disabled={!captured.trim()}>Add</button>
    </form>
    {#if captureNote}<p class="capture-note" role="status">{captureNote}</p>{/if}
  {/if}

  {#if saveError}<p class="error" role="alert">{saveError}</p>{/if}

  {#if editing}
    <section class="edit" aria-label="edit home">
      <p class="note">
        {set
          ? 'Your Home. Each card: its size, whether it is on the phone, its place.'
          : 'Home is made from what you use until you change it. Changing anything here makes it yours.'}
      </p>
      <ol class="list" aria-label="home cards">
        {#each cards as c, i (c.ref)}
          {@const name = labelOf(c, sections)}
          <li
            class:off-phone={narrow && !c.phone}
            draggable="true"
            ondragstart={() => (dragFrom = i)}
            ondragover={(e) => e.preventDefault()}
            ondrop={() => drop(i)}
          >
            <span class="name"
              >{name}{#if narrow && !c.phone}<span class="off-note">
                  · not shown on the phone</span
                >{/if}</span
            >
            <select
              aria-label={`size of ${name}`}
              value={c.size}
              onchange={(e) =>
                change(i, { size: (e.currentTarget as HTMLSelectElement).value as CardSize })}
            >
              {#each sizesOf(c.ref) as z (z)}<option value={z}
                  >{z === 'glance' ? 'Glance' : z === 'summary' ? 'Summary' : 'Full'}</option
                >{/each}
            </select>
            {#if !narrow || c.size !== 'glance'}
              {#if !narrow}
                <select
                  aria-label={`width of ${name}`}
                  value={String(c.cols)}
                  onchange={(e) =>
                    resize(c, { cols: Number((e.currentTarget as HTMLSelectElement).value) })}
                >
                  {#each Object.keys(WIDTHS) as CardWidth[] as w (w)}<option
                      value={String(WIDTHS[w])}>{WIDTH_NAMES[w]}</option
                    >{/each}
                  {#if !Object.values(WIDTHS).includes(c.cols)}<option value={String(c.cols)}
                      >{c.cols} columns</option
                    >{/if}
                </select>
              {/if}
              <select
                aria-label={`height of ${name}`}
                value={String(c.rows)}
                onchange={(e) =>
                  resize(c, { rows: Number((e.currentTarget as HTMLSelectElement).value) })}
              >
                {#each Object.keys(HEIGHTS) as CardHeight[] as h (h)}<option
                    value={String(HEIGHTS[h])}>{HEIGHT_NAMES[h]}</option
                  >{/each}
                {#if !Object.values(HEIGHTS).includes(c.rows)}<option value={String(c.rows)}
                    >{c.rows} rows</option
                  >{/if}
              </select>
            {/if}
            <label class="phone"
              ><input
                type="checkbox"
                checked={c.phone}
                aria-label={`${name} on the phone`}
                onchange={(e) =>
                  change(i, { phone: (e.currentTarget as HTMLInputElement).checked })}
              /> Show on the phone</label
            >
            <button
              type="button"
              aria-label={`move ${name} up`}
              onclick={() => move(i, -1)}
              disabled={i === 0}>↑</button
            >
            <button
              type="button"
              aria-label={`move ${name} down`}
              onclick={() => move(i, 1)}
              disabled={i === cards.length - 1}>↓</button
            >
            <button type="button" aria-label={`remove ${name}`} onclick={() => remove(i)}>✕</button>
          </li>
        {/each}
      </ol>
      <div class="add">
        <label
          >Add
          <select bind:value={adding} aria-label="card to add">
            <option value="">Choose…</option>
            {#each catalogue as g (g.group)}
              {#if g.items.length}
                <optgroup label={g.group}>
                  {#each g.items as it (it.ref)}
                    <option value={it.ref} disabled={cards.some((c) => c.ref === it.ref)}
                      >{it.label}</option
                    >
                  {/each}
                </optgroup>
              {/if}
            {/each}
          </select>
        </label>
        <button
          type="button"
          disabled={!adding}
          onclick={() => {
            const it = catalogue.flatMap((g) => g.items).find((x) => x.ref === adding);
            add(adding, it?.size);
            adding = '';
          }}>Add to Home</button
        >
        {#if !narrow}
          <button
            type="button"
            title="Close the gaps: every card moves up and left as far as it fits"
            onclick={() => savePlaces(tidy(cards))}>Tidy up</button
          >
        {/if}
        <button type="button" class="reset" onclick={reset} disabled={!set}
          >Reset to the default</button
        >
      </div>
      <label class="words"
        >Your own words at the top of Home
        <textarea
          rows="3"
          bind:value={words}
          onblur={() => words !== doc.text && void save({ cards: own(), text: words })}></textarea>
      </label>
    </section>
  {/if}

  {#if placed.glance.length}
    <ul class="glance" aria-label="at a glance">
      {#each placed.glance as c (c.ref)}
        {@const fam = c.ref.split(':')[0]}
        {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
        <li class="tile">
          <button type="button" onclick={() => whole(c)?.()}>
            {#if fam === 'stat'}
              {@const st = stat(arg)}
              <b>{st.week}{st.trend ? ` ${st.trend}` : ''}</b><span>{labelOf(c, sections)}</span>
            {:else if fam === 'board'}
              {@const b = board(c.ref)}
              <b>{b.open}</b><span
                >open in {sectionOf(arg)?.label ?? arg}{b.overdue
                  ? ` · ${b.overdue} overdue`
                  : ''}</span
              >
            {:else if c.ref === 'due'}
              <b>{due.length}</b><span
                >due today{overdueDue.length ? ` · ${overdueDue.length} overdue` : ''}</span
              >
            {:else if c.ref === 'justin'}
              <b>{justIn.length}</b><span>came in this week</span>
            {/if}
          </button>
          {@render more(c)}
        </li>
      {/each}
    </ul>
  {/if}

  {#if placed.watch.length}
    <ul class="watch" aria-label="kept an eye on">
      {#each placed.watch as c (c.ref)}
        {@const fam = c.ref.split(':')[0]}
        {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
        <li class="tile">
          <button type="button" onclick={() => whole(c)?.()}>
            {#if fam === 'note'}{c.label || notes[c.ref]?.title || 'A note'}
            {:else if fam === 'section'}{labelOf(c, sections)} <span class="n">{count(arg)}</span>
            {:else}{labelOf(c, sections)}{/if}
          </button>
          {@render more(c)}
        </li>
      {/each}
    </ul>
  {/if}

  {#if doc.text}<p class="words-shown">{doc.text}</p>{/if}

  <!-- **The grid** (2026-11-23; placed, 2026-11-25): on a wide screen every card at its place,
       width and height, figures and links among them; on a narrow one, the phone's cards in the
       list's order. Each has a ⠿ grip to move it (the arrows too) and edges to size it. -->
  <ol
    class="grid"
    class:arranging={!!moving || !!resizing}
    aria-label="home cards shown"
    style={`--row:${ROW_REM}rem;--gap:${GAP_REM}rem`}
  >
    {#each placed.main as c (c.ref)}
      {@const fam = c.ref.split(':')[0]}
      {@const arg = c.ref.slice(c.ref.indexOf(':') + 1)}
      {@const full = c.size === 'full'}
      {@const open = whole(c)}
      {@const b = boxOf(c.ref)}
      {@const small = c.size === 'glance' && c.ref !== 'journal'}
      {@const name = labelOf(c, sections)}
      <li
        class="card"
        class:journal-card={c.ref === 'journal'}
        class:tile-card={small}
        data-ref={c.ref}
        data-place={b && !narrow ? `${b.x},${b.y},${b.cols},${b.rows}` : undefined}
        onfocusin={(e) => c.ref === 'journal' && enterWriting(e)}
        onfocusout={(e) => c.ref === 'journal' && leaveWriting(e)}
        class:resizing={resizing?.ref === c.ref}
        class:moving={moving?.ref === c.ref || lifted === c.ref}
        class:dropping={dropOn === c.ref}
        style={narrow || !b
          ? `--h:${rowsRem(c.rows)}rem`
          : `grid-column:${b.x + 1} / span ${b.cols};grid-row:${b.y + 1} / span ${b.rows}`}
      >
        <button
          type="button"
          class="move-grip"
          aria-label={`move ${name}`}
          title={narrow ? 'Hold and drag to move' : 'Drag to move (or the arrow keys)'}
          onpointerdown={(e) => lift(e, c)}
          onkeydown={(e) => onGripKey(e, c)}>⠿</button
        >
        {#if small}
          {@render tile(c)}
        {:else if c.ref !== 'journal'}
          <h3>
            {#if open}<button type="button" class="title" onclick={open}>{name}</button>
            {:else}{name}{/if}
          </h3>
        {/if}
        {@render more(c)}
        {#if !small}
          <div class="body">
            {#if c.ref === 'journal'}
              <JournalEntry bind:this={entry} day={today} {onopen} compact={narrow} />
              {#if full}
                <button type="button" class="more-link" onclick={() => go('/f/today')}
                  >Earlier days</button
                >
              {/if}
            {:else if c.ref === 'due'}
              {#if due.length}
                <ul class="mini">
                  {#each due.slice(0, full ? 30 : 6) as n (n.id)}
                    <li>
                      <button type="button" class="link" onclick={() => onopen(n.id)}
                        >{title(n)}</button
                      >
                      {#if isOverdue(n)}<span class="why">overdue</span>{/if}
                    </li>
                  {/each}
                </ul>
              {:else}<p class="muted">Nothing planned or due today.</p>{/if}
            {:else if c.ref === 'justin'}
              {#if justIn.length}
                <ul class="mini">
                  {#each justIn.slice(0, full ? 20 : 6) as n (n.id)}
                    <li>
                      <button type="button" class="link" onclick={() => onopen(n.id)}
                        >{title(n)}</button
                      >
                    </li>
                  {/each}
                </ul>
              {:else}<p class="muted">Pages, songs and meetings you bring in show here.</p>{/if}
            {:else if c.ref === 'continue'}
              {#if carryOn}
                <button
                  type="button"
                  class="link strong"
                  onclick={() => carryOn && onopen(carryOn.id)}>{title(carryOn)}</button
                >
                {#if carryOn.preview && carryOn.preview !== carryOn.title}<p class="preview">
                    {carryOn.preview}
                  </p>{/if}
              {:else}<p class="muted">The note you were in last shows here.</p>{/if}
            {:else if c.ref === 'recent'}
              <ul class="mini">
                {#each recent.slice(0, full ? 15 : 6) as n (n.id)}
                  <li>
                    <button type="button" class="link" onclick={() => onopen(n.id)}
                      >{title(n)}</button
                    >
                  </li>
                {/each}
              </ul>
            {:else if c.ref === 'pinned'}
              {#if pinned.length}
                <ul class="mini">
                  {#each pinned as n (n.id)}
                    <li>
                      <button type="button" class="link" onclick={() => onopen(n.id)}
                        >{title(n)}</button
                      >
                    </li>
                  {/each}
                </ul>
              {:else}<p class="muted">Tag a note <code>pinned</code> to keep it here.</p>{/if}
            {:else if c.ref === 'start'}
              <ul class="steps">
                {#each steps as st (st.label)}
                  <li class:done={st.done}>
                    <span aria-hidden="true">{st.done ? '✓' : '○'}</span>
                    <button type="button" class="link" onclick={st.go}>{st.label}</button>
                  </li>
                {/each}
              </ul>
            {:else if fam === 'heat'}
              {@const hm = heat(arg, full ? 53 : 26)}
              <div
                class="heat"
                style={`--weeks:${hm.cols.length}`}
                role="img"
                aria-label={`${labelOf(c, sections)}: ${hm.total} in the last ${full ? 'year' : 'six months'}`}
              >
                {#each hm.cols as col, i (i)}
                  <div class="hcol">
                    {#each col as d (d.day)}
                      <span
                        class={`hcell l${d.level}`}
                        class:future={d.future}
                        title={`${d.day}: ${d.n}`}
                      ></span>
                    {/each}
                  </div>
                {/each}
              </div>
              <p class="figure small">
                <b>{hm.total}</b> in the last {full ? 'year' : 'six months'}
              </p>
            {:else if fam === 'stat'}
              {@const days = perDay(arg, full ? 30 : 7)}
              {@const top = Math.max(1, ...days)}
              <p class="figure"><b>{sum(days)}</b> in the last {full ? 30 : 7} days</p>
              <div class="bars" aria-hidden="true">
                {#each days as d, i (i)}<span
                    style={`height:${Math.round((d / top) * 100)}%`}
                    title={`${d}`}
                  ></span>{/each}
              </div>
            {:else if fam === 'board'}
              {@const b = board(c.ref)}
              <p class="figure">
                {#each b.columns.filter((col) => col.status !== 'done') as col, k (col.status)}{k
                    ? ' · '
                    : ''}{col.cards.length}
                  {col.status}{/each}
              </p>
              {#if full}
                <div class="columns">
                  {#each b.columns as col (col.status)}
                    <div class="column">
                      <h4>{col.status} <span class="n">{col.cards.length}</span></h4>
                      {#each col.cards.slice(0, 10) as n (n.id)}
                        <button type="button" class="chip-card" onclick={() => onopen(n.id)}
                          >{title(n)}</button
                        >
                      {/each}
                    </div>
                  {/each}
                </div>
              {:else if b.look.length}
                <ul class="mini">
                  {#each b.look.slice(0, 6) as x (x.n.id)}
                    <li>
                      <button type="button" class="link" onclick={() => onopen(x.n.id)}
                        >{title(x.n)}</button
                      >
                      <span class="why">{x.why}</span>
                    </li>
                  {/each}
                </ul>
              {:else}<p class="muted">Nothing overdue, blocked or stuck.</p>{/if}
            {:else if fam === 'section' || fam === 'view'}
              {#if rows[c.ref]?.length}
                <ul class="mini">
                  {#each rows[c.ref].slice(0, full ? 20 : 6) as n (n.id)}
                    <li>
                      <button type="button" class="link" onclick={() => onopen(n.id)}
                        >{title(n)}</button
                      >
                    </li>
                  {/each}
                </ul>
              {:else}<p class="muted">Nothing here yet.</p>{/if}
            {:else if fam === 'note'}
              {@const n = notes[c.ref]}
              {#if n}
                <p class="preview">{full ? n.body : n.body.slice(0, 280)}</p>
              {:else}<p class="muted">This note is not here.</p>{/if}
            {/if}
          </div>
        {/if}
        {#if !narrow}
          <span
            class="grip x"
            role="separator"
            aria-label={`width of ${labelOf(c, sections)}: drag`}
            onpointerdown={(e) => grip(e, c, 'x')}
          ></span>
          <span
            class="grip y"
            role="separator"
            aria-label={`height of ${labelOf(c, sections)}: drag`}
            onpointerdown={(e) => grip(e, c, 'y')}
          ></span>
          <span class="grip xy" aria-hidden="true" onpointerdown={(e) => grip(e, c, 'xy')}></span>
        {/if}
      </li>
    {/each}
  </ol>
</div>

<style>
  .home {
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .home.narrow {
    padding: var(--space-3) var(--space-3) var(--space-5);
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
  }
  .customize {
    margin-left: auto;
    font: inherit;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    cursor: pointer;
  }
  .edit {
    border: 1px dashed var(--accent);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .edit .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  /* Every control stays on the screen: a row wraps, the name on a line of its own on a phone. */
  .list li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: 3px 0;
    border-bottom: 1px dashed var(--border);
    cursor: grab;
  }
  .list .name {
    flex: 1;
    min-width: 8rem;
  }
  .narrow .list .name {
    flex-basis: 100%;
  }
  .list select {
    max-width: 100%;
    min-width: 0;
  }
  .list li.off-phone .name {
    color: var(--text-muted);
  }
  .off-note {
    font-size: var(--text-xs);
  }
  .list button {
    border: 1px solid var(--border);
    background: var(--bg);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .phone {
    font-size: var(--text-sm);
    white-space: nowrap;
  }
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .add .reset {
    margin-left: auto;
  }
  .words {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }
  .words textarea {
    font: inherit;
  }
  .glance,
  .watch {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: var(--space-2);
  }
  .glance {
    flex-wrap: wrap;
  }
  .glance button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 9rem;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-align: left;
  }
  .glance b {
    font-size: var(--text-xl);
  }
  .glance span {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  /* The watch row is one line that scrolls sideways (a phone keeps it to one line). */
  .watch {
    overflow-x: auto;
    flex-wrap: nowrap;
  }
  .watch button {
    white-space: nowrap;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .n {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  .words-shown {
    margin: 0;
    white-space: pre-line;
  }
  /* **The grid** (2026-11-23): twelve columns; each card spans its width and holds its height,
     its content scrolling inside. A narrow screen has one column: every card full width. */
  /* Rows of one height (`ROW_REM`), so a card's place and height are whole rows; each card is
     put at its column and row by `arrange` (`lib/home.ts`). A narrow screen has one column, the
     cards in the list's order, each at its own height. */
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    grid-auto-rows: var(--row, 2.5rem);
    gap: var(--gap, 0.75rem);
    align-items: stretch;
  }
  .narrow .grid {
    grid-template-columns: minmax(0, 1fr);
    grid-auto-rows: auto;
  }
  /* While a card is moved or sized, the grid's rows show, so its snapping can be seen. */
  .grid.arranging {
    background-image: linear-gradient(
      to bottom,
      color-mix(in srgb, var(--accent) 7%, transparent) var(--row),
      transparent var(--row)
    );
    background-size: 100% calc(var(--row) + var(--gap));
    border-radius: var(--radius-md);
  }
  .card {
    position: relative;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    min-width: 0;
    overflow: hidden;
  }
  .narrow .card {
    grid-column: 1 / -1;
    height: var(--h, 14.5rem);
  }
  .card.moving {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    box-shadow: var(--shadow-lg, 0 8px 24px rgb(0 0 0 / 0.2));
    z-index: 5;
  }
  .narrow .card.dropping {
    box-shadow: 0 -4px 0 var(--accent);
  }
  /* ⠿ in a card's top left corner: held and dragged, the card moves; its arrows move it too. On a
     touch screen it waits for a short hold, so the page still scrolls under a finger. */
  .move-grip {
    position: absolute;
    top: var(--space-2);
    left: 4px;
    z-index: 4;
    border: none;
    background: none;
    color: var(--text-muted);
    font: inherit;
    line-height: 1;
    padding: 2px 3px;
    border-radius: var(--radius-sm);
    cursor: grab;
    touch-action: none;
    opacity: 0;
  }
  .card:hover .move-grip,
  .move-grip:focus-visible,
  .card.moving .move-grip,
  .narrow .move-grip {
    opacity: 1;
  }
  .move-grip:active {
    cursor: grabbing;
  }
  @media (pointer: coarse) {
    .move-grip {
      opacity: 1;
      min-width: 2rem;
      min-height: 2rem;
      top: 2px;
      left: 0;
    }
  }
  .card h3 {
    padding-left: 1.1rem;
    padding-right: 1.6rem;
  }
  @media (pointer: coarse) {
    .card h3 {
      padding-left: 1.6rem;
    }
  }
  /* A figure or a link on the grid: a small card, its number and words, the grip and ⋯ beside. */
  .card.tile-card {
    padding: var(--space-2) var(--space-3) var(--space-2) 1.4rem;
    justify-content: center;
  }
  .tile-btn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 0 1.4rem 0 0;
    border: none;
    background: none;
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-align: left;
    min-width: 0;
    overflow: hidden;
  }
  .tile-btn b {
    font-size: var(--text-xl);
  }
  .tile-btn b.link-name {
    font-size: var(--text-md);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .tile-btn span {
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .card.resizing {
    outline: 2px dashed var(--accent);
    outline-offset: 2px;
  }
  .card.dropping {
    box-shadow: inset 4px 0 0 var(--accent);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* The journal card is the entry itself: its own frame, filling the card's room. */
  .card.journal-card {
    padding: 0;
    border: none;
    background: none;
  }
  /* The entry's own header leaves room for the card's ⋯ in its corner. */
  .journal-card .body :global(.entry header) {
    padding-right: 1.8rem;
    padding-left: 1.4rem;
  }
  .journal-card .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .journal-card .body :global(.entry) {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  /* The edges to drag: the right one for the width, the bottom one for the height. */
  .grip {
    position: absolute;
    z-index: 2;
  }
  .grip.x {
    top: 0;
    right: 0;
    width: 8px;
    height: 100%;
    cursor: ew-resize;
  }
  .grip.y {
    left: 0;
    bottom: 0;
    width: 100%;
    height: 8px;
    cursor: ns-resize;
  }
  .grip.xy {
    right: 0;
    bottom: 0;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
    z-index: 3;
  }
  .card:hover .grip.xy {
    background: linear-gradient(135deg, transparent 50%, var(--border) 50%);
    border-bottom-right-radius: var(--radius-md);
  }
  /* ⋯ on every card and figure: size, room, the phone, remove. */
  .more-btn {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    z-index: 4;
    border: none;
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-md);
    line-height: 1;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    opacity: 0;
  }
  .card:hover .more-btn,
  .tile:hover .more-btn,
  .more-btn:focus-visible,
  .more-btn[aria-expanded='true'],
  .narrow .more-btn {
    opacity: 1;
  }
  .more-btn:hover {
    background: var(--card-bg);
    color: var(--text);
  }
  .cmenu {
    position: absolute;
    top: 2.2rem;
    right: var(--space-2);
    z-index: 20;
    min-width: 15rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md, 0 4px 16px rgb(0 0 0 / 0.18));
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text);
  }
  .opt-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .opt-row > span {
    width: 3.6rem;
    color: var(--text-muted);
  }
  .opt-row button {
    font: inherit;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  .opt-row button[aria-pressed='true'] {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .cmenu .remove {
    margin-top: var(--space-1);
    font: inherit;
    text-align: left;
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--danger-fg);
    cursor: pointer;
  }
  .cmenu .remove:hover {
    background: var(--card-bg);
  }
  /* A figure's or a link's ⋯ sits beside it, never over it (the layout guard, 2026-11-23). */
  .tile {
    position: relative;
    display: flex;
    align-items: flex-start;
  }
  .tile .cmenu {
    top: 100%;
    left: 0;
    right: auto;
  }
  .tile .more-btn {
    position: static;
    align-self: center;
  }
  /* A heatmap, as GitHub draws one: a column per week, Monday at the top. The squares share the
     card's width, so a year always fits; they stay square. */
  .heat {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    gap: 2px;
    max-width: calc(var(--weeks, 26) * 15px);
  }
  .hcol {
    display: grid;
    grid-template-rows: repeat(7, auto);
    gap: 2px;
  }
  .hcell {
    aspect-ratio: 1;
    border-radius: 2px;
    background: color-mix(in srgb, var(--text) 9%, transparent);
  }
  .hcell.l1 {
    background: color-mix(in srgb, var(--accent) 32%, var(--surface));
  }
  .hcell.l2 {
    background: color-mix(in srgb, var(--accent) 55%, var(--surface));
  }
  .hcell.l3 {
    background: color-mix(in srgb, var(--accent) 78%, var(--surface));
  }
  .hcell.l4 {
    background: var(--accent);
  }
  .hcell.future {
    visibility: hidden;
  }
  .figure.small {
    margin-top: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .figure.small b {
    font-size: var(--text-md);
    color: var(--text);
  }
  .more-link {
    align-self: flex-start;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .card h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }
  .title {
    font: inherit;
    font-weight: inherit;
    color: inherit;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .title:hover {
    text-decoration: underline;
  }
  .steps,
  .mini {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .steps li {
    display: flex;
    gap: var(--space-2);
    padding: 2px 0;
  }
  .steps li.done .link {
    color: var(--text-muted);
    text-decoration: line-through;
  }
  .mini li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 2px 0;
    border-bottom: 1px dashed var(--border);
  }
  .link {
    font: inherit;
    color: var(--text);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link.strong {
    font-weight: 600;
  }
  .why {
    font-size: var(--text-xs);
    color: var(--danger-fg);
    white-space: nowrap;
  }
  .figure {
    margin: 0 0 var(--space-2);
  }
  .figure b {
    font-size: var(--text-xl);
  }
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 3rem;
  }
  .bars span {
    flex: 1;
    min-height: 2px;
    background: var(--accent);
    border-radius: 2px 2px 0 0;
    opacity: 0.75;
  }
  .columns {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(10rem, 1fr);
    gap: var(--space-2);
    overflow-x: auto;
  }
  .column h4 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    text-transform: capitalize;
  }
  .chip-card {
    display: block;
    width: 100%;
    text-align: left;
    margin-bottom: 4px;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    font: inherit;
    font-size: var(--text-sm);
    color: inherit;
    cursor: pointer;
  }
  .more {
    align-self: flex-start;
    font: inherit;
    font-size: var(--text-sm);
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .muted {
    color: var(--text-muted);
    margin: 0;
  }
  .preview {
    margin: var(--space-1) 0 0;
    white-space: pre-line;
  }
  .error {
    color: var(--danger-fg);
  }
  /* The capture bar stays at the bottom, within a thumb's reach. */
  /* Writing on the phone: the entry fills the room above the keyboard; the jot bar gives way. */
  .home.narrow.writing .card.journal-card {
    height: calc(var(--app-h, 100dvh) - var(--space-4));
  }
  .home.writing .capture {
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
  }
  @media (prefers-reduced-motion: no-preference) {
    .card.journal-card {
      transition: height 0.22s ease;
    }
    .capture {
      transition:
        opacity 0.18s ease,
        visibility 0.18s;
    }
  }
  /* The jot box sits under Home's title: in the page, never floating over it (2026-11-23). */
  .capture {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) 0;
  }
  .capture input {
    flex: 1;
    font: inherit;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: inherit;
  }
  .capture button {
    font: inherit;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-fg, #fff);
  }
  .capture-note {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
</style>
