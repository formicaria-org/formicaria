import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderInto, extractMath, type ResolvedAsset, type ResolvedNote } from './render';
import { SAMPLE_BODY } from './mock';

// The two heavy upgrades (KaTeX math, Mermaid diagrams) are lazily imported by
// render.ts and need a real browser to do anything useful. We replace them with
// spies so the suite is hermetic and fast, and so we can drive the two branches
// that matter — the upgrade succeeds, or it fails and the note degrades to its
// raw text. `vi.hoisted` builds the spies before the hoisted `vi.mock` factories
// run, and lets the tests reach them without importing the untyped module paths.
const { mermaidInit, mermaidRender, katexRender } = vi.hoisted(() => ({
  mermaidInit: vi.fn(),
  mermaidRender: vi.fn(),
  // render.ts fills each math placeholder with katex.renderToString; the spy
  // returns a marker so tests can see the formula reached KaTeX.
  katexRender: vi.fn((tex: string) => `<span class="katex">${tex}</span>`),
}));

vi.mock('mermaid', () => ({
  default: { initialize: mermaidInit, render: mermaidRender },
}));
vi.mock('katex', () => ({ default: { renderToString: katexRender } }));
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
    // The label the note gave it comes first, so the line still reads as the missing thing.
    expect(placeholder?.textContent).toBe('trust-region figure — not available');
    expect(el.querySelector('img')).toBeNull(); // the broken <img> is gone
  });

  // The reason is on screen because on a real device there is nowhere else to put it: an
  // Android WebView routes no `console` output to logcat, which cost a whole build/sign/install
  // round trip on 2026-07-20 before it was noticed. These two cases pin that it stays visible.
  it("shows the resolver's reason when it declines to resolve", async () => {
    const el = pane();
    const why = vi.fn(async () => ({ reason: 'no bytes in vault "notes"' }));
    await renderInto(el, '![IMG_0042.jpg](asset:sha256-deadbeef)', why);
    expect(el.querySelector('.asset-missing-inline')?.textContent).toBe(
      'IMG_0042.jpg — no bytes in vault "notes"',
    );
  });

  it('shows the message when the resolver throws, rather than swallowing it', async () => {
    const el = pane();
    const boom = vi.fn(async () => {
      throw new Error('asset_status: not an asset reference');
    });
    await renderInto(el, '![IMG_0042.jpg](asset:sha256-deadbeef)', boom);
    expect(el.querySelector('.asset-missing-inline')?.textContent).toBe(
      'IMG_0042.jpg — asset_status: not an asset reference',
    );
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
  it('renders each formula through KaTeX into its placeholder', async () => {
    const el = pane();
    await renderInto(el, 'Euler: $e^{i\\pi} + 1 = 0$ and $$a^2 + b^2 = c^2$$', noAsset);
    // One inline + one display formula → two KaTeX calls, display flag preserved.
    expect(katexRender).toHaveBeenCalledTimes(2);
    expect(katexRender).toHaveBeenCalledWith('e^{i\\pi} + 1 = 0', expect.objectContaining({ displayMode: false }));
    expect(katexRender).toHaveBeenCalledWith('a^2 + b^2 = c^2', expect.objectContaining({ displayMode: true }));
    // Placeholders are gone (filled), and no raw `$` leaks into the pane.
    expect(el.querySelector('span[data-math]')?.innerHTML).toContain('katex');
  });

  it('skips the KaTeX import entirely when there is no math (the cheap gate)', async () => {
    const el = pane();
    await renderInto(el, 'no math here, just prose.', noAsset);
    expect(katexRender).not.toHaveBeenCalled();
  });

  it('keeps a broken formula visible with the error as a tooltip, not a blank pane', async () => {
    katexRender.mockImplementationOnce(() => {
      throw new Error("Undefined control sequence: \\nope");
    });
    const el = pane();
    await renderInto(el, 'Bad: $$ \\nope{x} $$', noAsset);
    const host = el.querySelector('span.math-error') as HTMLElement | null;
    expect(host).not.toBeNull();
    expect(host?.textContent).toBe('$$ \\nope{x} $$'); // raw source preserved
    expect(host?.title).toContain('Undefined control sequence');
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
    expect(katexRender).toHaveBeenCalled(); // the $…$ / $$…$$ math
    expect(mermaidRender).toHaveBeenCalled(); // the ```mermaid block
    expect(el.querySelector('.asset-missing-inline')).not.toBeNull(); // the asset: image
  });

  // The security boundary. A note body is untrusted (it arrives from collaborators via the
  // merge driver), and render.ts assigns marked's output to innerHTML, so DOMPurify runs on
  // it first. A hostile `onerror` must never reach the DOM; benign inline formatting must
  // survive so real notes still render.
  it('strips event handlers and scripts while keeping benign HTML', async () => {
    const el = pane();
    await renderInto(
      el,
      'text <b>injected</b> and <img src="z" onerror="danger()"> and <script>evil()</script>',
      noAsset,
    );
    expect(el.querySelector('b')?.textContent).toBe('injected'); // benign formatting survives
    expect(el.querySelector('img[onerror]')).toBeNull(); // the handler is gone
    expect(el.querySelector('script')).toBeNull(); // and so is the script tag
    expect(el.innerHTML).not.toContain('onerror');
  });

  // The trap the sanitizer must NOT fall into: our own pipeline speaks three schemes that
  // are not in DOMPurify's default allow-list. If sanitizing strips them, every asset image
  // and note chip silently vanishes — a regression that looks like the feature never worked.
  it('preserves note: and asset: schemes so the resolve passes still fire', async () => {
    const el = pane();
    await renderInto(
      el,
      '![pic](asset:sha256-abc) and [a note](note:01KXNOTE)',
      // asset resolver returns null → the asset-missing placeholder proves the scheme survived
      noAsset,
    );
    // The note link kept its scheme (no resolveNote passed, so it stays an <a href="note:…">).
    expect(el.querySelector('a[href^="note:"]')).not.toBeNull();
    // The asset image was recognised by scheme and replaced by the missing-asset placeholder;
    // if the scheme had been stripped, resolveAssets would have skipped it and left a raw img.
    expect(el.querySelector('.asset-missing-inline')).not.toBeNull();
    expect(el.querySelector('img[src^="asset:"]')).toBeNull();
  });

  // The other half of the trap: the math/diagram pipeline runs in passes AFTER sanitize, on
  // placeholders sanitize must keep. If DOMPurify ate the `<span data-math>` or the mermaid
  // code block, math and diagrams would silently stop rendering. Assert both survive and both
  // downstream renderers still fire.
  it('keeps the math and mermaid placeholders through sanitize', async () => {
    const el = pane();
    await renderInto(el, 'inline $a^2$ and\n\n```mermaid\ngraph TD; A-->B\n```\n', noAsset);
    expect(katexRender).toHaveBeenCalled(); // span[data-math] survived → KaTeX ran
    expect(mermaidRender).toHaveBeenCalled(); // code.language-mermaid survived → Mermaid ran
  });
});

