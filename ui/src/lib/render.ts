// The read view. Markdown is the literal truth; this renders it *nicely* without
// ever mutating the bytes. Everything heavy is imported lazily on first use, so
// the core bundle stays small (KaTeX, Mermaid, and marked are all separate
// chunks). Every upgrade degrades gracefully: a broken formula or diagram falls
// back to its raw text, and a missing asset falls back to a placeholder — a bad
// note never blanks the pane.
import { marked } from 'marked';
import DOMPurify from 'dompurify';

// A note body is now untrusted input. Before the collaboration work it was only ever the
// author's own text; now bodies arrive from other people through the `.md` merge driver, and
// `render.ts` assigns marked's output straight to `innerHTML`. Without this, a collaborator's
// `<img src=x onerror=…>` runs script in our origin — and fm-serve's CSRF guard allows
// no-Origin requests, so that script can call any `/api/*`: read every note, delete them, or
// set a git remote and push a private vault off the machine. This is the fix for that.
//
// The trap: sanitizing must not eat our OWN pipeline. Three schemes are load-bearing and are
// NOT in DOMPurify's default allow-list — `note:` (a reference chip), `asset:`/`sha256:` (an
// inline blob). marked emits them as `<a href="note:…">` / `<img src="asset:…">`, and
// resolveNotes/resolveAssets read them back by scheme *after* this runs; strip the scheme and
// those images and chips silently vanish. So the default URI regexp is widened by exactly
// those three, and nothing else. (The `<span data-math>` placeholders survive for free —
// span + data-* are allowed by default.) These schemes are inert in a browser and are fully
// replaced before display, so allowing them adds no sink.
const URI_ALLOWED =
  /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|note|asset|sha256):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i;

function sanitize(html: string): string {
  return DOMPurify.sanitize(html, { ALLOWED_URI_REGEXP: URI_ALLOWED });
}

export interface ResolvedAsset {
  url: string;
  mime: string; // '' = unknown; the browser content-sniffs an <img>
}
export type AssetResolver = (ref: string) => Promise<ResolvedAsset | null>;

/** What a `note:` chip needs to draw itself — the live fields, not the body. */
export interface ResolvedNote {
  id: string;
  type: string;
  title: string | null;
  status: string | null;
}
export type NoteResolver = (id: string) => Promise<ResolvedNote | null>;

/** Render `body` into `el`, then upgrade math, diagrams, asset images, and
 *  `note:` references. `resolveNote` is optional: without it a note reference
 *  stays the plain link marked produced, which is what the unit tests and any
 *  caller that has no vault handle want. */
export async function renderInto(
  el: HTMLElement,
  body: string,
  resolveAsset: AssetResolver,
  resolveNote?: NoteResolver,
): Promise<void> {
  // Math is pulled out of the source *before* Markdown so the parser can never
  // mangle a formula (underscores, backslashes, asterisks) and a stray `$` can't
  // mis-pair the delimiters — the two ways `$…$` breaks when Markdown runs first.
  // Each formula leaves an empty <span data-math=i> that survives marked untouched;
  // renderMath fills them with KaTeX afterwards.
  const { text, math } = extractMath(body);
  // Markdown → HTML → sanitized. Fenced ```mermaid becomes <pre><code class="language-mermaid">.
  // Sanitize BEFORE the resolve passes so they operate on already-clean DOM, and so a
  // hostile `onerror` never reaches the parser's output at all.
  el.innerHTML = sanitize(marked.parse(text, { async: false, gfm: true }) as string);
  await resolveAssets(el, resolveAsset);
  if (resolveNote) await resolveNotes(el, resolveNote);
  await renderMath(el, math);
  await renderMermaid(el);
}

export interface MathSpan {
  tex: string;
  display: boolean;
}

/** Pull `$$…$$` (display) and `$…$` (inline) math out of Markdown *source*,
 *  replacing each with an empty `<span data-math=i>` placeholder (returned in
 *  `math`, in order). Display `$$` is matched first as unambiguous pairs; inline
 *  `$` is deliberately conservative — it won't open on whitespace or a digit (so
 *  `$5` currency stays literal), won't cross a `$$`, and won't span a blank line.
 *  Backslash escapes (`\$`) and code spans/fences are copied through verbatim so a
 *  `$` inside them is never treated as math. Pure + synchronous, so it's unit-tested
 *  without a DOM. */
