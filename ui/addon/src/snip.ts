// **Snip** (owner, 2026-11-16: *"what if the user could select an area like a snipping tool and
// everything in it can be ported appropriately?"*). The person drags a rectangle over the page;
// everything mostly inside it comes into the note in front, each as itself (text, tables, code,
// images, links, video), and what has no text behind it (a canvas, a chart drawn by the page) as a
// picture of the area. The result is said **on the area itself**, with Undo and *As a picture
// instead*, where the eyes and the pointer already are.
//
// Started by the ant in the address bar, Alt+Shift+S, or Ctrl+Shift+drag on the page. Part of the
// page script (`select.ts`), so it is ready on every page without injecting anything; it does nothing
// until started, draws in a closed shadow root, and reads only the area the person drew.
declare const browser: { runtime: { sendMessage<T>(m: unknown): Promise<T> } };

export interface Block {
  kind: 'text' | 'block' | 'image' | 'video' | 'audio' | 'picture';
  html?: string;
  text?: string;
  src?: string;
  alt?: string;
  time?: number;
  rect?: { x: number; y: number; width: number; height: number };
}
interface Result {
  ok: boolean;
  title?: string;
  error?: string;
  note?: string;
  inserted?: string;
  alt?: { label: string; token: string };
  open?: string;
  what?: string;
  /** A sentence of its own, instead of *Into "…"*. */
  say?: string;
}

const CANDIDATES =
  'p,li,ul,ol,dl,h1,h2,h3,h4,h5,h6,blockquote,pre,table,figure,figcaption,img,video,audio,canvas,svg,div,section,article,span';

let active: (() => void) | null = null;

function overlap(r: DOMRect, a: DOMRect): number {
  const w = Math.min(r.right, a.right) - Math.max(r.left, a.left);
  const h = Math.min(r.bottom, a.bottom) - Math.max(r.top, a.top);
  if (w <= 0 || h <= 0 || r.width * r.height === 0) return 0;
  return (w * h) / (r.width * r.height);
}

/** Where an element's contents actually are on screen (its text, its images), not its box. */
function contentBox(el: Element): DOMRect {
  const r = document.createRange();
  r.selectNodeContents(el);
  const b = r.getBoundingClientRect();
  return b.width && b.height ? b : el.getBoundingClientRect();
}

/** Does this element carry text of its own (not only through its children)? */
function ownText(el: Element): boolean {
  return Array.from(el.childNodes).some((n) => n.nodeType === 3 && (n.textContent ?? '').trim());
}

/** What is mostly inside `area`, outermost first, in reading order. */
export function blocksIn(area: DOMRect): Block[] {
  const kept: Element[] = [];
  for (const el of Array.from(document.body.querySelectorAll(CANDIDATES))) {
    const t = el.tagName;
    if ((t === 'DIV' || t === 'SECTION' || t === 'ARTICLE' || t === 'SPAN') && !ownText(el))
      continue;
    // A paragraph's box spans its column even when its text is short: what counts is where its
    // contents are (an image, a canvas, a video: the element itself).
    const r = /^(IMG|VIDEO|AUDIO|CANVAS|SVG|svg)$/.test(t)
      ? el.getBoundingClientRect()
      : contentBox(el);
    if (!r.width || !r.height || overlap(r, area) < 0.5) continue;
    const style = getComputedStyle(el);
    if (style.visibility === 'hidden' || style.display === 'none') continue;
    if (kept.some((k) => k.contains(el))) continue;
    kept.push(el);
  }
  return toBlocks(kept);
}

