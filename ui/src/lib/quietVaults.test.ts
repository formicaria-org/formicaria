// The chip that reports an **absence** — the one signal that does not depend on any other signal
// working.
//
// It began as "this vault has not saved anything in N days", pinning the thirty-nine days a vault
// sat frozen (2026-07/08) while every specific detector was either missing or blocked by the same
// conflict it would have reported.
//
// **It now reports a second absence, because the first one could not see the failure that
// actually happened.** On 2026-09-08 the owner had 125 changes that had never left the device,
// over six days, and said: *"I had no icon saying that I had any uncommitted/unbacked notes."*
// They were right, and the reason is that saving and sending are different facts — the auto-commit
// loop keeps *saving* perfect, so a vault that never sends still reads as maximally healthy here.
// The two thresholds differ for a reason the tests below state.
//
// The tests are still mostly about the states that would make such a chip *worse* than nothing —
// a never-committed vault, a vault that has never sent, and a clock that runs backwards — because
// a nag that fires on a first run is a nag people learn to ignore before it is ever right.
import { describe, expect, it } from 'vitest';
import {
  QUIET_AFTER_DAYS,
  UNSENT_AFTER_DAYS,
  quietLabel,
  quietTitle,
  quietVaults,
} from './quietVaults';
import type { LastCommit } from './types';

const NOW = Date.UTC(2026, 8, 7, 12, 0, 0);
const daysAgo = (n: number) => Math.floor((NOW - n * 86_400_000) / 1000);

/// A wire row. Defaults are the healthy vault — saved just now, sent just now, nothing waiting —
/// so each test states only the fact it is about.
const row = (over: Partial<LastCommit> & { vault: string }): LastCommit => ({
  last_commit: daysAgo(0),
  last_sent: daysAgo(0),
  unsent: 0,
  ...over,
});

describe('vaults that have gone quiet', () => {
  it('says nothing about a vault that saved recently', () => {
    expect(quietVaults([row({ vault: 'personal', last_commit: daysAgo(2) })], NOW)).toEqual([]);
  });

  it('stays silent right up to the threshold and speaks on it', () => {
    const at = (n: number) => quietVaults([row({ vault: 'v', last_commit: daysAgo(n) })], NOW);
    expect(at(QUIET_AFTER_DAYS - 1)).toEqual([]);
    expect(at(QUIET_AFTER_DAYS)).toEqual([{ vault: 'v', days: QUIET_AFTER_DAYS, kind: 'saved' }]);
  });

  it('ignores a vault that has never been committed', () => {
    // The state that would break it: `null` read as `0` is 1970, which renders as twenty thousand
    // days of silence — an alarm shown to the one person who has done nothing wrong yet.
    expect(quietVaults([row({ vault: 'fresh', last_commit: null })], NOW)).toEqual([]);
  });

  it('ignores a commit dated in the future', () => {
    // Two devices, two clocks, and one of them ahead: ordinary, and not a reason to say anything.
    expect(quietVaults([row({ vault: 'skewed', last_commit: daysAgo(-3) })], NOW)).toEqual([]);
  });

  it('reports the longest silence first', () => {
    const q = quietVaults(
      [
        row({ vault: 'lab', last_commit: daysAgo(39) }),
        row({ vault: 'personal', last_commit: daysAgo(1) }),
        row({ vault: 'archive', last_commit: daysAgo(200) }),
      ],
      NOW,
    );
    expect(q).toEqual([
      { vault: 'archive', days: 200, kind: 'saved' },
      { vault: 'lab', days: 39, kind: 'saved' },
    ]);
  });
});

