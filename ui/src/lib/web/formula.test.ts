import { describe, expect, it } from 'vitest';
import { asMarkdown, onlyFormula, takeFormulas, texOf } from './formula';

const el = (html: string) => {
  const d = document.createElement('div');
  d.innerHTML = html;
  return d.firstElementChild!;
};

// The shapes as the pages write them (shortened, the attributes that matter kept).
const WIKIPEDIA_INLINE =
  '<span class="mwe-math-element"><span class="mwe-math-mathml-inline mwe-math-mathml-a11y" style="display: none;"><math xmlns="http://www.w3.org/1998/Math/MathML" alttext="{\\displaystyle a^{2}+b^{2}=c^{2}}"><semantics><mrow><msup><mi>a</mi><mn>2</mn></msup></mrow><annotation encoding="application/x-tex">{\\displaystyle a^{2}+b^{2}=c^{2}}</annotation></semantics></math></span><img src="https://wikimedia.org/api/rest_v1/media/math/render/svg/abc" class="mwe-math-fallback-image-inline" aria-hidden="true" alt="{\\displaystyle a^{2}+b^{2}=c^{2}}"></span>';
const WIKIPEDIA_DISPLAY =
  '<span class="mwe-math-element"><span class="mwe-math-mathml-display mwe-math-mathml-a11y" style="display: none;"><math display="block" alttext="{\\displaystyle \\int_0^1 x\\,dx}"><semantics><mrow></mrow><annotation encoding="application/x-tex">{\\displaystyle \\int_0^1 x\\,dx}</annotation></semantics></math></span><img class="mwe-math-fallback-image-display" alt="{\\displaystyle \\int_0^1 x\\,dx}"></span>';
const KATEX =
  '<span class="katex"><span class="katex-mathml"><math><semantics><mrow></mrow><annotation encoding="application/x-tex">e^{i\\pi}+1=0</annotation></semantics></math></span><span class="katex-html" aria-hidden="true">e iπ + 1 = 0</span></span>';
const KATEX_DISPLAY = `<span class="katex-display">${KATEX}</span>`;
const ARXIV =
  '<math alttext="\\mathcal{O}(n\\log n)" display="inline"><semantics><mrow></mrow></semantics></math>';
const MATHJAX2 =
  '<span class="MathJax_Preview"></span><span class="MathJax" id="MathJax-Element-1-Frame">x²</span><script type="math/tex" id="MathJax-Element-1">x^2</script>';
const DATA_LATEX = '<span data-latex="\\frac{a}{b}">a/b</span>';

describe('a formula as LaTeX', () => {
  it("reads each page's own source, and whether it stands on its own line", () => {
    expect(texOf(el(WIKIPEDIA_INLINE))).toEqual({ tex: 'a^{2}+b^{2}=c^{2}', display: false });
    expect(texOf(el(WIKIPEDIA_DISPLAY))).toEqual({ tex: '\\int_0^1 x\\,dx', display: true });
    expect(texOf(el(KATEX))).toEqual({ tex: 'e^{i\\pi}+1=0', display: false });
    expect(texOf(el(KATEX_DISPLAY))).toEqual({ tex: 'e^{i\\pi}+1=0', display: true });
    expect(texOf(el(ARXIV))).toEqual({ tex: '\\mathcal{O}(n\\log n)', display: false });
    expect(texOf(el(DATA_LATEX))).toEqual({ tex: '\\frac{a}{b}', display: false });
    // Wikipedia's picture alone (a selection that caught only the image).
    expect(
      texOf(el('<img class="mwe-math-fallback-image-inline" alt="{\\displaystyle \\pi r^{2}}">')),
    ).toEqual({
      tex: '\\pi r^{2}',
      display: true,
    });
  });

  it('is not guessed when the page kept no source', () => {
    expect(texOf(el('<mjx-container class="MathJax"><svg></svg></mjx-container>'))).toBeNull();
    expect(texOf(el('<img alt="a diagram">'))).toBeNull();
  });

  it("writes it as a note's formula", () => {
    expect(asMarkdown({ tex: 'x^2', display: false })).toBe('$x^2$');
    expect(asMarkdown({ tex: 'x^2', display: true })).toBe('$$x^2$$');
  });
});

