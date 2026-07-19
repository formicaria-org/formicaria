// Deriving the blob origin — and above all, never throwing while doing it.
import { describe, expect, it } from 'vitest';
import { blobBase } from './blobBase';

describe('blobBase', () => {
  /** **The bug this exists to prevent.** `assetUrl` is called *during render* of any note that
   *  has an asset, so an exception here does not degrade to a broken image — it takes the
   *  component down and the app is a blank screen with nothing in the console. That shipped: the
   *  emulator's vault was empty so the path never ran there, while a real vault with one asset
   *  note blanked on launch. A wrong URL costs an image; a throw costs the application. */
  it('never throws, whatever Tauri does', () => {
    expect(() =>
      blobBase(() => {
        throw new Error('convertFileSrc exploded');
      }),
    ).not.toThrow();
    expect(
      blobBase(() => {
        throw new Error('boom');
      }),
    ).toBe('fmblob://localhost/');
  });

  it('falls back when the helper is absent', () => {
    expect(blobBase(undefined)).toBe('fmblob://localhost/');
  });

  it('trims the probe back to its origin', () => {
    // What Android returns: wry rewrites the custom scheme onto an http origin.
    expect(blobBase((p) => `http://fmblob.localhost/${p}`)).toBe('http://fmblob.localhost/');
  });

  it('falls back when the probe comes back unrecognisable', () => {
    // A helper that ignores the path entirely gives nothing to trim, and guessing would produce
    // a URL that silently addresses the wrong thing.
    expect(blobBase(() => 'something-else')).toBe('fmblob://localhost/');
    expect(blobBase(() => '__base__')).toBe('fmblob://localhost/');
  });
});
