// **A formula on a page, as LaTeX** (owner, 2026-11-17: *"if there is a formula it can propose a
// LaTeX code … if not, port the image with the formula"*). Notes show `$…$` and `$$…$$` with KaTeX
// (`render.ts`), so a formula brought in as its source stays a formula: editable, searchable, sharp.
//
// Pages that draw formulas nearly always keep their source next to the drawing, for screen readers
// or copying, and that source is read here, never guessed from the picture:
//
// - **MathML with a TeX annotation**: Wikipedia (`<annotation encoding="application/x-tex">`), and
//   KaTeX's output (its hidden `.katex-mathml`);
// - **MathML's `alttext`**: arXiv's HTML papers (LaTeXML), Wikipedia;
// - **Wikipedia's formula pictures**: their `alt` is the source, `{\displaystyle …}`;
// - **MathJax 2**: the source in `<script type="math/tex">` beside the drawing;
// - **`data-latex` / `data-tex`** attributes (several editors and blogs).
//
// What has none of these (MathJax 3 without its accessibility extension; a formula in a picture or a
// PDF) is not read: the person brings in its picture (the selection's image, or Snip).

export interface Formula {
  tex: string;
  /** On its own line (`$$…$$`) rather than inside a sentence (`$…$`). */
  display: boolean;
}

/** The elements that hold one formula each, outermost first: a whole formula is replaced, never a
 *  part of one. */
const HOLDERS = [
  '.mwe-math-element',
  '.katex-display',
  '.katex',
  'mjx-container',
  '.MathJax_Display',
  '.MathJax',
  '.MathJax_Preview',
  'script[type^="math/tex"]',
  'math',
  'img.mwe-math-fallback-image-inline',
  'img.mwe-math-fallback-image-display',
  'img[alt^="{\\displaystyle"]',
  'img[alt^="{\\textstyle"]',
  '[data-latex]',
  '[data-tex]',
].join(', ');

/** `{\displaystyle a^{2}}` → `a^{2}`: Wikipedia's wrapper, which only says how big to draw it. */
function unwrap(tex: string): string {
  const t = tex.trim();
  const m = /^\{\\(?:displaystyle|textstyle)\s*([\s\S]*)\}$/.exec(t);
  return (m ? m[1] : t).trim();
}

const isDisplay = (el: Element): boolean =>
  el.matches(
    '.katex-display, .MathJax_Display, img.mwe-math-fallback-image-display, mjx-container[display="true"], math[display="block"], script[type*="mode=display"]',
  ) ||
  !!el.querySelector(
    '.katex-display, img.mwe-math-fallback-image-display, math[display="block"], mjx-container[display="true"]',
  ) ||
  /^\{\\displaystyle/.test(el.getAttribute('alt') ?? '') ||
  (el.classList.contains('mwe-math-element') &&
    !!el.querySelector('.mwe-math-mathml-display, .mwe-math-fallback-image-display'));

/** The LaTeX source of one formula element, if the page kept it. */
export function texOf(el: Element): Formula | null {
  const display = isDisplay(el);
  const found = (tex: string | null | undefined) => {
    const t = unwrap(tex ?? '');
    return t ? { tex: t, display } : null;
  };
  const attr = el.getAttribute('data-latex') ?? el.getAttribute('data-tex');
  if (attr) return found(attr);
  if (el.matches('script[type^="math/tex"]')) return found(el.textContent);
  const ann = el.matches('annotation')
    ? el
    : el.querySelector('annotation[encoding="application/x-tex"], annotation[encoding="TeX"]');
  if (ann?.textContent?.trim()) return found(ann.textContent);
  const math = el.matches('math') ? el : el.querySelector('math');
  if (math?.getAttribute('alttext')) return found(math.getAttribute('alttext'));
  const img = el.matches('img') ? el : el.querySelector('img');
  const alt = img?.getAttribute('alt') ?? '';
  if (/^\{\\(?:displaystyle|textstyle)/.test(alt)) return found(alt);
  if (img && img.classList.contains('mwe-math-fallback-image-inline')) return found(alt);
  if (img && img.classList.contains('mwe-math-fallback-image-display')) return found(alt);
  const inner = el.querySelector('[data-latex], [data-tex], script[type^="math/tex"]');
  return inner ? texOf(inner) : null;
}

/** As a note writes it. */
export function asMarkdown(f: Formula): string {
  return f.display ? `$$${f.tex}$$` : `$${f.tex}$`;
}

/** The outermost formula holders under `root`. */
function holders(root: ParentNode): Element[] {
  const all = Array.from(root.querySelectorAll(HOLDERS));
  return all.filter((el) => !all.some((o) => o !== el && o.contains(el)));
}

/** **A passage's formulas, taken out before it is turned into Markdown**: each becomes a marker in the
 *  HTML, and `restore` puts its `$…$` back afterwards, so the conversion can never escape a `_` or a
 *  `\` inside one. A formula whose source the page did not keep is left as it was. */
export function takeFormulas(html: string): {
  html: string;
  count: number;
  restore: (markdown: string) => string;
} {
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  const found: string[] = [];
  // MathJax 2 draws a formula beside its `<script>`: the drawing goes, the script speaks for both.
  for (const el of Array.from(
    doc.body.querySelectorAll('.MathJax_Preview, .MathJax, .MathJax_Display'),
  )) {
    const next = el.nextElementSibling;
    if (next?.matches('script[type^="math/tex"]') || next?.matches('.MathJax, .MathJax_Display'))
      el.remove();
  }
  for (const el of holders(doc.body)) {
    const f = texOf(el);
    if (!f) continue;
    const mark = `FMFORMULA${found.length}X`;
    found.push(asMarkdown(f));
    if (f.display) {
      const p = doc.createElement('p');
      p.textContent = mark;
      el.replaceWith(p);
    } else el.replaceWith(doc.createTextNode(mark));
  }
  return {
    html: found.length ? doc.body.innerHTML : html,
    count: found.length,
    restore: (md) => md.replace(/FMFORMULA(\d+)X/g, (_, i) => found[Number(i)] ?? ''),
  };
}

/** **The selection is one formula** (nothing else but space around it): its source, or null. */
export function onlyFormula(html: string): Formula | null {
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  const hs = holders(doc.body).filter((el) => texOf(el));
  if (hs.length !== 1) return null;
  const f = texOf(hs[0])!;
  hs[0].remove();
  for (const s of Array.from(
    doc.body.querySelectorAll('.MathJax_Preview, .MathJax, .MathJax_Display'),
  ))
    s.remove();
  return (doc.body.textContent ?? '').trim() ? null : f;
}