export function extractMath(src: string): { text: string; math: MathSpan[] } {
  const math: MathSpan[] = [];
  const place = (tex: string, display: boolean): string => {
    math.push({ tex, display });
    return `<span data-math="${math.length - 1}"></span>`;
  };
  let out = '';
  let i = 0;
  while (i < src.length) {
    const ch = src[i];
    // A backslash escapes the next char (`\$` is a literal dollar) — copy the pair
    // through so it can't open a formula.
    if (ch === '\\') {
      out += ch + (src[i + 1] ?? '');
      i += 2;
      continue;
    }
    // Code span / fence — copy through untouched so a `$` inside code isn't grabbed.
    if (ch === '`') {
      const run = /^`+/.exec(src.slice(i))![0];
      const close = src.indexOf(run, i + run.length);
      const end = close === -1 ? i + run.length : close + run.length;
      out += src.slice(i, end);
      i = end;
      continue;
    }
    if (ch === '$' && src[i + 1] === '$') {
      const close = src.indexOf('$$', i + 2);
      if (close !== -1) {
        out += place(src.slice(i + 2, close), true);
        i = close + 2;
        continue;
      }
    } else if (ch === '$') {
      const next = src[i + 1];
      if (next && !/[\s\d]/.test(next)) {
        let j = i + 1;
        for (; j < src.length; j++) {
          const c = src[j];
          if (c === '\n' && src[j + 1] === '\n') { j = -1; break; } // never span a blank line
          if (c === '$' && src[j + 1] === '$') { j = -1; break; } // never cross a $$ display
          if (c === '$' && !/\s/.test(src[j - 1]) && !/\d/.test(src[j + 1] ?? '')) break;
        }
        if (j > i && j < src.length) {
          out += place(src.slice(i + 1, j), false);
          i = j + 1;
          continue;
        }
      }
    }
    out += ch;
    i++;
  }
  return { text: out, math };
}

/** Resolve `asset:`/`sha256:` refs to inline media by MIME, or a placeholder. */
async function resolveAssets(el: HTMLElement, resolveAsset: AssetResolver): Promise<void> {
  const imgs = Array.from(el.querySelectorAll('img'));
  for (const img of imgs) {
    const src = img.getAttribute('src') ?? '';
    if (!src.startsWith('asset:') && !src.startsWith('sha256:')) continue;
    const alt = img.getAttribute('alt') ?? '';
    const asset = await resolveAsset(src).catch(() => null);
    if (!asset) {
      const ph = document.createElement('span');
      ph.className = 'asset-missing-inline';
      ph.textContent = alt || 'asset not available';
      img.replaceWith(ph);
      continue;
    }
    upgradeAsset(img, asset, alt);
  }
}

/** Swap the placeholder `<img>` for the element that best fits the MIME. Uses
 *  only native browser elements — an unknown/absent MIME stays an `<img>` (the
 *  browser content-sniffs it), and only `application/pdf` reaches an iframe, so
 *  no arbitrary-HTML sink is added.
 *
 *  What actually stops an HTML file wearing a PDF's MIME from scripting this
 *  origin is **server-side**, not the `sandbox` attribute: `blob.rs` sends
 *  `X-Content-Type-Options: nosniff` with an explicit `Content-Type`, and
 *  `inline_safe()` is an allowlist that hands anything else
 *  `Content-Disposition: attachment`. Both gates have to agree before a byte is
 *  rendered here. This comment used to claim the iframe was sandboxed; it never
 *  was, and it deliberately still is not — every browser's built-in PDF viewer
 *  needs `allow-scripts allow-same-origin`, which together are a no-op, so the
 *  attribute would buy a false sense of security at the cost of the viewer. */
function upgradeAsset(img: HTMLImageElement, asset: ResolvedAsset, alt: string): void {
  const mime = asset.mime;
  if (mime.startsWith('image/') || mime === '') {
    img.src = asset.url;
    return;
  }
  if (mime === 'application/pdf') {
    const frame = document.createElement('iframe');
    frame.className = 'asset-pdf';
    frame.src = asset.url; // native, scrollable, multi-page viewer
    frame.title = alt || 'PDF document';
    replaceWithFigure(img, frame, alt);
    return;
  }
  if (mime.startsWith('video/')) {
    const v = document.createElement('video');
    v.className = 'asset-video';
    v.src = asset.url;
    v.controls = true;
    v.preload = 'metadata';
    replaceWithFigure(img, v, alt);
    return;
  }
  if (mime.startsWith('audio/')) {
    const a = document.createElement('audio');
    a.className = 'asset-audio';
    a.src = asset.url;
    a.controls = true;
    a.preload = 'metadata';
    replaceWithFigure(img, a, alt);
    return;
  }
  const link = document.createElement('a');
  link.className = 'asset-file';
  link.href = asset.url;
  link.target = '_blank';
  link.rel = 'noopener';
  link.textContent = alt ? `Open ${alt}` : 'Open attachment';
  img.replaceWith(link);
}

/** Wrap non-image media in `<figure>`, using the alt text as a `<figcaption>`. */
function replaceWithFigure(img: HTMLImageElement, media: HTMLElement, alt: string): void {
  if (!alt) {
    img.replaceWith(media);
    return;
  }
  const fig = document.createElement('figure');
  fig.className = 'asset-figure';
  fig.appendChild(media);
  const cap = document.createElement('figcaption');
  cap.textContent = alt;
  fig.appendChild(cap);
  img.replaceWith(fig);
}

/** Resolve `note:<id>` links into live chips. Mirrors resolveAssets: marked has
 *  already made each reference an ordinary `<a href="note:…">`, so we only have
 *  to recognise the scheme afterwards — no Markdown parser to extend. An id that
 *  no longer resolves degrades to a visible placeholder keeping the link text,
 *  so a stale reference is obvious but never blanks the pane. */
async function resolveNotes(el: HTMLElement, resolveNote: NoteResolver): Promise<void> {
  const links = Array.from(el.querySelectorAll('a'));
  for (const a of links) {
    const href = a.getAttribute('href') ?? '';
    if (!href.startsWith('note:')) continue;
    const text = a.textContent ?? '';
    const note = await resolveNote(href.slice('note:'.length)).catch(() => null);
    if (!note) {
      const ph = document.createElement('span');
      ph.className = 'note-missing-inline';
      ph.textContent = text || 'note not available';
      a.replaceWith(ph);
      continue;
    }
    a.replaceWith(noteChip(note, text));
  }
}

/** The chip for a resolved note reference: title + live status, keyed by type.
 *  A `<button>` rather than an `<a>` — there is nothing for a `note:` href to
 *  navigate to, and a real href would send the webview to a dead scheme if the
 *  click handler ever missed. A button is focusable and Enter/Space activate it
 *  natively, so the keyboard path costs nothing.
 *
 *  Built from createElement + textContent only, never innerHTML: a note's title
 *  or status is user text, and this keeps it text. `data-type` is the styling
 *  hook (same trick as Card.svelte:38) so no type→icon map lives in JS. */
function noteChip(note: ResolvedNote, text: string): HTMLElement {
  const chip = document.createElement('button');
  chip.type = 'button';
  chip.className = 'note-chip';
  chip.dataset.noteId = note.id;
  chip.dataset.type = note.type;

  const title = document.createElement('span');
  title.className = 'note-chip-title';
  // The link text is the author's own wording for the target; prefer it, and
  // fall back to the live title only when the reference was left untitled.
  title.textContent = text || note.title || note.id;
  chip.appendChild(title);

  if (note.status) {
    const status = document.createElement('span');
    status.className = 'note-chip-status';
    status.textContent = note.status;
    chip.appendChild(status);
  }
  return chip;
}

/** Fill each `<span data-math=i>` placeholder with KaTeX, lazily. A formula that
 *  fails to parse keeps its raw `$…$` source with the KaTeX message as a tooltip
 *  (and a console warning) instead of a silent red blob — so a bad formula is
 *  visible *and* diagnosable, and the note never blanks. */
async function renderMath(el: HTMLElement, math: MathSpan[]): Promise<void> {
  if (math.length === 0) return; // exact gate — no formulas, no import
  const hosts = Array.from(el.querySelectorAll<HTMLElement>('span[data-math]'));
  if (hosts.length === 0) return;
  const raw = (s: MathSpan) => (s.display ? `$$${s.tex}$$` : `$${s.tex}$`);
  let mod: typeof import('katex');
  try {
    mod = await import('katex');
    await import('katex/dist/katex.min.css');
  } catch {
    // KaTeX chunk unavailable — show the raw source rather than an empty span.
    for (const host of hosts) {
      const s = math[Number(host.dataset.math)];
      if (s) host.textContent = raw(s);
    }
    return;
  }
  const katex = mod.default;
  for (const host of hosts) {
    const s = math[Number(host.dataset.math)];
    if (!s) continue;
    try {
      host.innerHTML = katex.renderToString(s.tex, { displayMode: s.display, throwOnError: true });
    } catch (e) {
      host.className = 'math-error';
      host.textContent = raw(s);
      host.title = e instanceof Error ? e.message : String(e);
      console.warn('KaTeX could not render:', raw(s), e);
    }
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
        // The diagram source is note-controlled, i.e. untrusted, and this is an innerHTML
        // sink. The control is Mermaid's own `securityLevel: 'strict'` set above — it strips
        // HTML from labels and disables click-bound scripts, which is the mechanism designed
        // for exactly this. We deliberately do NOT re-run DOMPurify here: its SVG profile can
        // drop the `foreignObject` Mermaid uses for text wrapping, a visual regression headless
        // CI cannot see, traded against a control that already holds. Keep 'strict'.
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
