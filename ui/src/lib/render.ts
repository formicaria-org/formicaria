// The read view. Markdown is the literal truth; this renders it *nicely* without
// ever mutating the bytes. Everything heavy is imported lazily on first use, so
// the core bundle stays small (KaTeX, Mermaid, and marked are all separate
// chunks). Every upgrade degrades gracefully: a broken formula or diagram falls
// back to its raw text, and a missing asset falls back to a placeholder — a bad
// note never blanks the pane.
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { TEXT_TOKENS, CALLOUT_TYPES } from './render-vocab';

// Decorative Markdown extensions, registered once at module load. **They run inside
// `marked.parse` (below), upstream of the single `sanitize()` call — never as a post-`innerHTML`
// pass.** That is the maintainable dividing line already in this file: pure *syntax→HTML* belongs
// in a marked extension (pre-sanitize, so there is no new pass to misorder and no bytes are
// mutated); only *async vault resolution* is a post-sanitize DOM walk (`resolveAssets`/
// `resolveNotes`). Each extension emits a DOMPurify-default-allowed element (`mark`/`span`/`div`)
// with a class from a CLOSED vocabulary (`render-vocab.ts`); the sanitizer never learns the
// vocabulary, and an unknown value degrades to literal, readable Markdown (MASTERPLAN:326).
marked.use({
  extensions: [
    // `==highlight==` → `<mark>`. Near-standard (Obsidian/CommonMark); degrades to literal `==`.
    {
      name: 'highlight',
      level: 'inline',
      start(src: string) {
        return src.indexOf('==');
      },
      tokenizer(src: string) {
        const m = /^==(?=\S)([\s\S]*?\S)==(?!=)/.exec(src);
        if (m) return { type: 'highlight', raw: m[0], tokens: this.lexer.inlineTokens(m[1]) };
      },
      renderer(token) {
        return `<mark>${this.parser.parseInline(token.tokens ?? [])}</mark>`;
      },
    },
    // `[text]{.token}` → `<span class="tk-token">`, a semantic colour (Pandoc/djot bracketed-span
    // attributes). A plain viewer shows `[text]{.token}` with the words intact; `.token` is a
    // named intent, never a colour/hex, so it stays truthful across themes. Unknown → literal.
    {
      name: 'coloredText',
      level: 'inline',
      start(src: string) {
        return src.indexOf('[');
      },
      tokenizer(src: string) {
        const m = /^\[([^\]\n]+)\]\{\.([a-z]+)\}/.exec(src);
        if (m && (TEXT_TOKENS as readonly string[]).includes(m[2])) {
          return { type: 'coloredText', raw: m[0], token: m[2], tokens: this.lexer.inlineTokens(m[1]) };
        }
      },
      renderer(token) {
        return `<span class="tk-${token.token}">${this.parser.parseInline(token.tokens ?? [])}</span>`;
      },
    },
    // `> [!type] title` → `<div class="callout callout-type">`. GitHub/Obsidian standard; a plain
    // viewer shows an ordinary blockquote with a visible `[!type]` line. Closed type set → an
    // unknown kind stays a plain blockquote.
    {
      name: 'callout',
      level: 'block',
      start(src: string) {
        return src.indexOf('> [!');
      },
      tokenizer(src: string) {
        const m = /^> \[!([a-z]+)\]([^\n]*)((?:\n>[^\n]*)*)/.exec(src);
        if (!m || !(CALLOUT_TYPES as readonly string[]).includes(m[1])) return;
        const inner = m[3].replace(/^\n/, '').split('\n').map((l) => l.replace(/^>\s?/, '')).join('\n');
        return {
          type: 'callout',
          raw: m[0],
          calloutType: m[1],
          titleTokens: this.lexer.inlineTokens(m[2].trim()),
          tokens: this.lexer.blockTokens(inner),
        };
      },
      renderer(token) {
        const title = this.parser.parseInline(token.titleTokens ?? []);
        const body = this.parser.parse(token.tokens ?? []);
        const head = title ? `<p class="callout-title">${title}</p>` : '';
        // A type badge, GitHub-style: it names the callout's kind AND is the tap handle the note
        // editor wires to a type picker (`kind` is a closed-vocabulary word, validated above). In a
        // read-only context (an embed, a preview) it is just an inert label.
        const kind = `<button type="button" class="callout-kind" data-kind="${token.calloutType}">${token.calloutType}</button>`;
        return `<div class="callout callout-${token.calloutType}">${kind}${head}${body}</div>`;
      },
    },
  ],
});

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
//
// `geo:` is allowed too, but for a different reason: it is a *terminal* link, not a pipeline
// placeholder — `geo:<lat>,<lon>` (RFC 5870) hands a place's coordinates to the OS map app on
// click. It runs no script and, crucially, formicaria fetches nothing for it (the no-phone-home
// line holds — the handoff is the OS's, not ours), so it is as inert as `tel:`/`mailto:`.
const URI_ALLOWED =
  /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|geo|note|asset|sha256):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i;

