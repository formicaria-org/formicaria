// The contributor list: who has actually touched these notes.
import { describe, expect, it } from 'vitest';
import { contributors, setActivity } from './activity.svelte';

// ── One person is one contributor ──
describe('contributors', () => {
  const ev = (author: string, email: string, id = 'X') =>
    ({ id, title: null, type: 'note', vault: 'v', author, email, time: '2026-07-19T10:00:00Z' });

  /** The reported bug: a solo vault listing three people. `formicaria` is the sentinel
   *  `ensure_repo` writes before anyone says who they are, and `git::identity` already treats it
   *  as *absent* — so a contributor list that shows it as a person contradicts the rest of the
   *  app and invents a collaborator. */
  it('never lists the placeholder as a person', () => {
    setActivity([
      ev('formicaria', 'formicaria@localhost'),
      ev('Baljinder', 'b@example.org'),
    ] as never);
    expect(contributors()).toEqual(['Baljinder']);
  });

  /** The same human, signed differently on two machines, is still one human. Listing both is
   *  what makes a solo project look shared. */
  it('folds name spellings together by email', () => {
    setActivity([
      ev('Baljinder Singh', 'b@example.org'),
      ev('singhbal-baljinder', 'B@Example.org'), // same address, different case
    ] as never);
    expect(contributors()).toEqual(['Baljinder Singh']); // newest spelling wins
  });

  it('still separates genuinely different people', () => {
    setActivity([ev('Ada', 'ada@example.org'), ev('Bo', 'bo@example.org')] as never);
    expect(contributors()).toEqual(['Ada', 'Bo']);
  });

  it('falls back to the name when a commit carries no email', () => {
    setActivity([ev('Nameless', '')] as never);
    expect(contributors()).toEqual(['Nameless']);
  });
});
