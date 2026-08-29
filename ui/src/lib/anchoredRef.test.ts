// **A note pointing at a place in a PDF**, `[p. 4](asset:sha256-…#page=4)`.
//
// Three things were wrong before 2026-08-29, and each is silent:
//  - `resolveAssets` walked `img` only, while `URI_ALLOWED` deliberately lets the `asset:` scheme
//    through the sanitiser — so the anchor survived as a live link to a scheme nothing resolves.
//    On Android that navigates the WebView out of the app (the trap `noteChip` documents for
//    `note:`).
//  - An anchored *image* reference reached `parse_ref`, failed its all-hex check, and rendered as
//    a "not an asset reference" placeholder.
//  - `assetUrl` percent-encoded the whole reference into the path, burying the `#` where no
//    browser could act on it — so an anchored link opened the document at page one.
import { describe, expect, it } from 'vitest';
import { renderInto } from './render';
import { assetUrl } from './ipc';

const HEX = '9f2c8a1b3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8';
const REF = `asset:sha256-${HEX}`;

const resolver = async (reference: string) => ({ url: assetUrl(reference), mime: 'application/pdf' });

describe('assetUrl', () => {
  it('keeps an anchor as a real URL fragment', () => {
    const url = assetUrl(`${REF}#page=4`);
    // `#page=N` is the standard PDF open parameter; a real viewer honours it, which is what makes
    // the reference degrade to a working link outside this app.
    expect(url.endsWith('#page=4')).toBe(true);
    expect(url).not.toContain('%23');
  });
  it('leaves an unanchored reference exactly as it was', () => {
    expect(assetUrl(REF)).toBe(`/api/blob/${encodeURIComponent(REF)}`);
  });
});

describe('an anchored link in a note body', () => {
  it('resolves to the blob, at the page it names', async () => {
    const el = document.createElement('div');
    await renderInto(el, `See [p. 4](${REF}#page=4) for the claim.`, resolver);

    const a = el.querySelector('a.asset-link') as HTMLAnchorElement | null;
    expect(a, 'the anchor is resolved, not left on a dead scheme').not.toBeNull();
    expect(a!.getAttribute('href')).toContain(HEX);
    expect(a!.getAttribute('href')!.endsWith('#page=4')).toBe(true);
    // Never a dead scheme left in the document.
    expect(el.innerHTML).not.toContain('href="asset:');
    // A new tab: the note is what is being read, and replacing it loses the reader's place.
    expect(a!.getAttribute('target')).toBe('_blank');
    expect(a!.getAttribute('rel')).toBe('noopener');
    expect(a!.textContent).toBe('p. 4');
  });

  it('says what is missing when the blob is not here', async () => {
    const el = document.createElement('div');
    await renderInto(el, `See [p. 4](${REF}#page=4).`, async () => ({ reason: 'in another vault' }));
    expect(el.querySelector('a.asset-link')).toBeNull();
    const ph = el.querySelector('.asset-missing-inline');
    expect(ph?.textContent).toContain('in another vault');
    // The label survives in the placeholder so the sentence still reads.
    expect(ph?.textContent).toContain('p. 4');
  });

  it('an anchored image reference still embeds', async () => {
    const el = document.createElement('div');
    await renderInto(el, `![Figure 1](${REF}#page=3)`, resolver);
    expect(el.querySelector('.asset-missing-inline'), 'must not be a broken placeholder').toBeNull();
  });
});
