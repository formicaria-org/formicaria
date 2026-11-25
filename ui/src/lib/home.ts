// **Home: a glance, a journal, cards** (`unifying-home-research-2026-11-22.md` §12, 2026-11-23).
//
// Home is still one note tagged `home` (it syncs and keeps its history), now a list of **cards**,
// one per line, plus the person's own words:
//
//     ![Due and planned](card:due?size=summary&phone=1)
//     ![Lab board](card:board:tasks?size=summary&phone=0)
//     ![](card:note:01J…?size=glance&phone=1)
//
// Each card is shown at one of three sizes, **glance** (a figure or a link), **summary** (what needs
// a look) or **full** (the whole thing), and is on the phone or not. Lines that are not cards are
// the person's own words, kept as written. 0.61.0's lines (`![Today](view:today)`, `![](note:<id>)`)
// are read as cards, so a Home set up then is not lost.
//
// **Nothing set up is a Home too**: with no cards, Home is generated from what the person uses (the
// journal, what is due, a board, the figures of the busiest sections), and keeps up with them until
// the first edit. Pinning (`pinToHome`) is the customizing most people do (NN/g; Apple Health's
// Pinned; Home Assistant's Favorites), so it works from anywhere, not only on Home.
import * as ipc from './ipc';
import type { Section, SectionSummary } from './types';

export type CardSize = 'glance' | 'summary' | 'full';
/** **The room a card takes, and where** (2026-11-23, *"I cannot select the space they take"*;
 *  2026-11-25, *"why can't I move the options I add to home wherever I want … including size and
 *  position"*). Home on a wide screen is a grid of 12 columns and rows of [`ROW_REM`]: a card has
 *  a width in columns (1–12), a height in rows, and a place (`x` its column, `y` its row) — all
 *  chosen by dragging (it snaps) or from the card's ⋯. A card with no place is put in the first
 *  gap it fits, in the order of the list (which is also the phone's order). On a narrow screen
 *  every card is the full width, in the list's order; its height still holds. */
export type CardWidth = 'narrow' | 'half' | 'wide' | 'full';
export type CardHeight = 'short' | 'medium' | 'tall';
/** Named widths, in columns: what the ⋯ offers, and what an older Home note says. */
export const WIDTHS: Record<CardWidth, number> = { narrow: 4, half: 6, wide: 8, full: 12 };
/** Named heights, in rows. */
export const HEIGHTS: Record<CardHeight, number> = { short: 3, medium: 5, tall: 8 };
/** The grid: 12 columns; a row is this tall, and cards are this far apart (both in rem). */
export const COLS = 12;
export const ROW_REM = 2.5;
export const GAP_REM = 0.75;
export const MIN_COLS = 2;
export const MIN_ROWS = 2;
export const MAX_ROWS = 24;
/** A card's height in rem, from its rows (the gaps between them included). */
export const rowsRem = (rows: number) => rows * ROW_REM + (rows - 1) * GAP_REM;
export interface Card {
  /** What it shows: a built-in (`journal`, `due`…) or `stat:`/`board:`/`section:`/`view:`/`note:`. */
  ref: string;
  /** Its heading, as the person named it; empty means the default name. */
  label: string;
  size: CardSize;
  phone: boolean;
  /** Its width in columns and its height in rows. */
  cols: number;
  rows: number;
  /** Its place on the wide grid, once it has one: column (0–11) and row. */
  x?: number;
  y?: number;
}
export interface HomeDoc {
  cards: Card[];
  /** The person's own words: every line that is not a card, as written. */
  text: string;
}

const ALL: CardSize[] = ['glance', 'summary', 'full'];

/** The cards that are not about one section, view or note. */
export const BUILTIN: Record<
  string,
  { label: string; sizes: CardSize[]; group: string; phone: boolean }