function sanitize(html: string): string {
  return DOMPurify.sanitize(html, { ALLOWED_URI_REGEXP: URI_ALLOWED });
}

export interface ResolvedAsset {
  url: string;
  mime: string; // '' = unknown; the browser content-sniffs an <img>
  /** A downscaled copy, when the transport can point at one.
   *
   *  **Images only, and deliberately optional.** A 400x400 webp is not a video poster and not a
   *  PDF, so only the `<img>` branch reads it; every other branch keeps `url`. Absent on the mock
   *  transport, which hands back object URLs and has no second copy to offer — so the fallback is
   *  the ordinary case, not the error case. */
  thumb?: string;
}
/** Resolved bytes, or **why not**.
 *
 *  The reason is part of the contract rather than a console line, because on a real device the
 *  console is not readable: an Android WebView routes nothing to logcat by default and MIUI
 *  suppresses what is left. A note that renders its own filename as plain text is identical
 *  whether the bytes are in another vault, absent, or the URL is wrong — three problems with
 *  three different fixes, previously indistinguishable on the one screen anyone can see. */
export type AssetFailure = { reason: string };
export type AssetResolver = (ref: string) => Promise<ResolvedAsset | AssetFailure | null>;

/** What a `note:` chip needs to draw itself — the live fields, not the body. */
export interface ResolvedNote {
  id: string;
  type: string;
  title: string | null;
  status: string | null;
}
export type NoteResolver = (id: string) => Promise<ResolvedNote | null>;

/** What a `![alt](note:id)` **embed** needs: the target's title and its raw body, rendered inline
 *  (transclusion). Whole-note only — an embed is a *file*, never a block, so it never needs the
 *  per-block ids that would change the atom (MASTERPLAN:110). */
export interface ResolvedEmbed {
  title: string | null;
  body: string;
}
export type EmbedResolver = (id: string) => Promise<ResolvedEmbed | null>;

/** Render `body` into `el`, then upgrade math, diagrams, asset images, and
 *  `note:` references. `resolveNote` is optional: without it a note reference
 *  stays the plain link marked produced, which is what the unit tests and any
 *  caller that has no vault handle want. */
export async function renderInto(
  el: HTMLElement,
  body: string,
  resolveAsset: AssetResolver,
  resolveNote?: NoteResolver,
  resolveEmbed?: EmbedResolver,
  depth = 0,
  seen: Set<string> = new Set(),
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
  el.innerHTML = sanitize(marked.parse(text, { async: false, gfm: true }) as string); // sink-ok: DOMPurify-sanitized
  await resolveAssets(el, resolveAsset);
  if (resolveNote) await resolveNotes(el, resolveNote);
  if (resolveEmbed) await resolveEmbeds(el, resolveAsset, resolveNote, resolveEmbed, depth, seen);
  await renderMath(el, math);
  await renderMermaid(el);
}

/** The most a `![](note:…)` embed may nest before it stops recursing — a hand-built chain of
 *  embeds must not push the pane off the side of the screen or (with a cycle) never terminate. */
const MAX_EMBED_DEPTH = 3;

/** Resolve `![alt](note:id)` embeds — a `<img src="note:…">` marked produced — into the target
 *  note's **rendered body**, inline. Mirrors `resolveAssets`/`resolveNotes` (recognise the scheme
 *  after sanitize), but recurses: the embedded body is rendered by `renderInto` again, so it is
 *  sanitized at its own level and its own chips/assets/embeds resolve too. `seen` (the ancestor
 *  ids) plus `MAX_EMBED_DEPTH` make a cycle (`a` embeds `b` embeds `a`) or a very deep chain
 *  terminate with a visible marker rather than a hang — the same defensive stance the thread
 *  reader takes with `reply_to`. A missing target degrades to a placeholder keeping its label. */
