// A playlist's body: a numbered list of `note:` links, the rest of the note kept as it was.
import { expect, test } from 'vitest';
import { entries, move, rewrite } from './playlist';

const A = '01ARZ3NDEKTSV4RRFFQ69G5FAV';
const B = '01BX5ZZKBKACTAV9WEVGEMMVRZ';

test('entries are read in order, repeats kept, other lines ignored', () => {
  const body = `For the car.\n\n1. [Song \\[live\\]](note:${A})\n2. [B](note:${B})\n3. [Song](note:${A})\n`;
  expect(entries(body)).toEqual([
    { id: A, title: 'Song [live]' },
    { id: B, title: 'B' },
    { id: A, title: 'Song' },
  ]);
});

test('a rewrite renumbers the list where it stood and keeps every other line', () => {
  const body = `For the car.\n\n1. [A](note:${A})\n2. [B](note:${B})\n\nMade on a rainy day.\n`;
  const out = rewrite(body, move(entries(body), 1, 0));
  expect(out).toBe(
    `For the car.\n\n1. [B](note:${B})\n2. [A](note:${A})\n\nMade on a rainy day.\n`,
  );
  expect(rewrite(out, [])).toBe('For the car.\n\n\nMade on a rainy day.\n');
});

test('an empty playlist gets its list at the end', () => {
  expect(rewrite('', [{ id: A, title: 'A [x]' }])).toBe(`1. [A \\[x\\]](note:${A})\n`);
  expect(rewrite('Notes.\n', [{ id: B, title: 'B' }])).toBe(`Notes.\n\n1. [B](note:${B})\n`);
});