> = {
  journal: { label: 'Journal', sizes: ['summary', 'full'], group: 'Today', phone: true },
  due: { label: 'Due and planned', sizes: ALL, group: 'Today', phone: true },
  justin: { label: 'Came in', sizes: ALL, group: 'Today', phone: false },
  continue: { label: 'Continue', sizes: ['summary'], group: 'Today', phone: true },
  recent: { label: 'Recent', sizes: ['summary', 'full'], group: 'Your notes', phone: false },
  pinned: { label: 'Pinned', sizes: ['summary', 'full'], group: 'Your notes', phone: false },
  start: { label: 'Getting started', sizes: ['summary'], group: 'Your notes', phone: false },
};

/** 0.61.0's built-in blocks (`view:<name>`), as cards. */
const LEGACY: Record<string, string> = {
  today: 'due',
  justin: 'justin',
  recent: 'recent',
  pinned: 'pinned',
  start: 'start',
};

const family = (ref: string) => (ref.includes(':') ? ref.slice(0, ref.indexOf(':')) : ref);
const rest = (ref: string) => ref.slice(ref.indexOf(':') + 1);

/** The sizes a card offers. */
export function sizesOf(ref: string): CardSize[] {
  if (BUILTIN[ref]) return BUILTIN[ref].sizes;
  if (family(ref) === 'heat') return ['summary', 'full'];
  return ['stat', 'board', 'section', 'view', 'note'].includes(family(ref)) ? ALL : ['summary'];
}

/** A card's heading: its own label, or the name its kind of card gives it. */
export function labelOf(card: Card, sections: Section[]): string {
  if (card.label) return card.label;
  if (BUILTIN[card.ref]) return BUILTIN[card.ref].label;
  const s = sections.find((x) => x.key === rest(card.ref));
  switch (family(card.ref)) {
    case 'stat':
      return `${s?.label ?? rest(card.ref)} this week`;
    case 'heat':
      return `${s?.label ?? rest(card.ref)}, day by day`;
    case 'board':
      return `${s?.label ?? rest(card.ref)} board`;
    case 'section':
      return s?.label ?? rest(card.ref);
    case 'view':
      return rest(card.ref);
    default:
      return 'A note';
  }
}

/** The room a card starts with: the journal tall, a heatmap wide, a figure or a link small, the
 *  rest half. */
export function defaultRoom(
  ref: string,
  size: CardSize = 'summary',
): { cols: number; rows: number } {
  if (size === 'glance' && ref !== 'journal') return { cols: 3, rows: 2 };
  if (ref === 'journal') return { cols: WIDTHS.half, rows: HEIGHTS.tall };
  if (family(ref) === 'heat') return { cols: WIDTHS.wide, rows: HEIGHTS.medium };
  if (ref === 'continue' || family(ref) === 'stat')
    return { cols: WIDTHS.half, rows: HEIGHTS.short };
  return { cols: WIDTHS.half, rows: HEIGHTS.medium };
}
const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));
const int = (v: string | null): number | null =>
  v !== null && /^\d{1,3}$/.test(v) ? Number(v) : null;

/** The named width nearest a number of columns (what an older Home reads). */
export function nearestWidth(cols: number): CardWidth {
  return (Object.keys(WIDTHS) as CardWidth[]).reduce((best, w) =>
    Math.abs(WIDTHS[w] - cols) < Math.abs(WIDTHS[best] - cols) ? w : best,
  );
}
/** The named height nearest a number of rows. */
export function nearestHeight(rows: number): CardHeight {
  return (Object.keys(HEIGHTS) as CardHeight[]).reduce((best, h) =>
    Math.abs(HEIGHTS[h] - rows) < Math.abs(HEIGHTS[best] - rows) ? h : best,
  );
}

// ---- arranging the wide grid ----

/** A card's box on the grid. */
export interface Box {
  ref: string;
  x: number;
  y: number;
  cols: number;
  rows: number;
}
const overlaps = (a: Box, b: Box) =>
  a.x < b.x + b.cols && b.x < a.x + a.cols && a.y < b.y + b.rows && b.y < a.y + a.rows;

