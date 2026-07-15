import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderInto, type ResolvedAsset } from './render';
import { SAMPLE_BODY } from './mock';

// The two heavy upgrades (KaTeX math, Mermaid diagrams) are lazily imported by
// render.ts and need a real browser to do anything useful. We replace them with
// spies so the suite is hermetic and fast, and so we can drive the two branches
// that matter — the upgrade succeeds, or it fails and the note degrades to its
// raw text. `vi.hoisted` builds the spies before the hoisted `vi.mock` factories
// run, and lets the tests reach them without importing the untyped module paths.
const { mermaidInit, mermaidRender, katexAutoRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  katexAutoRender: vi.fn(),
}));

vi.mock('mermaid', () => ({
  default: { initialize: mermaidInit, render: mermaidRender },
}));
vi.mock('katex/dist/contrib/auto-render.js', () => ({ default: katexAutoRender }));
// render.ts pulls the stylesheet in for its side effect; a no-op module is plenty.
vi.mock('katex/dist/katex.min.css', () => ({}));

/** A fresh detached element to render into — the read pane, in miniature. */
function pane(): HTMLElement {
  return document.createElement('div');
}

/** The default read-view asset resolver: nothing resolves (missing / mock). */
const noAsset = async (): Promise<ResolvedAsset | null> => null;
/** A resolver returning a typed url+mime, like NotePanel's real one. */
const resolveAs = (mime: string, url = 'blob:test/asset') =>
  vi.fn(async (): Promise<ResolvedAsset> => ({ url, mime }));

beforeEach(() => {
  // Default: Mermaid renders a diagram. The failure test overrides this.
  mermaidRender.mockResolvedValue({ svg: '<svg data-mock-mermaid="1"></svg>' });
});

describe('renderInto — markdown becomes the expected HTML', () => {
  it('renders a paragraph of prose', async () => {
    const el = pane();
    await renderInto(el, 'just some prose.', noAsset);
    expect(el.querySelector('p')?.textContent).toBe('just some prose.');
  });

  it('renders headings, emphasis, and links', async () => {
    const el = pane();
    await renderInto(el, '# Title\n\n**bold** and _italic_ and [a link](https://example.com).', noAsset);
    expect(el.querySelector('h1')?.textContent).toBe('Title');
    expect(el.querySelector('strong')?.textContent).toBe('bold');
    expect(el.querySelector('em')?.textContent).toBe('italic');
    expect(el.querySelector('a')?.getAttribute('href')).toBe('https://example.com');
  });

  it('renders ordered and unordered lists', async () => {
    const el = pane();
    await renderInto(el, '- one\n- two\n\n1. first\n2. second\n', noAsset);
    expect(el.querySelectorAll('ul > li')).toHaveLength(2);
    expect(el.querySelectorAll('ol > li')).toHaveLength(2);
  });

  it('renders a GFM table (proves gfm:true is on)', async () => {
    const el = pane();
    await renderInto(el, '| A | B |\n| - | - |\n| 1 | 2 |\n', noAsset);
    expect(el.querySelector('table')).not.toBeNull();
    expect(el.querySelector('th')?.textContent).toBe('A');
  });

  it('renders a fenced code block without swallowing its contents', async () => {
    const el = pane();
    await renderInto(el, '```js\nconst x = 1 < 2;\n```\n', noAsset);
    const code = el.querySelector('pre > code');
    expect(code?.className).toContain('language-js');
    // `<` must survive as text, not be parsed as a tag.
    expect(code?.textContent).toContain('const x = 1 < 2;');
  });

  it('renders inline code, a blockquote, and a horizontal rule', async () => {
    const el = pane();
    await renderInto(el, 'use `fm set`.\n\n> a quote\n\nabove\n\n---\n\nbelow\n', noAsset);
    expect(el.querySelector('code')?.textContent).toBe('fm set');
    expect(el.querySelector('blockquote')?.textContent).toContain('a quote');
    expect(el.querySelector('hr')).not.toBeNull();
  });

  it('never blanks the pane on an empty or whitespace-only body', async () => {
    for (const body of ['', '   \n  \n']) {
      const el = pane();
      await expect(renderInto(el, body, noAsset)).resolves.toBeUndefined();
      expect(el.innerHTML.trim()).toBe('');
    }
  });
});

