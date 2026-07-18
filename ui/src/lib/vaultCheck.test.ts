import { describe, expect, it } from 'vitest';
import { describe as describeCheck, historyNote } from './vaultCheck';
import type { PathCheck } from './types';

/** A path that would create cleanly. Each test spoils exactly one thing, so what is being
 *  asserted is the difference and not the fixture. */
const okCheck = (over: Partial<PathCheck> = {}): PathCheck => ({
  path: '/home/you/notes',
  exists: false,
  empty: false,
  notes: 0,
  not_a_directory: false,
  parent_missing: false,
  writable: true,
  git_repo: false,
  name_ok: true,
  name_taken: false,
  path_taken: false,
  overlaps: null,
  config_writable: true,
  ok: true,
  ...over,
});

describe('describe', () => {
  it('has nothing to say about a clean create', () => {
    const d = describeCheck(okCheck({ exists: true, empty: true }));
    expect(d.blocking).toEqual([]);
    expect(d.warnings).toEqual([]);
  });

  it('says nothing at all before the first answer', () => {
    expect(describeCheck(null)).toEqual({ blocking: [], warnings: [] });
  });

  // The one this module exists for. Adoption is free — FileStore reindexes whatever is
  // already there — but free is not the same as expected, and creating a vault over
  // someone's notes without saying so is the surprise we refuse to ship.
  it('warns about notes it will adopt, and never blocks on them', () => {
    const d = describeCheck(okCheck({ notes: 12, exists: true }));
    expect(d.blocking).toEqual([]);
    expect(d.warnings.join(' ')).toContain('12 notes');
    expect(d.warnings.join(' ')).toContain('adopted');
  });

  it('counts one note without pluralising it', () => {
    expect(describeCheck(okCheck({ notes: 1, exists: true })).warnings.join(' ')).toContain(
      '1 note in it',
    );
  });

  it('blocks a taken name', () => {
    const d = describeCheck(okCheck({ name_taken: true, ok: false }));
    expect(d.blocking).toHaveLength(1);
    expect(d.blocking[0]).toContain('already have a vault with this name');
  });

  it('blocks a taken path, and an overlap by naming the other vault', () => {
    expect(describeCheck(okCheck({ path_taken: true, ok: false })).blocking[0]).toContain(
      'already a vault',
    );
    const overlap = describeCheck(okCheck({ overlaps: 'lab', ok: false }));
    expect(overlap.blocking[0]).toContain('lab');
    expect(overlap.blocking[0]).toContain('two audiences');
  });

  it('blocks an unwritable path', () => {
    expect(describeCheck(okCheck({ writable: false, ok: false })).blocking.join(' ')).toContain(
      'cannot write to /home/you/notes',
    );
  });

  // A file is not a folder AND is not writable; saying both would be noise.
  it('calls a file a file, without also complaining it is unwritable', () => {
    const d = describeCheck(okCheck({ not_a_directory: true, writable: false, ok: false }));
    expect(d.blocking).toEqual(['That is a file, not a folder.']);
  });

  // Unwritable config cannot be fixed by renaming the vault, so it is said first and it
  // promises we will not touch the file.
  it('blocks on an unwritable vault list, first, and promises not to touch it', () => {
    const d = describeCheck(okCheck({ config_writable: false, name_taken: true, ok: false }));
    expect(d.blocking[0]).toContain('vault list');
    expect(d.blocking[0]).toContain('nothing here will touch it');
  });

  it('promises to create the folders, and says so only once', () => {
    expect(describeCheck(okCheck({ parent_missing: true })).warnings).toEqual([
      "The folders don't exist yet — we'll create them.",
    ]);
    expect(describeCheck(okCheck({ exists: false })).warnings).toEqual([
      "The folder doesn't exist yet — we'll create it.",
    ]);
  });

  it('reports an existing repo as a warning, promising to leave its history alone', () => {
    const d = describeCheck(okCheck({ git_repo: true, exists: true, empty: true }));
    expect(d.blocking).toEqual([]);
    expect(d.warnings.join(' ')).toContain('leave its history alone');
  });
});

describe('historyNote', () => {
  // Absence must be stated, never swallowed: a notebook that quietly isn't versioned is
  // something you discover on the day you need the history.
  it('says notes are safe but unversioned when git is absent', () => {
    const s = historyNote(false);
    expect(s).toContain('files on disk');
    expect(s).toContain('safe');
    expect(s).toContain("won't have history");
  });

  it('says nothing needs setting up when git is there', () => {
    expect(historyNote(true)).toContain('versioned');
  });
});