describe('a passage with formulas in it', () => {
  it('keeps each as its source, untouched by the Markdown conversion', () => {
    const t = takeFormulas(
      `<p>By Pythagoras, ${WIKIPEDIA_INLINE} for a right triangle; and ${KATEX}.</p>`,
    );
    expect(t.count).toBe(2);
    expect(t.html).not.toContain('<math');
    expect(t.html).not.toContain('katex-html');
    // As if converted (the conversion would escape `_` and `\`; the markers carry neither).
    const md = t.restore(t.html.replace(/<[^>]+>/g, ''));
    expect(md).toBe(
      'By Pythagoras, $a^{2}+b^{2}=c^{2}$ for a right triangle; and $e^{i\\pi}+1=0$.',
    );
  });

  it('puts a displayed formula on a line of its own', () => {
    const t = takeFormulas(`<p>The area:</p>${WIKIPEDIA_DISPLAY}<p>done.</p>`);
    expect(t.restore(t.html)).toContain('<p>$$\\int_0^1 x\\,dx$$</p>');
  });

  it('reads MathJax 2 once, by its script, without the drawing beside it', () => {
    const t = takeFormulas(`<p>Here ${MATHJAX2} grows.</p>`);
    expect(t.restore(t.html.replace(/<[^>]+>/g, ''))).toBe('Here $x^2$ grows.');
  });

  it('leaves a passage without formulas exactly as it was', () => {
    const html = '<p>No <b>formulas</b> here, only a_b and \\c.</p>';
    expect(takeFormulas(html)).toMatchObject({ html, count: 0 });
  });
});

describe('a selection that is one formula', () => {
  it('is recognised as one, so it can be offered as LaTeX', () => {
    expect(onlyFormula(` ${WIKIPEDIA_INLINE} `)).toEqual({
      tex: 'a^{2}+b^{2}=c^{2}',
      display: false,
    });
    expect(onlyFormula(KATEX_DISPLAY)?.display).toBe(true);
    expect(onlyFormula(MATHJAX2)?.tex).toBe('x^2');
  });

  it('is not, when words or a second formula come with it', () => {
    expect(onlyFormula(`<p>so ${KATEX}</p>`)).toBeNull();
    expect(onlyFormula(`${KATEX} ${ARXIV}`)).toBeNull();
    expect(onlyFormula('<p>just words</p>')).toBeNull();
  });
});

// Wikipedia's pages as served on 2026-11-17 (Parsoid, native MathML): found by `usability-check`
// on *Pythagorean theorem*, where the selection came in as "a2+b2=c2".
const WIKIPEDIA_NATIVE =
  '<math xmlns="http://www.w3.org/1998/Math/MathML" class="mwe-math-element mwe-math-element-block" display="block" typeof="mw:Extension/math" data-mw=\'{"name":"math","attrs":{"display":"block"},"body":{"extsrc":"a^2 + b^2 = c^2 ."}}\'><mrow><mstyle displaystyle="true"><msup><mi>a</mi><mn>2</mn></msup><mo>+</mo><msup><mi>b</mi><mn>2</mn></msup><mo>=</mo><msup><mi>c</mi><mn>2</mn></msup><mo>.</mo></mstyle></mrow></math>';

it("reads Wikipedia's native MathML by the source it keeps", () => {
  expect(texOf(el(WIKIPEDIA_NATIVE))).toEqual({ tex: 'a^2 + b^2 = c^2 .', display: true });
  expect(onlyFormula(WIKIPEDIA_NATIVE)?.tex).toBe('a^2 + b^2 = c^2 .');
  const t = takeFormulas(`<p>It reads</p>${WIKIPEDIA_NATIVE}<p>for a right triangle.</p>`);
  expect(t.restore(t.html)).toContain('$$a^2 + b^2 = c^2 .$$');
});
