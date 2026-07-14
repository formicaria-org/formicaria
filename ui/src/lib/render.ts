// The read view. Markdown is the literal truth; this renders it *nicely* without
// ever mutating the bytes. Everything heavy is imported lazily on first use, so
// the core bundle stays small (KaTeX, Mermaid, and marked are all separate
// chunks). Every upgrade degrades gracefully: a broken formula or diagram falls
// back to its raw text, and a missing asset falls back to a placeholder — a bad
// note never blanks the pane.
import { marked } from 'marked';

export type AssetResolver = (ref: string) => Promise<string | null>;

/** Render `body` into `el`, then upgrade math, diagrams, and asset images. */
export async function renderInto(
  el: HTMLElement,
  body: string,
  resolveAsset: AssetResolver,
): Promise<void> {
  // Markdown → HTML. Fenced ```mermaid becomes <pre><code class="language-mermaid">.
  el.innerHTML = marked.parse(body, { async: false, gfm: true }) as string;
  await resolveAssets(el, resolveAsset);
  await renderMath(el);
  await renderMermaid(el);
}

/** Resolve `asset:`/`sha256:` image sources, or swap in an inline placeholder. */
async function resolveAssets(el: HTMLElement, resolveAsset: AssetResolver): Promise<void> {
  const imgs = Array.from(el.querySelectorAll('img'));
  for (const img of imgs) {
    const src = img.getAttribute('src') ?? '';
    if (!src.startsWith('asset:') && !src.startsWith('sha256:')) continue;
    const url = await resolveAsset(src).catch(() => null);
    if (url) {
      img.src = url;
    } else {
      const ph = document.createElement('span');
      ph.className = 'asset-missing-inline';
      ph.textContent = img.getAttribute('alt') || 'asset not available';
      img.replaceWith(ph);
    }
  }
}

/** KaTeX, lazily. Auto-render walks text nodes and skips code/pre by default. */
async function renderMath(el: HTMLElement): Promise<void> {
  if (!(el.textContent ?? '').includes('$')) return; // cheap gate
  try {
    const renderMathInElement = (await import('katex/dist/contrib/auto-render.js')).default;
    await import('katex/dist/katex.min.css');
    renderMathInElement(el, {
      delimiters: [
        { left: '$$', right: '$$', display: true },
        { left: '$', right: '$', display: false },
      ],
      throwOnError: false,
    });
  } catch {
    // KaTeX unavailable — leave the raw $…$ text untouched.
  }
}

/** Mermaid, lazily. Each ```mermaid block becomes an inline SVG diagram. */
async function renderMermaid(el: HTMLElement): Promise<void> {
  const blocks = Array.from(el.querySelectorAll('code.language-mermaid'));
  if (blocks.length === 0) return;
  try {
    const mermaid = (await import('mermaid')).default;
    mermaid.initialize({ startOnLoad: false, theme: 'dark', securityLevel: 'strict' });
    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];
      const host = block.closest('pre') ?? block;
      const code = block.textContent ?? '';
      try {
        const { svg } = await mermaid.render(`mmd-${i}-${code.length}`, code);
        const wrap = document.createElement('div');
        wrap.className = 'mermaid-diagram';
        wrap.innerHTML = svg;
        host.replaceWith(wrap);
      } catch {
        // Diagram didn't parse — leave the code block as written.
      }
    }
  } catch {
    // Mermaid failed to load — code blocks stay as code.
  }
}