/** Elements, outermost and in reading order, as the blocks the background turns into the note. */
function toBlocks(kept: Element[]): Block[] {
  const out: Block[] = [];
  const box = (el: Element) => {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top, width: r.width, height: r.height };
  };
  for (const el of kept) {
    const t = el.tagName;
    if (t === 'IMG') {
      const i = el as HTMLImageElement;
      out.push({ kind: 'image', src: i.currentSrc || i.src, alt: i.alt });
    } else if (t === 'VIDEO' || t === 'AUDIO') {
      const m = el as HTMLMediaElement;
      out.push({
        kind: t === 'VIDEO' ? 'video' : 'audio',
        src: m.currentSrc || m.src,
        time: m.currentTime,
      });
    } else if (t === 'CANVAS' || t === 'svg' || t === 'SVG') {
      out.push({ kind: 'picture', rect: box(el) });
    } else if (t === 'FIGURE') {
      for (const i of Array.from(el.querySelectorAll('img')))
        out.push({ kind: 'image', src: i.currentSrc || i.src, alt: i.alt });
      const cap = el.querySelector('figcaption');
      if (cap?.textContent?.trim())
        out.push({ kind: 'text', html: cap.innerHTML, text: cap.textContent.trim() });
    } else if (!(el.textContent ?? '').trim() && el.querySelector('img, video, audio')) {
      // A wrapper with no words of its own (a paragraph holding an image): its media, kept as media.
      for (const m of Array.from(el.querySelectorAll('img, video, audio'))) {
        if (m.tagName === 'IMG') {
          const i = m as HTMLImageElement;
          out.push({ kind: 'image', src: i.currentSrc || i.src, alt: i.alt });
        } else {
          const v = m as HTMLMediaElement;
          out.push({
            kind: m.tagName === 'VIDEO' ? 'video' : 'audio',
            src: v.currentSrc || v.src,
            time: v.currentTime,
          });
        }
      }
    } else if (t === 'TABLE' || t === 'PRE' || t === 'UL' || t === 'OL' || t === 'DL') {
      out.push({ kind: 'block', html: el.outerHTML, text: (el.textContent ?? '').trim() });
    } else {
      out.push({ kind: 'text', html: el.outerHTML, text: (el.textContent ?? '').trim() });
    }
  }
  return out;
}

/** The outermost of `els`, in document order. */
function outermost(els: Element[]): Element[] {
  const kept: Element[] = [];
  for (const el of els) if (!kept.some((k) => k.contains(el))) kept.push(el);
  return kept;
}

/** **Suggestions** (research §11.4): what a page plainly offers, one press each. */
/** A page whose point is a video or a sound: its own player, or a site known for them. */
export function mediaPage(): boolean {
  if (document.querySelector('video, audio')) return true;
  return /(^|\.)(youtube\.com|youtu\.be|vimeo\.com|dailymotion\.com|soundcloud\.com|bandcamp\.com|twitch\.tv|tiktok\.com|instagram\.com)$/.test(
    location.hostname,
  );
}

export function suggestions(): {
  label: string;
  blocks?: Block[];
  save?: boolean;
  media?: boolean;
}[] {
  const out: { label: string; blocks?: Block[]; save?: boolean; media?: boolean }[] = [];
  const has = (q: string) => !!document.querySelector(q);
  // **A video or a sound is what such a page offers first** (the owner, 2026-11-19, on a YouTube
  // page: *"it says only 12 images and no video"*): Save media, as the ant's panel does, and not the
  // page's thumbnails.
  const media = mediaPage();
  if (media) out.push({ label: 'Save media…', media: true });
  const ld = Array.from(document.querySelectorAll('script[type="application/ld+json"]'))
    .map((s) => s.textContent ?? '')
    .join(' ');
  if (/"@type"\s*:\s*(\[[^\]]*)?"Recipe"/.test(ld))
    out.push({ label: 'Save as a recipe', save: true });
  if (has('meta[name="citation_title"]')) out.push({ label: 'Save as a paper', save: true });
  // The main text: the page's own <article>/<main>, else the element holding the most paragraph text.
  let main = document.querySelector('article, main, [role="main"]');
  if (!main) {
    let best = 0;
    for (const p of Array.from(document.querySelectorAll('p'))) {
      const parent = p.parentElement;
      if (!parent) continue;
      const len = Array.from(parent.children)
        .filter((c) => c.tagName === 'P')
        .reduce((n, c) => n + (c.textContent ?? '').length, 0);
      if (len > best) {
        best = len;
        main = parent;
      }
    }
  }
  if (main && (main.textContent ?? '').trim().length > 200) {
    const parts = outermost(
      Array.from(main.querySelectorAll('h1,h2,h3,h4,h5,h6,p,ul,ol,blockquote,pre,table,figure')),
    );
    if (parts.length) out.push({ label: 'The main text', blocks: toBlocks(parts) });
  }
  const images = Array.from(document.images).filter(
    (i) => i.naturalWidth >= 120 && i.naturalHeight >= 120,
  );
  if (images.length && !media)
    out.push({
      label: images.length === 1 ? 'The image' : `The ${images.length} images`,
      blocks: toBlocks(images),
    });
  const tables = Array.from(document.querySelectorAll('table')).filter((t) => t.rows.length > 1);
  if (tables.length)
    out.push({
      label: tables.length === 1 ? 'The table' : `The ${tables.length} tables`,
      blocks: toBlocks(tables),
    });
  return out.slice(0, 4);
}

