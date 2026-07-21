// The proposals mock, driven as the Collaboration surface drives it. A proposal is a note
// carrying a well-formed `proposes: branch:<name>`; the mock's `isProposal` must agree with the
// server's `is_proposal` (and `parse_branch_ref`), or dev and CI would see a different world from
// production — the very drift the mock exists to prevent.

import { describe, expect, it } from 'vitest';
import * as mock from './mock';
import type { Board, ObjectMeta } from './types';

const capture = () => mock.handle<ObjectMeta>('capture', { body: 'a note' });
const setProp = (id: string, key: string, value: string) =>
  mock.handle('set_property', { id, key, value });
const proposals = () => mock.handle<ObjectMeta[]>('proposals', {});
const recent = () => mock.handle<ObjectMeta[]>('recent', {});
const cardCount = async () =>
  (await mock.handle<Board>('board', { groupBy: 'status' })).columns.reduce(
    (n, c) => n + c.cards.length,
    0,
  );

// The mock store is module-level with no reset, so every assertion is relative to a count taken
// inside the test (same discipline as the discussion tests).
describe('proposals in the mock backend', () => {
  it('lists a proposal and keeps it out of the planning views', async () => {
    const beforeRecent = (await recent()).length;
    const beforeCards = await cardCount();
    const beforeProps = (await proposals()).length;

    const p = await capture();
    await setProp(p.id, 'proposes', 'branch:fix-protocol');

    // Listed on the Collaboration surface...
    const props = await proposals();
    expect(props.length).toBe(beforeProps + 1);
    expect(props.some((m) => m.id === p.id)).toBe(true);

    // ...and therefore not a note you plan: the note list and the board are unchanged.
    expect((await recent()).length).toBe(beforeRecent);
    expect(await cardCount()).toBe(beforeCards);
  });

  // The parse-guard, mirrored: a word a board drop or typo would write must not turn a note into
  // a proposal, nor hide it — exactly what the Rust `parse_branch_ref` refuses.
  it('treats a stray proposes value as an ordinary note', async () => {
    const beforeRecent = (await recent()).length;
    const beforeProps = (await proposals()).length;

    const n = await capture();
    await setProp(n.id, 'proposes', 'doing');

    expect((await proposals()).length).toBe(beforeProps); // not a proposal
    expect((await recent()).length).toBe(beforeRecent + 1); // still an ordinary note
  });
});