// The pure heart of the fix: math is lifted out of the Markdown *source* before
// the parser runs, so a formula can never be mangled (underscores, backslashes)
// and a stray `$` can't mis-pair the delimiters. These pin the tricky cases the
// old auto-render-after-markdown path got wrong.
describe('extractMath — pulls formulas out before Markdown, dodging its traps', () => {
  const tex = (s: string) => extractMath(s).math.map((m) => `${m.display ? 'D' : 'I'}:${m.tex}`);

  it('lifts display and inline math, leaving numbered placeholders', () => {
    const { text, math } = extractMath('a $x_i$ b $$y_j$$ c');
    expect(text).toBe('a <span data-math="0"></span> b <span data-math="1"></span> c');
    expect(math).toEqual([
      { tex: 'x_i', display: false },
      { tex: 'y_j', display: true },
    ]);
  });

  it('preserves subscripts and backslashes verbatim (Markdown would eat them)', () => {
    expect(tex('$$ P(\\theta_t|u_t,m) $$')).toEqual(['D: P(\\theta_t|u_t,m) ']);
  });

  it('leaves a lone currency amount alone — `$` before a digit never opens math', () => {
    expect(tex('It costs $5 today.')).toEqual([]);
    expect(tex('From $5 to $10 is a range.')).toEqual([]);
  });

  it('renders the real math even when a stray `$` shares the line', () => {
    expect(tex('costs $5 but $$E=mc^2$$ holds and $x$ too')).toEqual(['D:E=mc^2', 'I:x']);
  });

  it('never grabs a `$` inside code spans or fences', () => {
    expect(tex('run `echo $PATH` then $$a$$')).toEqual(['D:a']);
    expect(tex('```\n$not math$\n```\n$b$')).toEqual(['I:b']);
  });

  it('treats an escaped \\$ as a literal dollar, not a delimiter', () => {
    expect(tex('a \\$5 and \\$6 literal')).toEqual([]);
  });

  it('does not let an unterminated `$` swallow the rest of the note', () => {
    // No closing `$` on the line → left as literal text, zero formulas.
    expect(extractMath('an $unterminated dollar\n\nnext para').math).toEqual([]);
  });
});

