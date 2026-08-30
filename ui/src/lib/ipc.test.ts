// **The asset URL, and the thumbnail it could never ask for.**
//
// Thumbnails were generated on every image ingest and readable through a command, but the URL form
// had no way to name one — so every inline image in the app decoded the original. `render.ts` puts
// that at "~50 MB of decoded pixels" for one 12 MP photo. The feed would multiply it by the
// screen, which is what finally forced the parameter.
//
// The default matters as much as the addition: every existing call site passes one argument, and
// if that stopped producing the same URL, inline media would break everywhere at once.

import { describe, expect, it } from 'vitest';
import { assetUrl } from './ipc';

describe('assetUrl', () => {
  it('is unchanged for every caller that does not ask for a thumbnail', () => {
    expect(assetUrl('sha256:abc')).toBe('/api/blob/sha256%3Aabc');
  });

  it('asks for the thumbnail with a query, not a different path', () => {
    expect(assetUrl('sha256:abc', 'thumb')).toBe('/api/blob/sha256%3Aabc?kind=thumb');
  });

  it('keeps a fragment after the query, where a URL parser expects it', () => {
    // `#page=3` is how the read view opens a PDF at the right page — a fix this file already made
    // once, when the whole reference was percent-encoded into the path and the fragment was buried
    // where no browser could act on it. Putting the query *after* the fragment would repeat it.
    expect(assetUrl('sha256:abc#page=3', 'thumb')).toBe('/api/blob/sha256%3Aabc?kind=thumb#page=3');
    expect(assetUrl('sha256:abc#page=3')).toBe('/api/blob/sha256%3Aabc#page=3');
  });
});