const STYLE = `
  :host { all: initial; }
  .track { flex: 1 0 140px; height: 6px; border-radius: 3px; background: #3a3a46; overflow: hidden; }
  .fill { height: 100%; width: 0; background: #dc2626; transition: width .25s; }
  .fill.busy { width: 30%; animation: fm-slide 1.2s ease-in-out infinite; }
  @keyframes fm-slide { from { margin-left: -30%; } to { margin-left: 100%; } }
  .veil { position: fixed; inset: 0; z-index: 2147483646; cursor: crosshair; background: rgba(0,0,0,.12); }
  .area { position: fixed; z-index: 2147483647; border: 2px solid #dc2626; background: rgba(220,38,38,.08);
    pointer-events: none; display: none; }
  .bar { position: fixed; z-index: 2147483647; left: 50%; top: 8px; transform: translateX(-50%);
    display: flex; gap: 6px; align-items: center; font: 13px/1.3 system-ui, sans-serif; color: #fff;
    background: #1d1d22; padding: 5px 6px 5px 12px; border-radius: 999px; box-shadow: 0 2px 8px rgba(0,0,0,.25); }
  .bubble { position: fixed; z-index: 2147483647; display: flex; flex-wrap: wrap; gap: 6px; align-items: center;
    max-width: 420px; font: 13px/1.3 system-ui, sans-serif; color: #fff; background: #1d1d22;
    padding: 6px 6px 6px 12px; border-radius: 12px; box-shadow: 0 2px 10px rgba(0,0,0,.3); }
  button { all: initial; cursor: pointer; font: 13px/1 system-ui, sans-serif; color: #fff; background: #3a3a46;
    padding: 6px 10px; border-radius: 999px; }
  button:hover { background: #dc2626; }
  .suggest { background: #5b2323; }
  .list { position: fixed; z-index: 2147483647; left: 50%; top: 48px; transform: translateX(-50%);
    width: min(420px, 90vw); max-height: 50vh; overflow: auto; font: 13px/1.35 system-ui, sans-serif;
    color: #1d1d22; background: #fff; border-radius: 10px; box-shadow: 0 4px 16px rgba(0,0,0,.25); padding: 6px; }
  .list a { display: block; padding: 6px 8px; border-radius: 6px; color: inherit; text-decoration: none; cursor: pointer; }
  .list a:hover { background: #f2f1ee; }
  .list small { display: block; color: #5b5b66; overflow-wrap: anywhere; }
`;

function shadowHost(tag: string) {
  const host = document.createElement(tag);
  const root = host.attachShadow({ mode: 'closed' });
  root.innerHTML = `<style>${STYLE}</style>`;
  document.documentElement.append(host);
  return { host, root };
}

const el = (tag: string, cls: string, text = '') => {
  const e = document.createElement(tag);
  e.className = cls;
  e.textContent = text;
  return e;
};

/** **What happened, where it happened**: a bubble on the area, with Undo and the alternative. */
export function bubble(at: { x: number; y: number }, r: Result) {
  const { host, root } = shadowHost('formicaria-bubble');
  const b = el('div', 'bubble');
  const say = el(
    'span',
    '',
    r.ok
      ? (r.say ?? `Brought into the note “${r.title ?? 'your note'}”`)
      : `Not brought in: ${r.error ?? ''}`,
  );
  b.append(say);
  const close = () => host.remove();
  if (r.ok && r.alt) {
    const alt = r.alt;
    const a = el('button', '', alt.label);
    a.onclick = async () => {
      a.remove();
      say.textContent = 'Changing it…';
      const done = await browser.runtime.sendMessage<Result>({
        type: 'alt',
        token: alt.token,
        note: r.note,
        from: r.inserted,
      });
      say.textContent = done?.ok ? 'Changed.' : `Not changed: ${done?.error ?? ''}`;
      setTimeout(close, 2500);
    };
    b.append(a);
  }
  if (r.ok && r.open) {
    const o = el('button', '', 'Open');
    o.onclick = () => {
      void browser.runtime.sendMessage({ type: 'open', id: r.open });
      close();
    };
    b.append(o);
    const link = el('button', '', 'Just the link instead');
    link.onclick = async () => {
      link.remove();
      const done = await browser.runtime.sendMessage<Result>({
        type: 'link-instead-here',
        id: r.open,
      });
      say.textContent = done?.ok
        ? 'Just the link now, as a bookmark note.'
        : `Not changed: ${done?.error ?? ''}`;
      setTimeout(close, 2500);
    };
    b.append(link);
  }
  // **Where it went, one press away** (the owner, 2026-11-19: *"I have no idea where and what was
  // saved"*): the note it went into, opened in its tab.
  if (r.ok && r.note && !r.open) {
    const o = el('button', '', 'Open the note');
    o.onclick = () => {
      void browser.runtime.sendMessage({ type: 'open', id: r.note });
      close();
    };
    b.append(o);
  }
  if (r.ok && r.inserted && r.note) {
    const u = el('button', '', 'Undo');
    u.onclick = async () => {
      await browser.runtime.sendMessage({ type: 'undo', note: r.note, from: r.inserted });
      close();
    };
    b.append(u);
  }
  root.append(b);
  const w = Math.min(420, innerWidth - 16);
  b.style.left = `${Math.max(8, Math.min(at.x, innerWidth - w - 8))}px`;
  b.style.top = `${Math.max(8, Math.min(at.y, innerHeight - 60))}px`;
  let hovered = false;
  b.onmouseenter = () => (hovered = true);
  b.onmouseleave = () => (hovered = false);
  const tick = () => (hovered ? setTimeout(tick, 2000) : close());
  // Long enough to read and press (it was 9 s, and went unseen); a close button ends it sooner.
  const x = el('button', '', '×');
  x.setAttribute('aria-label', 'Close');
  x.onclick = close;
  b.append(x);
  setTimeout(tick, 20000);
}

