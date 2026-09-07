// "This vault has not saved anything in N days" — the one signal that does not depend on any
// other signal working.
//
// The defect this pins is an absence: during the thirty-nine days a vault sat frozen (2026-07/08)
// nothing in the app said how long it had been. Every specific detector was either missing or
// blocked by the same conflict it would have reported, so the screen looked exactly like a
// notebook nobody had opened. The tests below are mostly about the two states that would make
// such a chip *worse* than nothing — a never-committed vault, and a clock that runs backwards —
// because a nag that fires on a first run is a nag people learn to ignore before it is ever right.
import { describe, expect, it } from 'vitest';
import { QUIET_AFTER_DAYS, quietLabel, quietTitle, quietVaults } from './quietVaults';

const NOW = Date.UTC(2026, 8, 7, 12, 0, 0);
const daysAgo = (n: number) => Math.floor((NOW - n * 86_400_000) / 1000);

describe('vaults that have gone quiet', () => {
  it('says nothing about a vault that saved recently', () => {
    expect(quietVaults([{ vault: 'personal', last_commit: daysAgo(2) }], NOW)).toEqual([]);
  });

  it('stays silent right up to the threshold and speaks on it', () => {
    const at = (n: number) => quietVaults([{ vault: 'v', last_commit: daysAgo(n) }], NOW);
    expect(at(QUIET_AFTER_DAYS - 1)).toEqual([]);
    expect(at(QUIET_AFTER_DAYS)).toEqual([{ vault: 'v', days: QUIET_AFTER_DAYS }]);
  });

  it('ignores a vault that has never been committed', () => {
    // The state that would break it: `null` read as `0` is 1970, which renders as twenty thousand
    // days of silence — an alarm shown to the one person who has done nothing wrong yet.
    expect(quietVaults([{ vault: 'fresh', last_commit: null }], NOW)).toEqual([]);
  });

  it('ignores a commit dated in the future', () => {
    // Two devices, two clocks, and one of them ahead: ordinary, and not a reason to say anything.
    expect(quietVaults([{ vault: 'skewed', last_commit: daysAgo(-3) }], NOW)).toEqual([]);
  });

  it('reports the longest silence first', () => {
    const q = quietVaults(
      [
        { vault: 'lab', last_commit: daysAgo(39) },
        { vault: 'personal', last_commit: daysAgo(1) },
        { vault: 'archive', last_commit: daysAgo(200) },
      ],
      NOW,
    );
    expect(q).toEqual([
      { vault: 'archive', days: 200 },
      { vault: 'lab', days: 39 },
    ]);
  });

  it('names the vault when there is one and counts them when there are more', () => {
    expect(quietLabel([{ vault: 'lab', days: 39 }])).toBe('lab: 39 days since a save');
    expect(
      quietLabel([
        { vault: 'lab', days: 39 },
        { vault: 'archive', days: 200 },
      ]),
    ).toBe('2 vaults: no save in weeks');
    expect(quietLabel([])).toBe('');
  });

  it('keeps the per-vault detail in the hover text the short label dropped', () => {
    expect(
      quietTitle([
        { vault: 'lab', days: 39 },
        { vault: 'archive', days: 200 },
      ]),
    ).toBe('lab: 39 days since a save · archive: 200 days since a save');
  });
});