/** **Where every card goes** (2026-11-25): no two overlap and nothing floats — each card settles
 *  upward as far as it can without passing another (a dashboard's *vertical compaction*). Cards
 *  with a place keep their column and their order from the top; `pin` (the card being dragged,
 *  or moved by a key) is put first at its place, so the others make way for it. Cards with no
 *  place go, in their order, into the first gap they fit, top to bottom and left to right. */
export function arrange(cards: Card[], pin?: Box): Box[] {
  const placed: Box[] = [];
  const free = (b: Box) => placed.every((o) => !overlaps(b, o));
  const room = (c: { cols: number; rows: number }) => ({
    cols: clamp(Math.round(c.cols), 1, COLS),
    rows: clamp(Math.round(c.rows), 1, MAX_ROWS),
  });
  const box = (ref: string, c: { cols: number; rows: number }, x: number, y: number): Box => {
    const r = room(c);
    return { ref, ...r, x: clamp(Math.round(x), 0, COLS - r.cols), y: Math.max(0, Math.round(y)) };
  };
  // The placed ones from the top, the pinned card first among those at its row: each settles.
  const at: { b: Box; first: boolean }[] = cards
    .filter((c) => c.ref !== pin?.ref && c.x !== undefined && c.y !== undefined)
    .map((c) => ({ b: box(c.ref, c, c.x!, c.y!), first: false }));
  if (pin) at.push({ b: box(pin.ref, pin, pin.x, pin.y), first: true });
  at.sort((p, q) => p.b.y - q.b.y || Number(q.first) - Number(p.first) || p.b.x - q.b.x);
  for (const { b } of at) {
    while (!free(b)) b.y++;
    while (b.y > 0 && free({ ...b, y: b.y - 1 })) b.y--;
    placed.push(b);
  }
  for (const c of cards.filter(
    (c) => c.ref !== pin?.ref && (c.x === undefined || c.y === undefined),
  )) {
    const r = room(c);
    let done = false;
    for (let y = 0; !done; y++)
      for (let x = 0; x + r.cols <= COLS && !done; x++) {
        const b = { ref: c.ref, ...r, x, y };
        if (free(b)) {
          placed.push(b);
          done = true;
        }
      }
  }
  return placed;
}

/** **Tidy up**: every card into the first gap it fits, in reading order — the holes closed. */
export function tidy(cards: Card[]): Box[] {
  const order = new Map(arrange(cards).map((b, i) => [b.ref, { b, i }]));
  const byReading = [...cards].sort((p, q) => {
    const a = order.get(p.ref)!.b;
    const b = order.get(q.ref)!.b;
    return a.y - b.y || a.x - b.x;
  });
  return arrange(byReading.map((c) => ({ ...c, x: undefined, y: undefined })));
}

/** Cards with their boxes written in: what is saved after a move, a resize or a tidy. */
export function placeAll(cards: Card[], boxes: Box[]): Card[] {
  const at = new Map(boxes.map((b) => [b.ref, b]));
  return cards.map((c) => {
    const b = at.get(c.ref);
    return b ? { ...c, x: b.x, y: b.y, cols: b.cols, rows: b.rows } : c;
  });
}

const LINE = /^!\[([^\]]*)\]\(((?:card|view|note):[^)\s]+)\)\s*$/;