describe('renderInto — assets degrade gracefully', () => {
  it('rewrites an asset image src when the resolver finds it', async () => {
    const el = pane();
    const resolve = resolveAs('image/png', 'blob:test/fig.png');
    await renderInto(el, '![a figure](asset:sha256-deadbeef)', resolve);
    expect(resolve).toHaveBeenCalledWith('asset:sha256-deadbeef');
    expect(el.querySelector('img')?.getAttribute('src')).toBe('blob:test/fig.png');
    expect(el.querySelector('.asset-missing-inline')).toBeNull();
  });

  it('renders a PDF as a scrollable inline iframe', async () => {
    const el = pane();
    await renderInto(el, '![paper](asset:sha256-deadbeef)', resolveAs('application/pdf', 'blob:test/p.pdf'));
    const frame = el.querySelector('iframe.asset-pdf');
    expect(frame).not.toBeNull();
    expect(frame?.getAttribute('src')).toBe('blob:test/p.pdf');
    expect(el.querySelector('figcaption')?.textContent).toBe('paper'); // alt → caption
    expect(el.querySelector('img')).toBeNull();
  });

  it('renders video and audio with native controls', async () => {
    const v = pane();
    await renderInto(v, '![clip](asset:sha256-aa)', resolveAs('video/mp4', 'blob:test/v.mp4'));
    expect(v.querySelector('video.asset-video')?.getAttribute('src')).toBe('blob:test/v.mp4');
    const a = pane();
    await renderInto(a, '![sound](asset:sha256-bb)', resolveAs('audio/mpeg', 'blob:test/a.mp3'));
    expect(a.querySelector('audio.asset-audio')?.getAttribute('src')).toBe('blob:test/a.mp3');
  });

  it('falls back to an open-link for an unknown non-image type', async () => {
    const el = pane();
    await renderInto(el, '![data](asset:sha256-cc)', resolveAs('application/octet-stream', 'blob:test/d.bin'));
    const link = el.querySelector('a.asset-file');
    expect(link?.getAttribute('href')).toBe('blob:test/d.bin');
    expect(link?.textContent).toBe('Open data');
  });

  it('swaps in an inline placeholder (with the alt text) when the asset is missing', async () => {
    const el = pane();
    await renderInto(el, '![trust-region figure](asset:sha256-deadbeef)', noAsset);
    const placeholder = el.querySelector('.asset-missing-inline');
    expect(placeholder).not.toBeNull();
    expect(placeholder?.textContent).toBe('trust-region figure');
    expect(el.querySelector('img')).toBeNull(); // the broken <img> is gone
  });

  it('resolves sha256: image refs too', async () => {
    const el = pane();
    const resolve = resolveAs('image/png', 'blob:test/x.png');
    await renderInto(el, '![x](sha256:abc123)', resolve);
    expect(resolve).toHaveBeenCalledWith('sha256:abc123');
    expect(el.querySelector('img')?.getAttribute('src')).toBe('blob:test/x.png');
  });

  it('leaves a normal web image untouched and never calls the resolver', async () => {
    const el = pane();
    const resolve = resolveAs('image/png');
    await renderInto(el, '![web](https://example.com/y.png)', resolve);
    expect(resolve).not.toHaveBeenCalled();
    expect(el.querySelector('img')?.getAttribute('src')).toBe('https://example.com/y.png');
  });
});

describe('renderInto — math and diagrams upgrade or fall back', () => {
  it('invokes KaTeX when the body contains math', async () => {
    const el = pane();
    await renderInto(el, 'Euler: $e^{i\\pi} + 1 = 0$ and $$a^2 + b^2 = c^2$$', noAsset);
    expect(katexAutoRender).toHaveBeenCalledOnce();
    expect(katexAutoRender.mock.calls[0][0]).toBe(el); // rendered against our pane
  });

  it('skips the KaTeX import entirely when there is no `$` (the cheap gate)', async () => {
    const el = pane();
    await renderInto(el, 'no math here, just prose.', noAsset);
    expect(katexAutoRender).not.toHaveBeenCalled();
  });

  it('replaces a mermaid fence with an inline diagram when it parses', async () => {
    const el = pane();
    await renderInto(el, '```mermaid\ngraph TD; A-->B;\n```\n', noAsset);
    expect(mermaidRender).toHaveBeenCalledOnce();
    expect(el.querySelector('.mermaid-diagram')).not.toBeNull();
    expect(el.querySelector('code.language-mermaid')).toBeNull();
  });

  it('leaves the code fence as-is when a mermaid diagram fails to parse', async () => {
    mermaidRender.mockRejectedValueOnce(new Error('unparseable diagram'));
    const el = pane();
    await renderInto(el, '```mermaid\nnot a real diagram\n```\n', noAsset);
    // Graceful degradation: the raw fence survives, the pane is not blanked.
    expect(el.querySelector('.mermaid-diagram')).toBeNull();
    expect(el.querySelector('code.language-mermaid')).not.toBeNull();
  });
});

describe('renderInto — characterization of the real sample note', () => {
  it('renders the shipped SAMPLE_BODY end to end', async () => {
    const el = pane();
    // Resolver returns null, exactly like NotePanel.svelte today.
    await expect(renderInto(el, SAMPLE_BODY, noAsset)).resolves.toBeUndefined();
    expect(el.querySelector('h1')?.textContent).toBe('GAE and inner-loop adaptation');
    expect(el.querySelectorAll('ul > li').length).toBeGreaterThan(0);
    expect(katexAutoRender).toHaveBeenCalled(); // the $…$ / $$…$$ math
    expect(mermaidRender).toHaveBeenCalled(); // the ```mermaid block
    expect(el.querySelector('.asset-missing-inline')).not.toBeNull(); // the asset: image
  });

  // NOT a security guarantee — a red flag. render.ts assigns marked's output to
  // innerHTML with no sanitizer, so raw HTML in a note body reaches the DOM. This
  // test pins that CURRENT behavior; see the plan's "Out of scope" note on adding
  // a sanitizer as a follow-up.
  it('passes raw HTML through unsanitized (documents the gap)', async () => {
    const el = pane();
    await renderInto(el, 'text <b>injected</b> and <img src="z" onerror="danger()">', noAsset);
    expect(el.querySelector('b')?.textContent).toBe('injected');
    expect(el.querySelector('img[onerror]')).not.toBeNull();
  });
});