/** **Work under way, seen** (the owner, 2026-11-19: *"if it takes time … a visible bar filling in,
 *  to suggest it is going on, or a clear error"*): a bubble with a bar that fills as the background
 *  says how far it is (`progress`), or moves while it cannot say; then the result's own bubble. Shown
 *  only after a moment, so a quick answer shows only its result, and an area's picture, taken by the
 *  browser first, never has it. Quiet for 90 s, it says so. */
let working: { step(done: number, total: number): void } | null = null;
export async function withProgress(
  at: { x: number; y: number },
  text: string,
  run: () => Promise<Result | undefined>,
): Promise<void> {
  const { host, root } = shadowHost('formicaria-working');
  const b = el('div', 'bubble');
  const say = el('span', '', text);
  const track = el('div', 'track');
  const fill = el('div', 'fill busy');
  track.append(fill);
  b.append(say, track);
  b.setAttribute('role', 'status');
  const w = Math.min(420, innerWidth - 16);
  b.style.left = `${Math.max(8, Math.min(at.x, innerWidth - w - 8))}px`;
  b.style.top = `${Math.max(8, Math.min(at.y, innerHeight - 60))}px`;
  const show = setTimeout(() => root.append(b), 400);
  let quiet: ReturnType<typeof setTimeout>;
  const hush = () => {
    clearTimeout(quiet);
    quiet = setTimeout(
      () => (say.textContent = `${text} (no word from formicaria for 90 s)`),
      90000,
    );
  };
  hush();
  working = {
    step(done, total) {
      hush();
      if (total > 0) {
        fill.classList.remove('busy');
        fill.style.width = `${Math.round((100 * Math.min(done, total)) / total)}%`;
        say.textContent = `${text} ${Math.min(done, total)} of ${total}`;
      }
    },
  };
  let r: Result | undefined;
  try {
    r = await run();
  } catch (e) {
    r = { ok: false, error: String(e) };
  }
  clearTimeout(show);
  clearTimeout(quiet!);
  working = null;
  host.remove();
  bubble(at, r ?? { ok: false, error: 'formicaria did not answer' });
}

/** The background's word on work under way (`{type: 'progress', done, total}`). */
export function progress(done: number, total: number) {
  working?.step(done, total);
}