/** The Home note's body → its cards and the person's words. */
export function parse(body: string): HomeDoc {
  const cards: Card[] = [];
  const words: string[] = [];
  for (const line of body.split('\n')) {
    const m = LINE.exec(line.trim());
    if (!m) {
      words.push(line);
      continue;
    }
    const [label, target] = [m[1], m[2]];
    if (target.startsWith('card:')) {
      const [ref, query = ''] = target.slice(5).split('?');
      const q = new URLSearchParams(query);
      const given = q.get('size') as CardSize | null;
      const size = given && sizesOf(ref).includes(given) ? given : 'summary';
      const room = defaultRoom(ref, size);
      // Columns and rows (`c`, `r`) when this version wrote them; else the named width and
      // height an older one did (`w`, `h`); else the card's own room.
      const w = q.get('w');
      const h = q.get('h');
      const cols = int(q.get('c')) ?? (w && w in WIDTHS ? WIDTHS[w as CardWidth] : room.cols);
      const rows = int(q.get('r')) ?? (h && h in HEIGHTS ? HEIGHTS[h as CardHeight] : room.rows);
      const x = int(q.get('x'));
      const y = int(q.get('y'));
      cards.push({
        ref,
        label,
        size,
        phone: q.get('phone') === '1',
        cols: clamp(cols, 1, COLS),
        rows: clamp(rows, 1, MAX_ROWS),
        ...(x !== null && y !== null ? { x: clamp(x, 0, COLS - 1), y } : {}),
      });
    } else if (target.startsWith('view:')) {
      const name = target.slice(5);
      const ref = LEGACY[name] ?? `view:${name}`;
      const builtinLabel = BUILTIN[ref]?.label;
      cards.push({
        ref,
        // 0.61.0 wrote its own names (`Today`); the card's name is the one to show now.
        label: LEGACY[name] ? '' : label === builtinLabel ? '' : label,
        size: 'summary',
        phone: BUILTIN[ref]?.phone ?? false,
        ...defaultRoom(ref),
      });
    } else {
      cards.push({ ref: target, label, size: 'summary', phone: false, ...defaultRoom(target) });
    }
  }
  return { cards, text: words.join('\n').trim() };
}

/** Cards and words → the Home note's body. The words first, then one card per line. The named
 *  width and height are written beside the columns and rows, so a device on an older version
 *  still shows the card about as wide and as tall. */
export function write(doc: HomeDoc): string {
  const lines = doc.cards.map((c) => {
    const place = c.x !== undefined && c.y !== undefined ? `&x=${c.x}&y=${c.y}` : '';
    return (
      `![${c.label}](card:${c.ref}?size=${c.size}&phone=${c.phone ? 1 : 0}` +
      `&w=${nearestWidth(c.cols)}&h=${nearestHeight(c.rows)}&c=${c.cols}&r=${c.rows}${place})`
    );
  });
  return (doc.text.trim() ? `${doc.text.trim()}\n\n` : '') + lines.join('\n') + '\n';
}

const count = (summary: SectionSummary[], key: string) =>
  summary.find((s) => s.key === key)?.count ?? 0;

/** **The Home nobody set up**: built from what the person uses, so it is good on the first day and
 *  keeps up until the first edit (Home Assistant's generated overview; NN/g on defaults). */
export function generated(summary: SectionSummary[]): Card[] {
  const card = (ref: string, size: CardSize, phone: boolean): Card => ({
    ref,
    label: '',
    size,
    phone,
    ...defaultRoom(ref, size),
  });
  const out: Card[] = [card('journal', 'summary', true)];
  // The busiest sections of the last month, as figures (at most three).
  const busy = summary
    .filter((s) => (s.recent?.length ?? 0) > 0 && s.key !== 'agenda')
    .sort((a, b) => (b.recent?.length ?? 0) - (a.recent?.length ?? 0))
    .slice(0, 3);
  for (const s of busy) out.push(card(`stat:${s.key}`, 'glance', true));
  if (count(summary, 'tasks') + count(summary, 'agenda') > 0)
    out.push(card('due', 'summary', true));
  if (count(summary, 'tasks') > 0) out.push(card('board:tasks', 'summary', false));
  if (count(summary, 'web') + count(summary, 'music') + count(summary, 'videos') > 0)
    out.push(card('justin', 'summary', false));
  out.push(card('continue', 'summary', true));
  const notes = summary.reduce((n, s) => n + s.count, 0);
  if (notes < 25) out.push(card('start', 'summary', false));
  return out;
}

/** The size a pinned thing starts at: a link in the watch row, a figure in the glance row, a
 *  board's summary among the cards. */
