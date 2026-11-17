// **A saved page is one self-contained file** (`decisions.md` 2026-11-17, *real artifacts*; the owner:
// *"I like to maintain real artifacts eventually, from any website"*). The copy `read.ts` makes still
// pointed at the site for its pictures, styles and fonts; this puts them inside the file, so the copy
// opens the same offline, years later. **Video and audio are never put inside** (one copy of heavy
// things: they are kept, if wanted, as files of their own; here they keep their address).
//
// Shared by every place a page is saved: the computer's add-on fetches with its own rights, the phone
// through the app's core (`web_resource`). Pure apart from `get`.

/** One fetched resource: its type and bytes, as base64. `null`: not available (left as an address). */
export type Got = { type: string; base64: string } | null;
export type Getter = (url: string) => Promise<Got>;

export interface Limits {
  /** The most one resource may weigh (bytes); a heavier one keeps its address. */
  each: number;
  /** The most all of them together may weigh; past it, the rest keep their addresses. */
  total: number;
}
export const LIMITS: Limits = { each: 5 * 1024 * 1024, total: 25 * 1024 * 1024 };

const sizeOf = (b64: string) => Math.floor((b64.length * 3) / 4);

function absolute(url: string, base: string): string | null {
  const u = url.trim();
  if (!u || u.startsWith('data:') || u.startsWith('#') || u.startsWith('javascript:')) return null;
  try {
    const a = new URL(u, base);
    return a.protocol === 'http:' || a.protocol === 'https:' ? a.href : null;
  } catch {
    return null;
  }
}

/** The best candidate of a `srcset`: the widest (or densest) one. */
export function bestOf(srcset: string): string | null {
  let best: { url: string; w: number } | null = null;
  for (const part of srcset.split(/,\s+(?=\S)/)) {
    const [url, d] = part.trim().split(/\s+/);
    if (!url) continue;
    const n = d ? parseFloat(d) * (d.endsWith('x') ? 1000 : 1) : 1;
    if (!best || n > best.w) best = { url, w: Number.isFinite(n) ? n : 1 };
  }
  return best?.url ?? null;
}

function decode(b64: string): string {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return new TextDecoder().decode(bytes);
}

/** Put a page's pictures, styles and fonts inside it. `html` is the copy `read.ts` made, `base` the
 *  page's address. Answers the new HTML and what was put in. */