describe('renderInto — note references become chips', () => {
  const ID = '01KXGCF248QC70Z7NB4E22A9QJ';
  const ref = `See [Q3 planning](note:${ID}) for context.`;
  /** A resolver standing in for the vault, like NotePanel's real one. */
  const resolveAs = (note: Partial<ResolvedNote> = {}) =>
    vi.fn(
      async (id: string): Promise<ResolvedNote> => ({
        id,
        type: 'meeting',
        title: 'Q3 planning',
        status: 'doing',
        ...note,
      }),
    );
  const noNote = async (): Promise<ResolvedNote | null> => null;

  // The whole `note:` syntax rests on marked passing an unknown URL scheme
  // through to the href untouched. If this ever fails, the reference never
  // reaches resolveNotes and every other test here is meaningless — so assert it
  // directly rather than only inferring it from the chips.
  it('marked passes the `note:` scheme through to the href', async () => {
    const el = pane();
    await renderInto(el, ref, noAsset);
    expect(el.querySelector('a')?.getAttribute('href')).toBe(`note:${ID}`);
  });

  it('replaces the link with a chip carrying the id, type, and live status', async () => {
    const el = pane();
    const resolve = resolveAs();
    await renderInto(el, ref, noAsset, resolve);

    expect(resolve).toHaveBeenCalledWith(ID);
    const chip = el.querySelector<HTMLElement>('button.note-chip');
    expect(chip?.dataset.noteId).toBe(ID);
    expect(chip?.dataset.type).toBe('meeting');
    expect(chip?.querySelector('.note-chip-title')?.textContent).toBe('Q3 planning');
    expect(chip?.querySelector('.note-chip-status')?.textContent).toBe('doing');
    expect(el.querySelector('a')).toBeNull(); // the raw link is gone
  });

  it('omits the status span for a note that has no status', async () => {
    const el = pane();
    await renderInto(el, ref, noAsset, resolveAs({ status: null }));
    expect(el.querySelector('.note-chip-status')).toBeNull();
    expect(el.querySelector('.note-chip-title')?.textContent).toBe('Q3 planning');
  });

  it('degrades a stale reference to a placeholder, keeping the link text', async () => {
    const el = pane();
    await renderInto(el, ref, noAsset, noNote);
    const ph = el.querySelector('.note-missing-inline');
    expect(ph?.textContent).toBe('Q3 planning');
    expect(el.querySelector('.note-chip')).toBeNull();
    expect(el.textContent).toContain('for context.'); // the note still renders
  });

  it('survives a resolver that throws', async () => {
    const el = pane();
    await renderInto(el, ref, noAsset, async () => {
      throw new Error('vault unreachable');
    });
    expect(el.querySelector('.note-missing-inline')?.textContent).toBe('Q3 planning');
  });

  it('leaves ordinary links alone', async () => {
    const el = pane();
    await renderInto(el, '[the spec](https://example.com/spec)', noAsset, resolveAs());
    expect(el.querySelector('a')?.getAttribute('href')).toBe('https://example.com/spec');
    expect(el.querySelector('.note-chip')).toBeNull();
  });

  it('falls back to the live title when the reference has no link text', async () => {
    const el = pane();
    await renderInto(el, `See [](note:${ID}) for context.`, noAsset, resolveAs());
    expect(el.querySelector('.note-chip-title')?.textContent).toBe('Q3 planning');
  });

  // Title and status are user text, and the chip is built right next to an
  // innerHTML sink. textContent is the only thing keeping them text.
  it('never lets a title or status become markup', async () => {
    const el = pane();
    const evil = '<img src=x onerror=alert(1)>';
    await renderInto(el, `[](note:${ID})`, noAsset, resolveAs({ title: evil, status: evil }));
    expect(el.querySelector('.note-chip img')).toBeNull();
    expect(el.querySelector('.note-chip-title')?.textContent).toBe(evil);
    expect(el.querySelector('.note-chip-status')?.textContent).toBe(evil);
  });
});

