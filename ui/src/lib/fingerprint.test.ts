// The phone's fingerprinting defence (`ui/page/fingerprint.ts`): the same on a site for a session,
// different between sites and sessions, and too small to see or hear.
import { describe, expect, it } from 'vitest';
import { perturbPixels, perturbSamples, prng, seedFor } from '../../page/fingerprint';

const pixels = () =>
  new Uint8ClampedArray(Array.from({ length: 4 * 1000 }, (_, i) => (i * 37) % 256));
const changed = (a: ArrayLike<number>, b: ArrayLike<number>) => {
  let n = 0;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) n++;
  return n;
};

describe('fingerprinting answered with made-up detail', () => {
  it('reads the same on one site, differently on another and in another session', () => {
    const s = 'session-secret';
    expect(seedFor(s, 'www.news.example')).toBe(seedFor(s, 'news.example'));
    expect(seedFor(s, 'news.example')).not.toBe(seedFor(s, 'shop.example'));
    expect(seedFor('another session', 'news.example')).not.toBe(seedFor(s, 'news.example'));
    const a = perturbPixels(pixels(), seedFor(s, 'news.example'));
    const again = perturbPixels(pixels(), seedFor(s, 'news.example'));
    const other = perturbPixels(pixels(), seedFor(s, 'shop.example'));
    expect(changed(a, again)).toBe(0);
    expect(changed(a, other)).toBeGreaterThan(0);
  });

  it('changes a picture by one step of colour in a few places, never its transparency', () => {
    const before = pixels();
    const after = perturbPixels(pixels(), 42);
    const n = changed(before, after);
    expect(n).toBeGreaterThan(0);
    expect(n).toBeLessThanOrEqual(before.length / 4 / 50);
    for (let i = 0; i < before.length; i++) {
      expect(Math.abs(before[i] - after[i])).toBeLessThanOrEqual(1);
      if (i % 4 === 3) expect(after[i]).toBe(before[i]);
    }
  });

  it('moves sound by less than anyone hears', () => {
    const before = new Float32Array(2048).map((_, i) => Math.sin(i / 10));
    const after = perturbSamples(Float32Array.from(before), 7);
    expect(changed(before, after)).toBeGreaterThan(0);
    for (let i = 0; i < before.length; i++)
      expect(Math.abs(before[i] - after[i])).toBeLessThan(1e-6);
  });

  it('the generator is the same for the same seed', () => {
    const a = prng(5);
    const b = prng(5);
    expect([a(), a(), a()]).toEqual([b(), b(), b()]);
  });
});