export function pinSize(ref: string): CardSize {
  return family(ref) === 'board' ? 'summary' : 'glance';
}

/** Where each card goes. **On a wide screen every card is on the grid** (2026-11-25: a figure or a
 *  link is a small card, placed and sized like any other). On a narrow screen only the phone's
 *  cards, in the list's order: at most three figures and the links in their rows, and nothing at
 *  full size but the journal (`§12.3`, *calm on small screens*). */
export function layout(cards: Card[], narrow: boolean) {
  if (!narrow) {
    const journal = cards.find((c) => c.ref === 'journal') ?? null;
    return { glance: [] as Card[], watch: [] as Card[], journal, main: cards };
  }
  const shown = cards.filter((c) => c.phone);
  const figure = (c: Card) => ['stat', 'board'].includes(family(c.ref)) || BUILTIN[c.ref];
  const glance = shown.filter((c) => c.size === 'glance' && figure(c) && c.ref !== 'journal');
  const watch = shown.filter((c) => c.size === 'glance' && !figure(c));
  // The journal is a card like the others, in the grid at its place (2026-11-23).
  const main = shown
    .filter((c) => c.size !== 'glance' || c.ref === 'journal')
    .map((c) =>
      c.size === 'full' && c.ref !== 'journal' ? { ...c, size: 'summary' as CardSize } : c,
    );
  const journal = main.find((c) => c.ref === 'journal') ?? null;
  return { glance: glance.slice(0, 3), watch, journal, main };
}

/** The Home note, if there is one: its id, version and what it says. */
export async function readHome(): Promise<{ id: string; version: string; doc: HomeDoc } | null> {
  const [h] = await ipc.tagged('home', 1).catch(() => []);
  if (!h) return null;
  const d = await ipc.getNote(h.id).catch(() => null);
  return d ? { id: d.id, version: d.version, doc: parse(d.body) } : null;
}

/** Write Home: the note it is, or a new one tagged `home`. */
export async function saveHome(
  doc: HomeDoc,
  home: { id: string; version: string } | null,
): Promise<{ id: string; version: string }> {
  const body = write(doc);
  if (home) {
    const version = await ipc.updateBody(home.id, body, home.version);
    return { id: home.id, version: version || home.version };
  }
  const [made] = await ipc.createNotes([
    { title: 'Home', body, tags: ['home'], start: '', props: {} },
  ]);
  const d = made ? await ipc.getNote(made.id) : null;
  if (!d) throw new Error('Home could not be made');
  return { id: d.id, version: d.version };
}

/** **Pin to Home**, from anywhere (a note's ⋯, a section's or a view's bar). A Home nobody set up
 *  becomes its generated cards plus this one, so pinning never empties it. Answers false when it
 *  was there already. */
export async function pinToHome(ref: string, label = ''): Promise<boolean> {
  const home = await readHome();
  const doc: HomeDoc = home?.doc ?? { cards: [], text: '' };
  if (!doc.cards.length) doc.cards = generated(await ipc.sectionSummary().catch(() => []));
  if (doc.cards.some((c) => c.ref === ref)) return false;
  const size = pinSize(ref);
  doc.cards.push({ ref, label, size, phone: true, ...defaultRoom(ref, size) });
  await saveHome(doc, home);
  return true;
}

/** **Remove from Home**, from where it was pinned (the same button, once pinned). Answers false
 *  when it was not on Home. */
export async function unpinFromHome(ref: string): Promise<boolean> {
  const home = await readHome();
  if (!home || !home.doc.cards.some((c) => c.ref === ref)) return false;
  await saveHome({ ...home.doc, cards: home.doc.cards.filter((c) => c.ref !== ref) }, home);
  return true;
}

/** Whether `ref` is on Home (for the menus' *Pin to Home* / *On Home*). */
export async function onHome(ref: string): Promise<boolean> {
  const home = await readHome();
  return !!home?.doc.cards.some((c) => c.ref === ref);
}