describe('renderInto — decorative extensions (highlight, colour, callout)', () => {
  it('renders ==text== as a <mark>', async () => {
    const el = pane();
    await renderInto(el, 'a ==key point== here', noAsset);
    expect(el.querySelector('mark')?.textContent).toBe('key point');
  });

  it('renders [text]{.token} as a semantic colour span, and leaves an unknown token literal', async () => {
    const el = pane();
    await renderInto(el, 'this is [important]{.accent} and [nope]{.bogus}', noAsset);
    expect(el.querySelector('span.tk-accent')?.textContent).toBe('important');
    // An unknown token is not a class — the bytes degrade to readable text.
    expect(el.querySelector('.tk-bogus')).toBeNull();
    expect(el.textContent).toContain('[nope]{.bogus}');
  });

  it('renders > [!type] as a callout with a title, and an unknown type stays a blockquote', async () => {
    const el = pane();
    await renderInto(el, '> [!warning] Heads up\n> be careful\n\n> [!bogus] plain\n> quote', noAsset);
    const callout = el.querySelector('div.callout.callout-warning');
    expect(callout).not.toBeNull();
    expect(callout?.querySelector('.callout-title')?.textContent).toBe('Heads up');
    expect(callout?.textContent).toContain('be careful');
    // An unknown type is not a callout — it degrades to an ordinary blockquote.
    expect(el.querySelector('.callout-bogus')).toBeNull();
    expect(el.querySelectorAll('blockquote').length).toBeGreaterThan(0);
  });

  // The extensions run BEFORE the single sanitize, so hostile HTML inside a decoration is still
  // stripped: adding syntax does not move the security boundary.
  it('sanitizes hostile HTML inside a highlight or a callout', async () => {
    const el = pane();
    await renderInto(
      el,
      '==<script>evil()</script>== and\n> [!note] <img src=x onerror=hack()>\n> body',
      noAsset,
    );
    expect(el.querySelector('script')).toBeNull();
    expect(el.querySelector('img[onerror]')).toBeNull();
    expect(el.innerHTML).not.toContain('onerror');
  });
});

describe('renderInto — note embeds (transclusion)', () => {
  const ID = '01ARZ3NDEKTSV4RRFFQ69G5FAV';

  it('inlines the target note body for ![alt](note:id)', async () => {
    const el = pane();
    const resolveEmbed = async (id: string) =>
      id === ID ? { title: 'Target', body: 'embedded **body**' } : null;
    await renderInto(el, `see ![](note:${ID})`, noAsset, undefined, resolveEmbed);
    const embed = el.querySelector('.note-embed');
    expect(embed).not.toBeNull();
    expect(embed?.querySelector('.note-embed-title')?.textContent).toBe('Target');
    // The embedded body was rendered as Markdown, not dumped as text.
    expect(embed?.querySelector('strong')?.textContent).toBe('body');
  });

  it('shows a placeholder for a missing embed target, keeping its label', async () => {
    const el = pane();
    const resolveEmbed = async () => null;
    await renderInto(el, `![the plan](note:${ID})`, noAsset, undefined, resolveEmbed);
    expect(el.querySelector('.note-embed')).toBeNull();
    expect(el.querySelector('.note-missing-inline')?.textContent).toBe('the plan');
  });

  it('stops a cycle (a note embedding itself) with a marker instead of hanging', async () => {
    const el = pane();
    const resolveEmbed = async (id: string) => ({ title: 't', body: `loop ![](note:${id})` });
    await renderInto(el, `![](note:${ID})`, noAsset, undefined, resolveEmbed);
    expect(el.querySelector('.note-embed')).not.toBeNull(); // rendered one level
    expect(el.querySelector('.note-embed-cycle')).not.toBeNull(); // then refused to recurse
  });
});
