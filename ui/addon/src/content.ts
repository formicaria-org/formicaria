// **The page side of formicaria's browser add-on** (`decisions.md` 2026-11-15). Injected only when
// the person presses *Keep* (`activeTab`), never into every page. It reads; it decides nothing and
// sends nothing — the background script does both.
//
// What it reads: the address and title, the structured data the page publishes for machines (JSON-LD,
// `<meta>`), the article text Mozilla's Readability finds, turned into Markdown by Turndown, what the
// person selected, and **a copy of the page as it is now** — the document with a `<base>` so its
// relative links still resolve, and its scripts left out: it is a copy to read, not a program to run.
import { Readability } from '@mozilla/readability';
import TurndownService from 'turndown';
import type { PageFacts } from '../../src/lib/web/clip';

function meta(): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const el of Array.from(document.querySelectorAll('meta'))) {
    const k = (el.getAttribute('name') ?? el.getAttribute('property') ?? '').trim().toLowerCase();
    const v = el.getAttribute('content');
    if (!k || v == null) continue;
    (out[k] ??= []).push(v);
  }
  return out;
}

function jsonld(): unknown[] {
  const out: unknown[] = [];
  for (const el of Array.from(document.querySelectorAll('script[type="application/ld+json"]'))) {
    try {
      out.push(JSON.parse(el.textContent ?? ''));
    } catch {
      // A page's broken block is the page's; the rest still count.
    }
  }
  return out;
}

function article(): PageFacts['article'] {
  try {
    const clone = document.cloneNode(true) as Document;
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

function snapshot(): string {
  const doc = document.documentElement.cloneNode(true) as HTMLElement;
  // Scripts are left out — a copy to read, not a program to run — **except the page's structured
  // data** (JSON-LD), which is data: the recipe reader reads it from this copy.
  for (const s of Array.from(doc.querySelectorAll('script'))) {
    if ((s.getAttribute('type') ?? '').toLowerCase() !== 'application/ld+json') s.remove();
  }
  const head = doc.querySelector('head');
  if (head && !head.querySelector('base')) {
    const base = document.createElement('base');
    base.href = location.href;
    head.prepend(base);
  }
  return `<!doctype html>\n${doc.outerHTML}`;
}

/** What the background script asks for: everything it needs, read once. Called by the background
 *  right after injecting this file (`formicaria_content.collect()`), so no message can race it. */
export function collect(): PageFacts & { snapshot: string } {
  return {
    url: location.href,
    title: document.title,
    jsonld: jsonld(),
    meta: meta(),
    article: article(),
    selection: window.getSelection()?.toString() ?? '',
    snapshot: snapshot(),
  };
}
