// **"Bring this in"**: one pipeline, whatever started it (`data-sources-research-2026-11-15.md` §11).
//
// A *piece* is what the person pointed at on a page: text with its formatting, an image, a link, a
// video or audio, or the page itself. Every way in (the selection chip, the right-click menu, the
// keyboard, Pick, a drop onto formicaria) makes one, and this module decides what it becomes in the
// note: **the content itself, or a link** for the heavy ones, always with where it came from. Shared
// by the browser add-on (which fetches and stores what is kept) and the app (which inserts it), and
// tested on its own.
import TurndownService from 'turndown';
import { textFragment } from './clip';

export type PieceKind = 'text' | 'image' | 'link' | 'video' | 'audio' | 'page';

export interface Piece {
  kind: PieceKind;
  /** The page it came from. */
  pageUrl: string;
  pageTitle: string;
  /** Text: the selection's HTML and its plain text. */
  html?: string;
  text?: string;
  /** Image, video, audio: the file's address; link: its target. */
  src?: string;
  /** Image: its description; link: its words. */
  alt?: string;
  /** Video, audio: where it was playing, in seconds. */
  time?: number;
}

/** Images up to this size are kept in the vault by default; above it, a link, with keeping a copy
 *  one press away. */
export const IMAGE_LIMIT = 10 * 1024 * 1024;

/** What a selection's HTML really is: one image, one video or audio, one link, or text. */
export function classify(html: string, text: string, page: { url: string; title: string }): Piece {
  const base = { pageUrl: page.url, pageTitle: page.title };
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  const b = doc.body;
  // What the selection *contains* decides, not its words: Firefox counts an image's description
  // as selected text, so a selected image would otherwise read as a passage.
  const words = html.trim() ? (b.textContent ?? '').trim() : text.trim();
  const media = b.querySelectorAll('img, video, audio');
  if (media.length === 1 && !words) {
    const m = media[0] as HTMLImageElement & HTMLMediaElement;
    const src = m.getAttribute('src') || m.querySelector('source')?.getAttribute('src') || '';
    const abs = absolute(src, page.url);
    if (m.tagName === 'IMG')
      return { ...base, kind: 'image', src: abs, alt: m.getAttribute('alt') ?? '' };
    return { ...base, kind: m.tagName === 'VIDEO' ? 'video' : 'audio', src: abs };
  }
  const links = b.querySelectorAll('a[href]');
  if (links.length === 1 && words && words === (links[0].textContent ?? '').trim()) {
    return {
      ...base,
      kind: 'link',
      src: absolute(links[0].getAttribute('href') ?? '', page.url),
      alt: words,
    };
  }
  if (!words && !html.trim()) return { ...base, kind: 'page' };
  return { ...base, kind: 'text', html, text: text.trim() || words };
}

export function absolute(src: string, page: string): string {
  try {
    return new URL(src, page).href;
  } catch {
    return src;
  }
}

/** Is this media played from pieces the browser assembles (a stream), with no one file to keep? */
export function streamed(src: string | undefined): boolean {
  return !src || /^(blob|mediasource|data):/.test(src);
}

const clean = (s: string) => s.replace(/[[\]]/g, '').replace(/\s+/g, ' ').trim();

/** The line every piece ends with: where it came from. */
export function source(p: Piece, passage?: string): string {
  const title = clean(p.pageTitle) || hostOf(p.pageUrl) || 'the page';
  const href = passage ? textFragment(p.pageUrl, passage) : p.pageUrl;
  return `— [${title}](${href})`;
}

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return '';
  }
}

