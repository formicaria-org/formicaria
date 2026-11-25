// **Home's model** (`home.ts`, 2026-11-23; placed, 2026-11-25): cards in the Home note, read and
// written; 0.61.0's and 0.62's Home read as cards; the Home nobody set up; where each card goes on a
// wide and a narrow screen; arranging the grid; Pin to Home.
import { beforeEach, expect, it } from 'vitest';
import * as mock from './mock';
import {
  arrange,
  generated,
  layout,
  nearestHeight,
  nearestWidth,
  parse,
  pinToHome,
  placeAll,
  readHome,
  tidy,
  unpinFromHome,
  write,
  type Box,
  type Card,
} from './home';
import type { SectionSummary } from './types';

beforeEach(() => mock.reset());

const card = (ref: string, cols: number, rows: number, at?: [number, number]): Card => ({
  ref,
  label: '',
  size: 'summary',
  phone: false,
  cols,
  rows,
  ...(at ? { x: at[0], y: at[1] } : {}),
});
const overlap = (a: Box, b: Box) =>
  a.x < b.x + b.cols && b.x < a.x + a.cols && a.y < b.y + b.rows && b.y < a.y + a.rows;
const noOverlap = (bs: Box[]) => bs.every((a, i) => bs.every((b, j) => i === j || !overlap(a, b)));
const where = (bs: Box[], ref: string) => {
  const b = bs.find((x) => x.ref === ref)!;
  return [b.x, b.y, b.cols, b.rows];
};

it('cards, their places and the person’s words survive a write and a read', () => {
  const doc = {
    text: 'This term: the thesis.',
    cards: [
      { ref: 'journal', label: '', size: 'full', phone: true, cols: 12, rows: 8, x: 0, y: 0 },
      { ref: 'board:tasks', label: 'Lab board', size: 'summary', phone: false, cols: 5, rows: 3 },
      {
        ref: 'note:01JD0000000000000000000000',
        label: '',
        size: 'glance',
        phone: true,
        cols: 3,
        rows: 2,
        x: 9,
        y: 8,
      },
    ] as Card[],
  };
  expect(parse(write(doc))).toEqual(doc);
});

it('a line keeps the named width and height for a device on an older version', () => {
  const line = write({ text: '', cards: [card('due', 7, 4, [2, 3])] });
  expect(line).toContain('&w=half&h=short&c=7&r=4&x=2&y=3');
  expect([nearestWidth(5), nearestWidth(11), nearestHeight(7)]).toEqual(['narrow', 'full', 'tall']);
});

it('a Home set up in 0.61.0 is read as cards, nothing lost', () => {
  const old = '![Today](view:today)\n![Menu](view:Menu)\n![](note:01JD0000000000000000000000)\n';
  expect(parse(old).cards).toEqual([
    { ref: 'due', label: '', size: 'summary', phone: true, cols: 6, rows: 5 },
    { ref: 'view:Menu', label: 'Menu', size: 'summary', phone: false, cols: 6, rows: 5 },
    {
      ref: 'note:01JD0000000000000000000000',
      label: '',
      size: 'summary',
      phone: false,
      cols: 6,
      rows: 5,
    },
  ]);
});

it('a Home arranged in 0.62 keeps its widths and heights, and has no places yet', () => {
  const [a, b] = parse(
    '![](card:due?size=summary&phone=1&w=narrow&h=tall)\n![](card:recent?size=full&phone=0&w=full&h=short)',
  ).cards;
  expect([a.cols, a.rows, a.x]).toEqual([4, 8, undefined]);
  expect([b.cols, b.rows]).toEqual([12, 3]);
});

it('a size a card does not offer falls back to summary', () => {
  expect(parse('![](card:continue?size=full&phone=1)').cards[0].size).toBe('summary');
});

it('the Home nobody set up is made from use: the journal, the busiest sections, what is due', () => {
  const now = new Date().toISOString();
  const s = (key: string, count: number, recent = 0): SectionSummary => ({
    key,
    count,
    latest: null,
    recent: Array(recent).fill(now),
  });
  const refs = generated([
    s('notes', 40, 9),
    s('tasks', 5, 2),
    s('food', 30, 20),
    s('music', 0),
    s('health', 3, 1),
    s('fitness', 4, 4),
  ]).map((c) => c.ref);
  expect(refs[0]).toBe('journal');
  expect(refs.filter((r) => r.startsWith('stat:'))).toEqual([
    'stat:food',
    'stat:notes',
    'stat:fitness',
  ]);
  expect(refs).toContain('due');
  expect(refs).toContain('board:tasks');
  expect(refs).not.toContain('start'); // a library this full needs no first steps
});

it('wide: every card is on the grid; narrow: the phone’s, three figures at most, nothing full', () => {
  const c = (ref: string, size: Card['size'], phone: boolean): Card => ({
    ref,
    label: '',
    size,
    phone,
    cols: 6,
    rows: 5,
  });
  const cards = [
    c('journal', 'full', true),
    c('stat:notes', 'glance', true),
    c('stat:food', 'glance', true),
    c('stat:fitness', 'glance', true),
    c('stat:health', 'glance', true),
    c('note:A', 'glance', true),
    c('board:tasks', 'full', true),
    c('justin', 'summary', false),
  ];
  const wide = layout(cards, false);
  expect(wide.glance).toEqual([]);
  expect(wide.main.map((x) => x.ref)).toEqual(cards.map((x) => x.ref));
  const narrow = layout(cards, true);
  expect(narrow.glance).toHaveLength(3);
  expect(narrow.watch.map((x) => x.ref)).toEqual(['note:A']);
  expect(narrow.journal?.ref).toBe('journal');
  expect(narrow.main).toEqual([c('journal', 'full', true), c('board:tasks', 'summary', true)]);
});