export async function inlinePage(
  html: string,
  base: string,
  get: Getter,
  limits: Limits = LIMITS,
): Promise<{ html: string; inlined: number; kept: number }> {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const pageBase = doc.querySelector('base')?.getAttribute('href') || base;
  let total = 0;
  let inlined = 0;
  let kept = 0;
  const cache = new Map<string, Promise<string | null>>();

  /** A resource as a `data:` address, within the limits; null keeps its address. */
  const dataUrl = (abs: string): Promise<string | null> => {
    let p = cache.get(abs);
    if (!p) {
      p = (async () => {
        if (total >= limits.total) return null;
        const g = await get(abs).catch(() => null);
        if (!g) return null;
        const n = sizeOf(g.base64);
        if (n > limits.each || total + n > limits.total) return null;
        total += n;
        return `data:${g.type || 'application/octet-stream'};base64,${g.base64}`;
      })();
      cache.set(abs, p);
    }
    return p;
  };

  /** Every `url(…)` in a style sheet, resolved against `from` and put inside. */
  const css = async (text: string, from: string, depth = 0): Promise<string> => {
    // `@import` first: its sheet's text, inlined in turn (two levels at most).
    const imports = [
      ...text.matchAll(/@import\s+(?:url\()?\s*['"]?([^'")\s;]+)['"]?\s*\)?[^;]*;/g),
    ];
    for (const m of imports) {
      const abs = absolute(m[1], from);
      let swap = '';
      if (abs && depth < 2) {
        const g = await get(abs).catch(() => null);
        if (g && sizeOf(g.base64) <= limits.each)
          swap = await css(decode(g.base64), abs, depth + 1);
      }
      text = text.replace(m[0], swap || m[0]);
    }
    const urls = [...text.matchAll(/url\(\s*(['"]?)([^'")]+)\1\s*\)/g)];
    for (const m of urls) {
      const abs = absolute(m[2], from);
      if (!abs) continue;
      const d = await dataUrl(abs);
      if (d) {
        text = text.split(m[0]).join(`url("${d}")`);
        inlined++;
      } else {
        text = text.split(m[0]).join(`url("${abs}")`);
        kept++;
      }
    }
    return text;
  };

  // Pictures: the best of a `srcset`, a lazy loader's real address, then put inside.
  for (const img of Array.from(doc.querySelectorAll('img'))) {
    const lazy = img.getAttribute('data-src') || img.getAttribute('data-lazy-src');
    const set = img.getAttribute('srcset') || img.getAttribute('data-srcset');
    const pick = (set && bestOf(set)) || lazy || img.getAttribute('src') || '';
    img.removeAttribute('srcset');
    img.removeAttribute('data-srcset');
    img.removeAttribute('loading');
    const abs = absolute(pick, pageBase);
    if (!abs) continue;
    const d = await dataUrl(abs);
    img.setAttribute('src', d ?? abs);
    if (d) inlined++;
    else kept++;
  }
  // `<picture>`'s alternatives go: the `<img>` inside holds the one picture kept.
  for (const s of Array.from(doc.querySelectorAll('picture source'))) s.remove();
  // Icons and other pictures a page names in `<link>`.
  for (const l of Array.from(doc.querySelectorAll('link[rel~="icon"]'))) {
    const abs = absolute(l.getAttribute('href') ?? '', pageBase);
    const d = abs && (await dataUrl(abs));
    if (d) {
      l.setAttribute('href', d);
      inlined++;
    }
  }
  // Style sheets: their text, with what they name inside, in place of the link.
  for (const l of Array.from(doc.querySelectorAll('link[rel~="stylesheet"]'))) {
    const abs = absolute(l.getAttribute('href') ?? '', pageBase);
    if (!abs) continue;
    const g = await get(abs).catch(() => null);
    if (!g || sizeOf(g.base64) > limits.each) {
      l.setAttribute('href', abs);
      kept++;
      continue;
    }
    const style = doc.createElement('style');
    const media = l.getAttribute('media');
    if (media) style.setAttribute('media', media);
    style.textContent = await css(decode(g.base64), abs);
    l.replaceWith(style);
    inlined++;
  }
  for (const s of Array.from(doc.querySelectorAll('style')))
    if (s.textContent?.includes('url(')) s.textContent = await css(s.textContent, pageBase);
  for (const el of Array.from(doc.querySelectorAll('[style]')).filter((e) =>
    (e.getAttribute('style') ?? '').includes('url('),
  ))
    el.setAttribute('style', await css(el.getAttribute('style') ?? '', pageBase));
  // Video and audio keep their (absolute) addresses: never inside.
  for (const m of Array.from(
    doc.querySelectorAll('video[src], audio[src], source[src], video[poster]'),
  )) {
    for (const a of ['src', 'poster']) {
      const v = m.getAttribute(a);
      const abs = v && absolute(v, pageBase);
      if (abs) m.setAttribute(a, abs);
    }
  }
  // Prefetch and preload hints would reach out to the site when the copy is opened.
  for (const l of Array.from(
    doc.querySelectorAll(
      'link[rel~="preload"], link[rel~="prefetch"], link[rel~="preconnect"], link[rel~="dns-prefetch"], link[rel~="modulepreload"]',
    ),
  ))
    l.remove();
  return { html: `<!doctype html>\n${doc.documentElement.outerHTML}`, inlined, kept };
}

/** Bytes as base64, in slices (a large file overflows one `String.fromCharCode` call). */
export function toBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000)
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

/** A getter with plain `fetch` (the add-on's rights reach any site), at most `n` at a time. */
export function fetchGetter(n = 6, init: RequestInit = {}): Getter {
  let running = 0;
  const waiting: (() => void)[] = [];
  return async (url) => {
    if (running >= n) await new Promise<void>((r) => waiting.push(r));
    running++;
    try {
      const r = await fetch(url, { cache: 'force-cache', ...init });
      if (!r.ok) return null;
      const type = (r.headers.get('content-type') ?? '').split(';')[0].trim();
      return { type, base64: toBase64(new Uint8Array(await r.arrayBuffer())) };
    } catch {
      return null;
    } finally {
      running--;
      waiting.shift()?.();
    }
  };
}