let td: TurndownService | null = null;
/** HTML to Markdown, tables included (as Markdown tables), code as fenced blocks. */
export function markdown(html: string): string {
  if (!td) {
    td = new TurndownService({
      headingStyle: 'atx',
      codeBlockStyle: 'fenced',
      bulletListMarker: '-',
    });
    td.remove(['script', 'style', 'noscript']);
    const cell = (c: Element) =>
      (c.textContent ?? '').replace(/\s+/g, ' ').replace(/\|/g, '\\|').trim();
    td.addRule('table', {
      filter: 'table',
      replacement: (_content, node) => {
        const rows = Array.from((node as HTMLTableElement).querySelectorAll('tr')).map((r) =>
          Array.from(r.querySelectorAll('th, td')).map(cell),
        );
        if (!rows.length) return '';
        const width = Math.max(...rows.map((r) => r.length));
        const pad = (r: string[]) => [...r, ...Array(width - r.length).fill('')];
        const [head, ...rest] = rows.map(pad);
        const line = (r: string[]) => `| ${r.join(' | ')} |`;
        return `\n\n${[line(head), line(head.map(() => '---')), ...rest.map(line)].join('\n')}\n\n`;
      },
    });
  }
  return td.turndown(html).trim();
}

/** Text as it lands: a quote with a link back to the passage, or, for a table, code or a list, the
 *  block itself followed by its source. */
export function textPiece(p: Piece): string {
  const md = p.html ? markdown(p.html) : (p.text ?? '').trim();
  const block = /<(table|pre|ul|ol|h[1-6]|img|figure)\b/i.test(p.html ?? '');
  if (block) return `${md}\n\n${source(p, p.text)}`;
  const quoted = md
    .split('\n')
    .map((l) => (l.trim() ? `> ${l}` : '>'))
    .join('\n');
  return `${quoted}\n>\n> ${source(p, p.text)}`;
}

export function linkPiece(p: Piece): string {
  const words = clean(p.alt || p.src || '');
  return `[${words}](${p.src}) ${source(p)}`;
}

export function pagePiece(p: Piece): string {
  return `[${clean(p.pageTitle) || p.pageUrl}](${p.pageUrl})`;
}

/** An image kept in the vault. */
export function imageKept(p: Piece, asset: string): string {
  return `![${clean(p.alt ?? '')}](asset:${asset})\n\n${source(p)}`;
}

/** An image as a link only. */
export function imageLinked(p: Piece): string {
  return `[Image${p.alt ? `: ${clean(p.alt)}` : ''}](${p.src}) ${source(p)}`;
}

const clock = (s: number) => {
  const t = Math.max(0, Math.floor(s));
  const h = Math.floor(t / 3600);
  const m = Math.floor((t % 3600) / 60);
  const sec = String(t % 60).padStart(2, '0');
  return h ? `${h}:${String(m).padStart(2, '0')}:${sec}` : `${m}:${sec}`;
};

/** The page's address at the moment it was playing (YouTube and Vimeo understand `t`). */
export function atMoment(url: string, time?: number): string {
  if (!time || time < 1) return url;
  try {
    const u = new URL(url);
    const s = Math.floor(time);
    if (/(^|\.)youtube\.com$|(^|\.)youtu\.be$/.test(u.hostname)) u.searchParams.set('t', `${s}s`);
    else if (/(^|\.)vimeo\.com$/.test(u.hostname)) u.hash = `t=${s}s`;
    else return url;
    return u.href;
  } catch {
    return url;
  }
}

/** Video or audio as a link: the file when there is one, else the page at the moment it was playing. */
export function mediaLinked(p: Piece): string {
  const what = p.kind === 'audio' ? 'Audio' : 'Video';
  const at = p.time && p.time >= 1 ? ` at ${clock(p.time)}` : '';
  const title = clean(p.pageTitle) || what;
  if (streamed(p.src)) return `▶ [${title}${at}](${atMoment(p.pageUrl, p.time)})`;
  return `▶ [${what}: ${title}${at}](${p.src}) ${source(p)}`;
}

/** Video or audio kept in the vault. */
export function mediaKept(p: Piece, asset: string, name: string): string {
  return `▶ [${clean(name)}](asset:${asset})\n\n${source(p)}`;
}

/** A file name for something brought in, from its address. */
export function fileName(src: string, fallback: string): string {
  try {
    const last = decodeURIComponent(new URL(src).pathname.split('/').pop() ?? '');
    return last && /\.[a-z0-9]{2,5}$/i.test(last) ? last : fallback;
  } catch {
    return fallback;
  }
}
