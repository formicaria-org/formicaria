import { describe, expect, it } from 'vitest';
import { hashHue } from './vaultColor';

describe('hashHue', () => {
  it('is deterministic — the same vault always gets the same hue', () => {
    expect(hashHue('lab')).toBe(hashHue('lab'));
    expect(hashHue('shared-vault-test')).toBe(hashHue('shared-vault-test'));
  });

  it('is always an integer hue in [0, 360)', () => {
    for (const name of ['', 'a', 'personal', 'lab', 'shared-vault-test', 'Lab — Ravi’s group']) {
      const h = hashHue(name);
      expect(Number.isInteger(h)).toBe(true);
      expect(h).toBeGreaterThanOrEqual(0);
      expect(h).toBeLessThan(360);
    }
  });

  it('separates distinct names (so two vaults read as different colours)', () => {
    expect(hashHue('personal')).not.toBe(hashHue('lab'));
  });
});
