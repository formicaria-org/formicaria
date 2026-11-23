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
/** **The room a card takes** (2026-11-23, the owner: *"I cannot select the space they take"*):
 *  a width on a 12-column grid and a height, chosen by dragging an edge (it snaps) or from the
 *  card's ⋯. On a narrow screen every card is the full width; its height still holds. */
export type CardWidth = 'narrow' | 'half' | 'wide' | 'full';
export type CardHeight = 'short' | 'medium' | 'tall';
export const WIDTHS: Record<CardWidth, number> = { narrow: 4, half: 6, wide: 8, full: 12 };
/** Heights in rem. */
export const HEIGHTS: Record<CardHeight, number> = { short: 9, medium: 16, tall: 26 };
export interface Card {
  /** What it shows: a built-in (`journal`, `due`…) or `stat:`/`board:`/`section:`/`view:`/`note:`. */
  ref: string;
  /** Its heading, as the person named it; empty means the default name. */
  label: string;
  size: CardSize;
  phone: boolean;
  w: CardWidth;
  h: CardHeight;
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

/** The room a card starts with: the journal tall, a heatmap wide, the rest half. */
export function defaultRoom(ref: string): { w: CardWidth; h: CardHeight } {
  if (ref === 'journal') return { w: 'half', h: 'tall' };
  if (family(ref) === 'heat') return { w: 'wide', h: 'medium' };
  if (ref === 'continue' || family(ref) === 'stat') return { w: 'half', h: 'short' };
  return { w: 'half', h: 'medium' };
}
const isWidth = (v: string | null): v is CardWidth => !!v && v in WIDTHS;
const isHeight = (v: string | null): v is CardHeight => !!v && v in HEIGHTS;

/** A width from a dragged edge: the fraction of the grid it reaches, to the nearest width. */
export function snapWidth(fraction: number): CardWidth {
  const cols = fraction * 12;
  return (Object.keys(WIDTHS) as CardWidth[]).reduce((best, w) =>
    Math.abs(WIDTHS[w] - cols) < Math.abs(WIDTHS[best] - cols) ? w : best,
  );
}
/** A height from a dragged edge, in rem, to the nearest height. */
export function snapHeight(rem: number): CardHeight {
  return (Object.keys(HEIGHTS) as CardHeight[]).reduce((best, h) =>
    Math.abs(HEIGHTS[h] - rem) < Math.abs(HEIGHTS[best] - rem) ? h : best,
  );
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
      const size = q.get('size') as CardSize | null;
      const room = defaultRoom(ref);
      const w = q.get('w');
      const h = q.get('h');
      cards.push({
        ref,
        label,
        size: size && sizesOf(ref).includes(size) ? size : 'summary',
        phone: q.get('phone') === '1',
        w: isWidth(w) ? w : room.w,
        h: isHeight(h) ? h : room.h,
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

/** Cards and words → the Home note's body. The words first, then one card per line. */
export function write(doc: HomeDoc): string {
  const lines = doc.cards.map(
    (c) => `![${c.label}](card:${c.ref}?size=${c.size}&phone=${c.phone ? 1 : 0}&w=${c.w}&h=${c.h})`,
  );
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
    ...defaultRoom(ref),
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

/** Where each card goes. On a narrow screen only the phone's cards, at most three figures, and
 *  nothing at full size but the journal (`§12.3`, *calm on small screens*). */
export function layout(cards: Card[], narrow: boolean) {
  const shown = narrow ? cards.filter((c) => c.phone) : cards;
  const figure = (c: Card) => ['stat', 'board'].includes(family(c.ref)) || BUILTIN[c.ref];
  const glance = shown.filter((c) => c.size === 'glance' && figure(c) && c.ref !== 'journal');
  const watch = shown.filter((c) => c.size === 'glance' && !figure(c));
  // The journal is a card like the others, in the grid at its place (2026-11-23).
  const main = shown
    .filter((c) => c.size !== 'glance' || c.ref === 'journal')
    .map((c) =>
      narrow && c.size === 'full' && c.ref !== 'journal'
        ? { ...c, size: 'summary' as CardSize }
        : c,
    );
  const journal = main.find((c) => c.ref === 'journal') ?? null;
  return { glance: narrow ? glance.slice(0, 3) : glance.slice(0, 6), watch, journal, main };
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
  doc.cards.push({ ref, label, size: pinSize(ref), phone: true, ...defaultRoom(ref) });
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
