// **Reading a page for a note** (`decisions.md` 2026-11-15, *Save page*; 2026-11-17, the phone too):
// what the page says of itself (JSON-LD, `<meta>`), the article Mozilla's Readability finds, turned into
// Markdown by Turndown, and a copy of the page without its scripts. One reading for every place a page
// is saved: the computer's add-on (the live page) and the phone (the page's HTML, from the shell).
import { Readability } from '@mozilla/readability';
import TurndownService from 'turndown';
import type { PageFacts } from './clip';

function meta(doc: Document): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const el of Array.from(doc.querySelectorAll('meta'))) {
    const k = (el.getAttribute('name') ?? el.getAttribute('property') ?? '').trim().toLowerCase();
    const v = el.getAttribute('content');
    if (!k || v == null) continue;
    (out[k] ??= []).push(v);
  }
  return out;
}

function jsonld(doc: Document): unknown[] {
  const out: unknown[] = [];
  for (const el of Array.from(doc.querySelectorAll('script[type="application/ld+json"]'))) {
    try {
      out.push(JSON.parse(el.textContent ?? ''));
    } catch {
      // A page's broken block is the page's; the rest still count.
    }
  }
  return out;
}

function article(doc: Document): PageFacts['article'] {
  try {
    const clone = doc.cloneNode(true) as Document;
    const a = new Readability(clone).parse();
    if (!a) return undefined;
    const td = new TurndownService({
      headingStyle: 'atx',
      codeBlockStyle: 'fenced',
      bulletListMarker: '-',
    });
    return {
      title: a.title ?? undefined,
      byline: a.byline ?? undefined,
      siteName: a.siteName ?? undefined,
      published: a.publishedTime ?? undefined,
      excerpt: a.excerpt ?? undefined,
      markdown: a.content ? td.turndown(a.content) : undefined,
    };
  } catch {
    return undefined;
  }
}

function snapshot(doc: Document, url: string): string {
  const root = doc.documentElement.cloneNode(true) as HTMLElement;
  // Scripts are left out — a copy to read, not a program to run — **except the page's structured
  // data** (JSON-LD), which is data: the recipe reader reads it from this copy.
  for (const s of Array.from(root.querySelectorAll('script'))) {
    if ((s.getAttribute('type') ?? '').toLowerCase() !== 'application/ld+json') s.remove();
  }
  const head = root.querySelector('head');
  if (head && !head.querySelector('base')) {
    const base = doc.createElement('base');
    base.href = url;
    head.prepend(base);
  }
  return `<!doctype html>\n${root.outerHTML}`;
}

/** Everything saving a page needs, read once from `doc` (the page at `url`). */
export function readPage(
  doc: Document,
  url: string,
  selection = '',
): PageFacts & { snapshot: string } {
  return {
    url,
    title: doc.title,
    jsonld: jsonld(doc),
    meta: meta(doc),
    article: article(doc),
    selection,
    canonical: (doc.querySelector('link[rel~="canonical"]') as HTMLLinkElement | null)?.href,
    snapshot: snapshot(doc, url),
  };
}

/** A page's HTML, as a document to read (the phone: the shell hands the HTML over). */
export function parsePage(html: string, url: string): Document {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  if (!doc.querySelector('base')) {
    const base = doc.createElement('base');
    base.href = url;
    doc.head?.prepend(base);
  }
  return doc;
}
