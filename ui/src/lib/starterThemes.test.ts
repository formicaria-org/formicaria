// The starters promise nothing the manual does not: every name they set is one the app keeps.
import { describe, expect, it } from 'vitest';
import { SUPPORTED } from './appearance';
import { STARTERS, freeName, slug } from './starterThemes';

describe('the starter themes', () => {
  it('set only the names the app promises to keep', () => {
    for (const s of STARTERS) {
      const names = [...s.css.matchAll(/--([a-z0-9-]+)\s*:/g)].map((m) => m[1]);
      expect(names.length, s.name).toBeGreaterThan(0);
      expect(
        names.filter((n) => !SUPPORTED.includes(n)),
        s.name,
      ).toEqual([]);
    }
  });
  it('are named as files the way the server names them', () => {
    expect(slug('Large and calm')).toBe('large-and-calm');
    expect(slug('  Night  ink ')).toBe('night-ink');
    expect(slug('a/b')).toBe('a-b');
  });
  it('take a number rather than a name already in use', () => {
    expect(freeName('Paper', [])).toBe('Paper');
    expect(freeName('Paper', ['paper'])).toBe('Paper 2');
    expect(freeName('Paper', ['paper', 'paper-2'])).toBe('Paper 3');
  });
});