describe('vaults whose notes have not left the device', () => {
  // **Why a tighter threshold than silence.** Fourteen days of silence is *suspicious* — it might
  // be a holiday. Work that has never been sent is a *single point of failure that grows*: those
  // notes exist in exactly one place, and the cost of being wrong is the whole week's writing, not
  // a late warning. Three days is short enough to catch the six-day gap that prompted this and
  // long enough that someone who backs up twice a week never sees it.
  it('stays silent right up to its own threshold and speaks on it', () => {
    const at = (n: number) =>
      quietVaults([row({ vault: 'v', last_sent: daysAgo(n), unsent: 12 })], NOW);
    expect(at(UNSENT_AFTER_DAYS - 1)).toEqual([]);
    expect(at(UNSENT_AFTER_DAYS)).toEqual([
      { vault: 'v', days: UNSENT_AFTER_DAYS, kind: 'sent', unsent: 12 },
    ]);
  });

  it('says nothing when everything has been sent, however long ago', () => {
    // The distinction the whole chip turns on: an old *send* is fine if nothing has happened
    // since. Firing on this would put an alarm on every vault a person has finished with.
    expect(quietVaults([row({ vault: 'done', last_sent: daysAgo(90), unsent: 0 })], NOW)).toEqual(
      [],
    );
  });

  it('ignores a vault that has never been sent at all', () => {
    // `null` is "no such moment", not "sent in 1970". A vault with nowhere to send is the Backup
    // panel's subject ("Nowhere to send yet"), and a vault made this morning has not *stopped*
    // sending — the same argument that keeps a never-committed vault out of the chip above.
    expect(quietVaults([row({ vault: 'fresh', last_sent: null, unsent: null })], NOW)).toEqual([]);
  });

  it('ignores a send dated in the future', () => {
    expect(quietVaults([row({ vault: 'skewed', last_sent: daysAgo(-3), unsent: 4 })], NOW)).toEqual(
      [],
    );
  });

  it('reports the deeper fault when a vault is both silent and unsent', () => {
    // A vault that cannot save is broken; a vault that has not been sent is merely behind. Naming
    // the second while the first is true would point at the wrong problem — and backing up will
    // not fix a vault that is frozen mid-merge, which is exactly how the thirty-nine days passed.
    const q = quietVaults(
      [row({ vault: 'stuck', last_commit: daysAgo(40), last_sent: daysAgo(40), unsent: 9 })],
      NOW,
    );
    expect(q).toEqual([{ vault: 'stuck', days: 40, kind: 'saved' }]);
  });

  it('this is the owner’s 2026-09-08 vault, and it must not be silent', () => {
    // Saving every few minutes, sending nothing for six days: healthy on every surface the app had.
    const q = quietVaults(
      [row({ vault: 'vault', last_commit: daysAgo(0), last_sent: daysAgo(6), unsent: 125 })],
      NOW,
    );
    expect(q).toEqual([{ vault: 'vault', days: 6, kind: 'sent', unsent: 125 }]);
  });
});

describe('what the chip says', () => {
  it('names the vault when there is one and counts them when there are more', () => {
    expect(quietLabel([{ vault: 'lab', days: 39, kind: 'saved' }])).toBe(
      'lab: 39 days since a save',
    );
    expect(
      quietLabel([
        { vault: 'lab', days: 39, kind: 'saved' },
        { vault: 'archive', days: 200, kind: 'saved' },
      ]),
    ).toBe('2 vaults: no save in weeks');
    expect(quietLabel([])).toBe('');
  });

  it('says the other absence in its own words', () => {
    expect(quietLabel([{ vault: 'vault', days: 6, kind: 'sent', unsent: 125 }])).toBe(
      'vault: 6 days without a backup',
    );
    expect(
      quietLabel([
        { vault: 'a', days: 6, kind: 'sent', unsent: 3 },
        { vault: 'b', days: 9, kind: 'sent', unsent: 4 },
      ]),
    ).toBe('2 vaults without a backup');
  });

  it('does not claim either absence when the reasons are mixed', () => {
    // Two vaults, two different problems, one chip. A sentence true of only half of them is worse
    // than a vaguer one that is true — the panel behind the chip is where they are told apart.
    expect(
      quietLabel([
        { vault: 'stuck', days: 40, kind: 'saved' },
        { vault: 'behind', days: 6, kind: 'sent', unsent: 12 },
      ]),
    ).toBe('2 vaults need attention');
  });

  it('keeps the per-vault detail in the hover text the short label dropped', () => {
    expect(
      quietTitle([
        { vault: 'lab', days: 39, kind: 'saved' },
        { vault: 'archive', days: 200, kind: 'saved' },
      ]),
    ).toBe('lab: 39 days since a save · archive: 200 days since a save');
  });

  it('puts the count in the hover text, where there is room for it', () => {
    // The label has to fit a toolbar; "how much would I lose" is the fact that makes the chip
    // worth clicking, so it goes where the space is.
    expect(quietTitle([{ vault: 'vault', days: 6, kind: 'sent', unsent: 125 }])).toBe(
      'vault: 125 changes not sent, 6 days ago',
    );
    expect(quietTitle([{ vault: 'v', days: 3, kind: 'sent', unsent: 1 }])).toBe(
      'v: 1 change not sent, 3 days ago',
    );
  });
});