/** Start a snip; `from` when it began with a Ctrl+Shift press on the page (the drag goes on). */
export function startSnip(from?: { x: number; y: number }) {
  if (active) return;
  const { host, root } = shadowHost('formicaria-snip');
  const veil = el('div', 'veil');
  const area = el('div', 'area');
  const bar = el('div', 'bar');
  const label = el('span', '', 'Drag over what to bring in');
  const save = el('button', '', 'Save the whole page');
  const cancel = el('button', '', 'Cancel');
  bar.append(label);
  // What the page plainly offers, one press each, before drawing anything.
  for (const sg of suggestions()) {
    const b = el('button', 'suggest', sg.label);
    b.onclick = async () => {
      stop();
      if (sg.media) {
        await browser.runtime.sendMessage({ type: 'save-media' });
        return;
      }
      const at = { x: innerWidth / 2 - 160, y: 12 };
      if (sg.save) {
        await withProgress(at, 'Saving the page…', () =>
          browser.runtime.sendMessage<Result>({ type: 'save-page' }),
        );
        return;
      }
      await withProgress(at, `Bringing ${sg.label.replace(/^The /, 'the ')} in…`, () =>
        browser.runtime.sendMessage<Result>({
          type: 'snip',
          blocks: sg.blocks ?? [],
          area: null,
          dpr: devicePixelRatio,
        }),
      );
    };
    bar.append(b);
  }
  bar.append(save, cancel);
  root.append(veil, area, bar);

  void browser.runtime
    .sendMessage<{
      front?: string;
      assistant?: string;
      notes?: { id: string; title: string; context: string }[];
    }>({ type: 'snip-info' })
    .then((info) => {
      label.textContent = info?.front
        ? `Drag over what to bring into “${info.front}”`
        : 'Drag over what to bring into a new note';
      // **Ask the assistant what matters here**: only when one is running and a note is in front.
      if (info?.assistant && info.front) {
        const main = suggestions().find((sg) => sg.label === 'The main text');
        const asking = el('button', 'suggest', `Ask ${info.assistant} what matters here`);
        asking.onclick = async () => {
          stop();
          const blocks: Block[] = main?.blocks ?? [
            { kind: 'text', html: '', text: (document.body.innerText ?? '').slice(0, 12000) },
          ];
          const r = await browser.runtime.sendMessage<Result>({ type: 'ask-assistant', blocks });
          bubble(
            { x: innerWidth / 2 - 160, y: 12 },
            r?.ok
              ? {
                  ok: true,
                  say: `${info.assistant} is reading it; its suggestions will come as proposals to accept`,
                }
              : (r ?? { ok: false, error: 'no answer' }),
          );
        };
        bar.insertBefore(asking, save);
      }
      const notes = info?.notes ?? [];
      if (!notes.length) return;
      const inNotes = el(
        'button',
        '',
        notes.length === 1 ? 'In one of your notes' : `In ${notes.length} of your notes`,
      );
      inNotes.onclick = () => {
        const list = el('div', 'list');
        for (const n of notes) {
          const a = el('a', '');
          a.append(el('span', '', n.title || 'Untitled'), el('small', '', n.context));
          a.onclick = () => {
            void browser.runtime.sendMessage({ type: 'open', id: n.id });
            stop();
          };
          list.append(a);
        }
        root.append(list);
        inNotes.remove();
      };
      bar.insertBefore(inNotes, save);
    });

  let start: { x: number; y: number } | null = from ?? null;
  const rect = (a: { x: number; y: number }, b: { x: number; y: number }) =>
    new DOMRect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(a.x - b.x), Math.abs(a.y - b.y));
  const draw = (r: DOMRect) => {
    Object.assign(area.style, {
      display: 'block',
      left: `${r.left}px`,
      top: `${r.top}px`,
      width: `${r.width}px`,
      height: `${r.height}px`,
    });
  };
  const down = (e: MouseEvent) => {
    if (e.button !== 0 || e.composedPath().some((n) => n === bar)) return;
    e.preventDefault();
    start = { x: e.clientX, y: e.clientY };
  };
  const move = (e: MouseEvent) => {
    if (!start) return;
    e.preventDefault();
    draw(rect(start, { x: e.clientX, y: e.clientY }));
  };
  const up = (e: MouseEvent) => {
    if (!start) return;
    e.preventDefault();
    const r = rect(start, { x: e.clientX, y: e.clientY });
    start = null;
    if (r.width < 8 || r.height < 8) return;
    stop();
    void bring(r);
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Escape') stop();
  };
  const stop = () => {
    removeEventListener('mousedown', down, true);
    removeEventListener('mousemove', move, true);
    removeEventListener('mouseup', up, true);
    removeEventListener('keydown', key, true);
    host.remove();
    active = null;
  };
  save.onclick = async () => {
    stop();
    await withProgress({ x: innerWidth / 2 - 160, y: 12 }, 'Saving the page…', () =>
      browser.runtime.sendMessage<Result>({ type: 'save-page' }),
    );
  };
  cancel.onclick = () => stop();
  addEventListener('mousedown', down, true);
  addEventListener('mousemove', move, true);
  addEventListener('mouseup', up, true);
  addEventListener('keydown', key, true);
  active = stop;
}

async function bring(r: DOMRect) {
  const blocks = blocksIn(r);
  // The picture is taken by the browser from what is on screen: wait for the veil to be gone.
  await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
  await withProgress({ x: r.left, y: r.bottom + 8 }, 'Bringing it in…', () =>
    browser.runtime.sendMessage<Result>({
      type: 'snip',
      blocks,
      area: { x: r.left, y: r.top, width: r.width, height: r.height },
      dpr: devicePixelRatio,
    }),
  );
}

export function snipping() {
  return !!active;
}
