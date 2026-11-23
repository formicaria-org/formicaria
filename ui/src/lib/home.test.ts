// **Home's model** (`home.ts`, 2026-11-23): cards in the Home note, read and written; 0.61.0's Home
// read as cards; the Home nobody set up; where each card goes on a wide and a narrow screen; Pin to Home.
import { beforeEach, expect, it } from 'vitest';
import * as mock from './mock';
import { generated, layout, parse, pinToHome, readHome, write, type Card } from './home';
import type { SectionSummary } from './types';

beforeEach(() => mock.reset());

it('cards and the person’s words survive a write and a read', () => {
  const doc = {
    text: 'This term: the thesis.',
    cards: [
      { ref: 'journal', label: '', size: 'full', phone: true },
      { ref: 'board:tasks', label: 'Lab board', size: 'summary', phone: false },
      { ref: 'note:01JD0000000000000000000000', label: '', size: 'glance', phone: true },
    ] as Card[],
  };
  expect(parse(write(doc))).toEqual(doc);
});

it('a Home set up in 0.61.0 is read as cards, nothing lost', () => {
  const old = '![Today](view:today)\n![Menu](view:Menu)\n![](note:01JD0000000000000000000000)\n';
  expect(parse(old).cards).toEqual([
    { ref: 'due', label: '', size: 'summary', phone: true },
    { ref: 'view:Menu', label: 'Menu', size: 'summary', phone: false },
    { ref: 'note:01JD0000000000000000000000', label: '', size: 'summary', phone: false },
  ]);
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

it('a narrow screen shows only the phone’s cards, three figures at most, nothing full', () => {
  const c = (ref: string, size: Card['size'], phone: boolean): Card => ({
    ref,
    label: '',
    size,
    phone,
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
  expect(wide.glance.map((x) => x.ref)).toHaveLength(4);
  expect(wide.main.map((x) => x.ref)).toEqual(['board:tasks', 'justin']);
  const narrow = layout(cards, true);
  expect(narrow.glance).toHaveLength(3);
  expect(narrow.watch.map((x) => x.ref)).toEqual(['note:A']);
  expect(narrow.journal?.ref).toBe('journal');
  expect(narrow.main).toEqual([c('board:tasks', 'summary', true)]);
});

it('Pin to Home keeps the generated Home and adds the pin once', async () => {
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
  });
});