it('cards with no place fill the grid in order, each in the first gap it fits', () => {
  const bs = arrange([card('a', 6, 5), card('b', 4, 3), card('c', 2, 2), card('d', 12, 2)]);
  expect(where(bs, 'a')).toEqual([0, 0, 6, 5]);
  expect(where(bs, 'b')).toEqual([6, 0, 4, 3]);
  expect(where(bs, 'c')).toEqual([10, 0, 2, 2]); // the gap beside b
  expect(where(bs, 'd')).toEqual([0, 5, 12, 2]);
  expect(noOverlap(bs)).toBe(true);
});

it('placed cards keep their column and settle upward; nothing overlaps', () => {
  const bs = arrange([card('a', 6, 4, [0, 10]), card('b', 6, 4, [6, 3]), card('c', 6, 4, [0, 12])]);
  // a had nothing above it: it rises to the top; c stops under a; b rises too.
  expect(where(bs, 'a')).toEqual([0, 0, 6, 4]);
  expect(where(bs, 'c')).toEqual([0, 4, 6, 4]);
  expect(where(bs, 'b')).toEqual([6, 0, 6, 4]);
});

it('a card dragged onto another takes its place, and that one makes way below', () => {
  const cards = [card('a', 6, 4, [0, 0]), card('b', 6, 4, [6, 0]), card('c', 12, 3, [0, 4])];
  const bs = arrange(cards, { ref: 'c', x: 0, y: 0, cols: 6, rows: 3 });
  expect(where(bs, 'c')).toEqual([0, 0, 6, 3]);
  expect(where(bs, 'a')).toEqual([0, 3, 6, 4]);
  expect(where(bs, 'b')).toEqual([6, 0, 6, 4]);
  expect(noOverlap(bs)).toBe(true);
});

it('a card dropped past the bottom settles under the rest, never floating', () => {
  const cards = [card('a', 12, 4, [0, 0]), card('b', 6, 3, [0, 4])];
  expect(where(arrange(cards, { ref: 'b', x: 6, y: 40, cols: 6, rows: 3 }), 'b')).toEqual([
    6, 4, 6, 3,
  ]);
});

it('a card grown wider pushes the one beside it down; a place off the grid is brought back', () => {
  const cards = [card('a', 6, 4, [0, 0]), card('b', 6, 4, [6, 0])];
  const bs = arrange(cards, { ref: 'a', x: 0, y: 0, cols: 9, rows: 4 });
  expect(where(bs, 'a')).toEqual([0, 0, 9, 4]);
  expect(where(bs, 'b')).toEqual([6, 4, 6, 4]);
  expect(where(arrange([card('z', 4, 2, [11, 0])]), 'z')).toEqual([8, 0, 4, 2]);
});

it('Tidy up closes the gaps in reading order, and the places are saved into the cards', () => {
  // A hole in the top row, between a and b; c below. Settling upward leaves the hole; tidying fills it.
  const cards = [card('a', 4, 2, [0, 0]), card('b', 4, 2, [8, 0]), card('c', 4, 2, [0, 2])];
  expect(where(arrange(cards), 'b')).toEqual([8, 0, 4, 2]);
  const bs = tidy(cards);
  expect(where(bs, 'a')).toEqual([0, 0, 4, 2]);
  expect(where(bs, 'b')).toEqual([4, 0, 4, 2]);
  expect(where(bs, 'c')).toEqual([8, 0, 4, 2]);
  const saved = placeAll(cards, bs);
  expect(saved.map((c) => [c.x, c.y])).toEqual([
    [0, 0],
    [4, 0],
    [8, 0],
  ]);
});

it('Pin to Home keeps the generated Home and adds the pin once, small', async () => {
  expect(await pinToHome('view:Menu')).toBe(true);
  expect(await pinToHome('view:Menu')).toBe(false);
  const home = await readHome();
  const refs = home!.doc.cards.map((x) => x.ref);
  expect(refs[0]).toBe('journal');
  expect(refs.filter((r) => r === 'view:Menu')).toHaveLength(1);
  expect(home!.doc.cards.at(-1)).toEqual({
    ref: 'view:Menu',
    label: '',
    size: 'glance',
    phone: true,
    cols: 3,
    rows: 2,
  });
  // And off again, from the same place.
  expect(await unpinFromHome('view:Menu')).toBe(true);
  expect((await readHome())!.doc.cards.some((x) => x.ref === 'view:Menu')).toBe(false);
});

it('a Home line without a width or height takes the card’s own room', () => {
  const [j, h] = parse(
    '![](card:journal?size=summary&phone=1)\n![](card:heat:notes?size=full&phone=0)',
  ).cards;
  expect([j.cols, j.rows]).toEqual([6, 8]);
  expect([h.cols, h.rows, h.size]).toEqual([8, 5, 'full']);
});
