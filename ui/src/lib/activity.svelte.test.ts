// The contributor list: who has actually touched these notes.
import { describe, expect, it } from 'vitest';
import { authorKey, contributors, setActivity } from './activity.svelte';

// ── One person is one contributor ──
describe('contributors', () => {
  const ev = (author: string, email: string, id = 'X') =>
    ({ id, title: null, type: 'note', vault: 'v', author, email, time: '2026-07-19T10:00:00Z' });
  /// The display half of the list. `contributors()` now returns `{key, label}`: the key is what the
  /// filter hides by, and it travels *with* the label precisely so the two cannot drift apart.
  const labels = () => contributors().map((c) => c.label);

  /** The reported bug: a solo vault listing three people. `formicaria` is the sentinel
   *  `ensure_repo` writes before anyone says who they are, and `git::identity` already treats it
   *  as *absent* — so a contributor list that shows it as a person contradicts the rest of the
   *  app and invents a collaborator. */
  it('never lists the placeholder as a person', () => {
    setActivity([
      ev('formicaria', 'formicaria@localhost'),
      ev('Baljinder', 'b@example.org'),
    ] as never);
    expect(labels()).toEqual(['Baljinder']);
  });

  /** The same human, signed differently on two machines, is still one human. Listing both is
   *  what makes a solo project look shared. */
  it('folds name spellings together by email', () => {
    setActivity([
      ev('Baljinder Singh', 'b@example.org'),
      ev('singhbal-baljinder', 'B@Example.org'), // same address, different case
    ] as never);
    expect(labels()).toEqual(['Baljinder Singh']); // newest spelling wins
  });

  it('still separates genuinely different people', () => {
    setActivity([ev('Ada', 'ada@example.org'), ev('Bo', 'bo@example.org')] as never);
    expect(labels()).toEqual(['Ada', 'Bo']);
  });

  it('falls back to the name when a commit carries no email', () => {
    setActivity([ev('Nameless', '')] as never);
    expect(labels()).toEqual(['Nameless']);
  });
});

// ── The chip and the filter must agree on what one person is ──
describe('authorKey', () => {
  /** The bug this exists to prevent, measured in the owner's own vault (2026-07-31): 534 commits as
   *  `singhbal-baljinder`, 77 as `Baljinder`, one email. The chips already folded those into one
   *  person; the *filter* compared the author name, so hiding that one chip hid only the spelling it
   *  was labelled with and left the other 77 notes on screen — while the chip read "hidden". */
  it('is the email, so every spelling of one person shares a key', () => {
    const a = authorKey({ author: 'singhbal-baljinder', email: 'singhbal.baljinder@gmail.com' });
    const b = authorKey({ author: 'Baljinder', email: 'SinghBal.Baljinder@Gmail.com' });
    expect(a).toBe(b);
  });

  it('keeps genuinely different people apart', () => {
    expect(authorKey({ author: 'Ada', email: 'ada@example.org' })).not.toBe(
      authorKey({ author: 'Bo', email: 'bo@example.org' }),
    );
  });

  it('falls back to the name when a commit has no email, rather than folding everyone into one', () => {
    expect(authorKey({ author: 'Nameless', email: '' })).toBe('nameless');
    expect(authorKey({ author: 'Other', email: '' })).toBe('other');
  });

  /** The key the chips hand to the filter must be exactly the key the filter computes from an
   *  event — same function, both sides. That identity is the fix. */
  it('matches the key the contributor list publishes', () => {
    setActivity([
      { id: 'X', title: null, type: 'note', vault: 'v', author: 'Baljinder', email: 'b@example.org', time: '2026-07-19T10:00:00Z' },
    ] as never);
    const [chip] = contributors();
    expect(chip.key).toBe(authorKey({ author: 'Baljinder', email: 'b@example.org' }));
  });
});