async function resolveEmbeds(
  el: HTMLElement,
  resolveAsset: AssetResolver,
  resolveNote: NoteResolver | undefined,
  resolveEmbed: EmbedResolver,
  depth: number,
  seen: Set<string>,
): Promise<void> {
  const imgs = Array.from(el.querySelectorAll('img'));
  for (const img of imgs) {
    const src = img.getAttribute('src') ?? '';
    if (!src.startsWith('note:')) continue;
    const id = src.slice('note:'.length);
    const label = img.getAttribute('alt') ?? '';
    if (depth >= MAX_EMBED_DEPTH || seen.has(id)) {
      const ph = document.createElement('span');
      ph.className = 'note-embed-cycle';
      ph.textContent = label || `↪ ${id}`;
      img.replaceWith(ph);
      continue;
    }
    const embed = await resolveEmbed(id).catch(() => null);
    if (!embed) {
      const ph = document.createElement('span');
      ph.className = 'note-missing-inline';
      ph.textContent = label || 'note not available';
      img.replaceWith(ph);
      continue;
    }
    const wrap = document.createElement('div');
    wrap.className = 'note-embed';
    const head = document.createElement('div');
    head.className = 'note-embed-title';
    // Title is user text — textContent, never innerHTML.
    head.textContent = label || embed.title || id;
    wrap.appendChild(head);
    const bodyEl = document.createElement('div');
    wrap.appendChild(bodyEl);
    const nextSeen = new Set(seen);
    nextSeen.add(id);
    await renderInto(bodyEl, embed.body, resolveAsset, resolveNote, resolveEmbed, depth + 1, nextSeen);
    img.replaceWith(wrap);
  }
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

/** Resolve `asset:`/`sha256:` refs to inline media by MIME, or a placeholder.
 *
 *  **Concurrent, and deduplicated by reference.** This was a `for … await` loop, so a note with
 *  fifteen images cost fifteen *sequential* backend round trips before the read view was usable —
 *  and on Android each of those parked the WebView's JS thread, which is a large part of why
 *  opening an image-heavy note felt like a hang. Each iteration touches a distinct element and the
 *  list is captured up front, so there is no ordering dependency to preserve.
 *
 *  The dedupe matters as much as the concurrency: the same asset referenced twice in a note used
 *  to be asked about twice. `asset_status` is a lock-taking command, so that is not free. */
async function resolveAssets(el: HTMLElement, resolveAsset: AssetResolver): Promise<void> {
  const imgs = Array.from(el.querySelectorAll('img')).filter((img) => {
    const src = img.getAttribute('src') ?? '';
    return src.startsWith('asset:') || src.startsWith('sha256:');
  });
  // **A *link* to an asset, not an embed of one** — `[p. 4](asset:sha256-…#page=4)`, how a note
  // points at a place in a PDF rather than inlining it.
  //
  // This pass used to walk `img` only, and `URI_ALLOWED` deliberately lets the `asset:` scheme
  // through the sanitiser — so the anchor survived as a live link to a scheme nothing resolves.
  // On Android that navigates the WebView out of the app, which is the exact trap `noteChip`
  // documents for `note:`. Here the fix is *not* a button: unlike `note:`, this resolves to a real
  // same-origin URL, so an `<a href>` is the honest element and the fragment does its job.
  const links = Array.from(el.querySelectorAll('a')).filter((a) => {
    // Lowercased first: `URI_ALLOWED` carries `/i`, so `ASSET:` survives the sanitiser too — and a
    // scheme that slips this filter stays a *live* link to something nothing resolves, which is
    // the navigate-out-of-the-WebView trap this pass exists to close.
    const href = (a.getAttribute('href') ?? '').toLowerCase();
    return href.startsWith('asset:') || href.startsWith('sha256:');
  });
  // One in-flight promise per distinct reference, shared by every element that names it.
  const inflight = new Map<string, ReturnType<AssetResolver>>();
  const ask = (src: string) => {
    const existing = inflight.get(src);
    if (existing) return existing;
    const p = resolveAsset(src).catch((e) => ({
      reason: e instanceof Error ? e.message : String(e),
    }));
    inflight.set(src, p);
    return p;
  };

  await Promise.all(
    links.map(async (a) => {
      const href = a.getAttribute('href') ?? '';
      const asset = await ask(href);
      if (!asset || 'reason' in asset) {
        // Same shape as a missing embed: the line still reads as the thing that is gone, and says
        // why. A dead `asset:` href left in place would be worse than no link at all.
        const ph = document.createElement('span');
        ph.className = 'asset-missing-inline';
        const why = asset && 'reason' in asset ? asset.reason : 'not available';
        ph.textContent = `${a.textContent || 'asset'} — ${why}`;
        a.replaceWith(ph);
        return;
      }
      a.setAttribute('href', asset.url);
      // A new tab: the note is the thing being read, and replacing it with a PDF loses the reader's
      // place. `noopener` because the blob is served from our own origin.
      a.setAttribute('target', '_blank');
      a.setAttribute('rel', 'noopener');
      a.classList.add('asset-link');
    }),
  );

  await Promise.all(
    imgs.map(async (img) => {
      const src = img.getAttribute('src') ?? '';
      const alt = img.getAttribute('alt') ?? '';
      const asset = await ask(src);
      if (!asset || 'reason' in asset) {
        const ph = document.createElement('span');
        ph.className = 'asset-missing-inline';
        // The label the note gave it, then the reason — so the line still reads as the thing that
        // is missing, and says what happened to it.
        const why = asset && 'reason' in asset ? asset.reason : 'not available';
        ph.textContent = `${alt || 'asset'} — ${why}`;
        img.replaceWith(ph);
        return;
      }
      upgradeAsset(img, asset, alt);
    }),
  );
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
    // **The small copy when there is one, the full blob when there is not** (2026-09-04).
    //
    // Every inline image used to decode at whatever resolution the camera produced. A 12 MP JPEG
    // is ~50 MB of decoded pixels; five in one note is a renderer kill on a phone, and the kill is
    // silent — `onRenderProcessGone` is unhandled, so the framework default takes the process and
    // the app simply vanishes.
    //
    // **The fallback is the load-bearing half, not the thumbnail.** `vipsthumbnail` does not exist
    // on Android, so *every* image ingested on a phone has a blob and no derivative. Asking for
    // `?kind=thumb` used to be a hard error, which would have turned each one into a 404
    // placeholder — a performance fix that breaks the picture. `resolve_asset_bytes` now degrades
    // to the full blob (`commands::blob_path_of_kind`), so this is safe to ask for everywhere.
    //
    // `lazy` and `async` stay: they are what keeps the images below the fold costing nothing, and
    // they help most on exactly the vaults where no thumbnail exists.
    img.src = asset.thumb ?? asset.url;
    img.loading = 'lazy';
    img.decoding = 'async';
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
  const links = Array.from(el.querySelectorAll('a')).filter((a) =>
    (a.getAttribute('href') ?? '').startsWith('note:'),
  );
  // Same shape as `resolveAssets`: concurrent, and one request per distinct id. A note that
  // mentions the same reference five times used to cost five `get` round trips — sequentially,
  // and on the phone each one blocking the UI thread.
  const inflight = new Map<string, ReturnType<NoteResolver>>();
  const ask = (id: string) => {
    const existing = inflight.get(id);
    if (existing) return existing;
    const p = resolveNote(id).catch(() => null);
    inflight.set(id, p);
    return p;
  };

  await Promise.all(
    links.map(async (a) => {
      const href = a.getAttribute('href') ?? '';
      const text = a.textContent ?? '';
      const note = await ask(href.slice('note:'.length));
      if (!note) {
        const ph = document.createElement('span');
        ph.className = 'note-missing-inline';
        ph.textContent = text || 'note not available';
        a.replaceWith(ph);
        return;
      }
      a.replaceWith(noteChip(note, text));
    }),
  );
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
      host.innerHTML = katex.renderToString(s.tex, { displayMode: s.display, throwOnError: true }); // sink-ok: KaTeX, throwOnError
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
        wrap.innerHTML = svg; // sink-ok: Mermaid securityLevel:'strict' (see comment above)
        host.replaceWith(wrap);
      } catch {
        // Diagram didn't parse — leave the code block as written.
      }
    }
  } catch {
    // Mermaid failed to load — code blocks stay as code.
  }
}
