// **formicaria's browser add-on: keeping a page** (`decisions.md` 2026-11-15).
//
// One flow, from the toolbar button or the page's right-click menu: read the page (`content.ts`),
// ask formicaria to read a recipe out of it if it publishes one (`recipe_from_page`, no fetch), build
// the note with the same code the app uses (`src/lib/web/clip.ts`), and keep it with a copy of the
// page (`clip_save`). The key and formicaria's address come from Firefox's managed storage, which
// formicaria writes into its browser's policies; no other add-on can read them.
import { inlinePage, fetchGetter } from '../../src/lib/web/inline';
import type { WebRecipe } from '../../src/lib/types';
import { buildClip, kindOf, type PageFacts } from '../../src/lib/web/clip';
import { choose, onCreated, type Pending, type Where } from './downloads';
import {
  IMAGE_LIMIT,
  classify,
  fileName,
  formulaPiece,
  imageKept,
  imageLinked,
  linkPiece,
  mediaKept,
  mediaLinked,
  pagePiece,
  streamed,
  markdown,
  source,
  textPiece,
  type Piece,
} from '../../src/lib/web/bring';
import type { Block } from './snip';
import { sectionOf } from '../../src/lib/sections';
import { addressUrl } from '../../src/feature/route';
// Exported for the end-to-end test's copy of the add-on, which answers the question itself.
export { choose };

/* The slice of the WebExtension API this uses. */
declare const browser: {
  storage: { managed: { get(): Promise<{ key?: string; server?: string }> } };
  tabs: {
    executeScript(tab: number, d: { file?: string; code?: string }): Promise<unknown[]>;
    sendMessage<T>(tab: number, m: unknown): Promise<T>;
    query(q: {
      url?: string;
      active?: boolean;
      currentWindow?: boolean;
    }): Promise<{ id?: number; windowId?: number; url?: string }[]>;
    update(tab: number, d: { url?: string; active?: boolean }): Promise<unknown>;
    create(d: { url: string; index?: number }): Promise<unknown>;
    get(tab: number): Promise<{
      id?: number;
      windowId?: number;
      url?: string;
      title?: string;
      index?: number;
    }>;
  };
  windows: { update(id: number, d: { focused: boolean }): Promise<unknown> };
  runtime: {
    onMessage: {
      addListener(
        f: (m: {
          type?: string;
          tab?: number;
          server?: string;
          id?: string;
        }) => Promise<unknown> | undefined,
      ): void;
    };
    onInstalled: { addListener(f: () => void): void };
  };
  contextMenus: {
    create(d: { id: string; title: string; contexts: string[] }): void;
    onClicked: { addListener(f: (info: unknown, tab?: { id?: number }) => void): void };
  };
  notifications: {
    create(d: {
      type: 'basic';
      title: string;
      message: string;
      iconUrl?: string;
    }): Promise<unknown>;
  };
};

export interface Kept {
  ok: boolean;
  id?: string;
  title?: string;
  kind?: string;
  error?: string;
  server?: string;
}

const KIND_WORDS: Record<string, string> = {
  recipe: 'a recipe',
  paper: 'a paper',
  video: 'a video',
  song: 'a song',
  article: 'an article',
  bookmark: 'a bookmark, on the Bookmarks shelf',
};

/** The time now, `14:32`: when a page was saved or a piece brought in. */
const clockNow = () => {
  const d = new Date();
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

const today = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

async function config() {
  const m = await browser.storage.managed
    .get()
    .catch(() => ({}) as { key?: string; server?: string });
  if (!m.key || !m.server) {
    throw new Error(
      'This add-on works inside formicaria’s own browser. Open formicaria from its launcher.',
    );
  }
  return { key: m.key, server: m.server.replace(/\/+$/, '') };
}

async function call(server: string, key: string, path: string, body: BodyInit, json: boolean) {
  const r = await fetch(`${server}/api/${path}`, {
    method: 'POST',
    headers: { 'X-Formicaria-Key': key, 'Content-Type': json ? 'application/json' : 'text/plain' },
    body,
  });
  const text = await r.text();
  if (!r.ok) throw new Error(text || `formicaria answered ${r.status}`);
  return text ? JSON.parse(text) : null;
}

/** Keep the page in tab `tab`. */
export async function keep(tab: number): Promise<Kept> {
  let server: string | undefined;
  try {
    const c = await config();
    server = c.server;
    await browser.tabs.executeScript(tab, { file: '/content.js' });
    const [facts] = (await browser.tabs.executeScript(tab, {
      code: 'formicaria_content.collect()',
    })) as (PageFacts & { snapshot: string })[];
    if (!facts) throw new Error('The page could not be read.');
    if (!/^https?:/.test(facts.url)) throw new Error('Only a web page can be kept.');
    let recipe: WebRecipe | null = null;
    if (kindOf(facts) === 'recipe') {
      recipe = await call(
        c.server,
        c.key,
        `recipe_from_page?url=${encodeURIComponent(facts.url)}`,
        facts.snapshot,
        false,
      ).catch(() => null);
    }
    const draft = buildClip(facts, today(), recipe, clockNow());
    // **One self-contained file** (2026-11-17): the copy's pictures, styles and fonts put inside it,
    // fetched with the add-on's rights (and the page's cookies, as the page showed them).
    try {
      facts.snapshot = (
        await inlinePage(facts.snapshot, facts.url, fetchGetter(6, { credentials: 'include' }))
      ).html;
    } catch {
      /* the plain copy is still kept */
    }
    const made = await call(
      c.server,
      c.key,
      'clip_save',
      JSON.stringify({
        url: facts.url,
        title: draft.title,
        body: draft.body,
        tags: draft.tags,
        props: draft.props,
        bibtex: draft.bibtex ?? '',
        snapshot: facts.snapshot,
      }),
      true,
    );
    return { ok: true, id: made?.id, title: draft.title, kind: draft.kind, server };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e), server };
  }
}

/** **A bookmark is a note** (`browser-plan` §13.3): the page's title and link, on the Bookmarks shelf,
 *  without a copy of the page, so it is light. Synced by git like every note. */
export async function bookmark(tab: { url?: string; title?: string }): Promise<Kept> {
  try {
    const c = await config();
    const url = tab.url ?? '';
    if (!/^https?:/.test(url)) throw new Error('Only a web page can be bookmarked.');
    const title = (tab.title || url).trim();
    const made = await call(
      c.server,
      c.key,
      'clip_save',
      JSON.stringify({
        url,
        title,
        body: `[${title.replace(/[[\]]/g, '')}](${url})\n\n---\n\nBookmarked on ${today()}.\n`,
        tags: ['web', 'shelf:Bookmarks'],
        props: { source_url: url, saved: today() },
        bibtex: '',
        snapshot: '',
      }),
      true,
    );
    return { ok: true, id: made?.id, title, kind: 'bookmark', server: c.server };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** **Show a note: its own tab** (`decisions.md` 2026-11-17, *one server, many views*). */
export async function show(server: string, id: string) {
  await browser.tabs.create({ url: `${server}/f/note/${id}` });
}

/** **Open one of formicaria's items in a tab of its own**: a note, a feature, a view
 *  (`note:<id>`, `view:group:<name>`, …), or `new:` for a new note. */
async function openItem(address: string) {
  const c = await config();
  if (address === 'new:') {
    const made = (await call(
      c.server,
      c.key,
      'create_notes',
      JSON.stringify({ notes: [{ title: '', body: '' }] }),
      true,
    )) as { id: string }[] | null;
    if (made?.[0]?.id) await browser.tabs.create({ url: `${c.server}/f/note/${made[0].id}` });
    return;
  }
  await browser.tabs.create({ url: `${c.server}${addressUrl(address)}` });
}

/** **Put a piece into a note, on the server** (`actions.rs`): the one way content moves in, whether
 *  or not the note is open anywhere. `replace` swaps what an earlier put wrote (Undo, an
 *  alternative). */
async function putPiece(args: Record<string, unknown>): Promise<{ id?: string } | null> {
  const c = await config();
  return (await call(c.server, c.key, 'put', JSON.stringify(args), true)) as { id?: string } | null;
}
async function replaceIn(note: unknown, from: unknown, to: string): Promise<Reply> {
  if (typeof note !== 'string' || typeof from !== 'string')
    return { ok: false, error: 'nothing to change' };
  await putPiece({ to: { note }, md: to, replace: from });
  return { ok: true };
}

// --- the bridge into formicaria's panel (`bridge.ts`) ----------------------------------------

interface Port {
  name: string;
  sender?: { url?: string };
  onMessage: { addListener(f: (m: unknown) => void): void };
  onDisconnect: { addListener(f: () => void): void };
  postMessage(m: unknown): void;
  disconnect(): void;
}
interface Reply {
  n?: number;
  ok?: boolean;
  title?: string;
  error?: string;
  /** The note it went into, what went in, and its one alternative: for Undo and the bubble. */
  note?: string;
  inserted?: string;
  alt?: { label: string; token: string };
  open?: string;
  /** What the bubble says, when not "Into …" (a video being fetched). */
  say?: string;
}
let bridge: Port | null = null;
let asked = 0;
const waiting = new Map<number, (r: Reply) => void>();

(
  browser.runtime as unknown as { onConnect: { addListener(f: (p: Port) => void): void } }
).onConnect.addListener((p) => {
  if (p.name !== 'formicaria-bridge') return;
  void config()
    .then((c) => {
      // Only formicaria's own page, at this run's address, may be the bridge.
      if (!(p.sender?.url ?? '').startsWith(`${c.server}/`)) return p.disconnect();
      bridge = p;
      p.onDisconnect.addListener(() => {
        if (bridge === p) bridge = null;
      });
      p.onMessage.addListener((m) => {
        const r = m as Reply & { up?: string };
        if (typeof r?.n === 'number') {
          waiting.get(r.n)?.(r);
          waiting.delete(r.n);
        } else if (typeof r?.up === 'string') {
          void fromApp(m as Record<string, unknown>);
        }
      });
    })
    .catch(() => p.disconnect());
});

/** Ask the app in the panel; it answers `{ ok, title?, error? }`. */
export function ask(m: Record<string, unknown>): Promise<Reply> {
  const b = bridge;
  if (!b) return Promise.reject(new Error('formicaria is not open. Alt+Shift+F opens it.'));
  const n = ++asked;
  return new Promise((resolve, reject) => {
    waiting.set(n, resolve);
    setTimeout(() => {
      if (waiting.delete(n)) reject(new Error('formicaria did not answer.'));
    }, 8000);
    b.postMessage({ ...m, n });
  });
}

// --- "Bring this in" (`src/lib/web/bring.ts`; research §11) ------------------------------------

/** The note in front in formicaria, as the app last said: where *Bring in* goes. */
interface Front {
  id: string;
  title: string;
  vault: string;
  /** The assistant running now, by name (`Myrme`), or empty. */
  assistant: string;
}
let front: Front | null = null;
/** The one alternative each bring-in offers afterwards (*Link only instead*, *Keep the file*). */
const alternatives = new Map<string, () => Promise<string>>();
let tokens = 0;

const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));
const mb = (n: number) => `${(n / 1048576).toFixed(n < 10485760 ? 1 : 0)} MB`;

/** Hand the text to the note in front (or a new note named after the page), with its alternative. */
let quietNow = false;
/** Where the piece goes, when chosen on the page (the chooser): a note, or a new one. */
let aimNow: { note?: string; noteTitle?: string; fresh?: boolean } = {};
async function deliver(
  p: Piece,
  text: string,
  what: string,
  alt?: { label: string; make: () => Promise<string> },
  quiet = quietNow,
): Promise<Reply> {
  let token: string | undefined;
  if (alt) {
    token = String(++tokens);
    alternatives.set(token, alt.make);
    if (alternatives.size > 50) alternatives.delete(alternatives.keys().next().value as string);
  }
  const offered = alt && token ? { label: alt.label, token } : undefined;
  void quiet;
  try {
    // Into the note chosen on the page, else the note tab looked at last, else a new note named
    // after the page.
    const into = aimNow.note
      ? { id: aimNow.note, title: aimNow.noteTitle || 'your note' }
      : !aimNow.fresh && front
        ? { id: front.id, title: front.title || 'your note' }
        : null;
    if (into) {
      await putPiece({ to: { note: into.id }, md: text });
      return { ok: true, title: into.title, note: into.id, inserted: text, alt: offered };
    }
    const title = p.pageTitle || 'From the web';
    const made = await putPiece({ to: { new: { title } }, md: text });
    if (made?.id) front = { id: made.id, title, vault: '', assistant: front?.assistant ?? '' };
    void what;
    return { ok: true, title, note: made?.id, inserted: text, alt: offered };
  } catch (e) {
    return { ok: false, error: msg(e) };
  }
}

/** The file at `src`, fetched as the page would (its cookies, its address as referrer). */
async function fetchBytes(src: string, referrer: string) {
  const r = await fetch(src, { credentials: 'include', referrer: referrer || undefined });
  if (!r.ok) throw new Error(`the site answered ${r.status}`);
  const bytes = new Uint8Array(await r.arrayBuffer());
  if (!bytes.length) throw new Error('the site sent nothing');
  return { bytes, type: r.headers.get('content-type') ?? '' };
}

/** Keep bytes in the vault of the note in front; answers the reference and the attachment's note. */
async function store(bytes: Uint8Array, name: string) {
  const c = await config();
  const v = encodeURIComponent(front?.vault ?? '');
  const meta = (await call(
    c.server,
    c.key,
    `ingest?name=${encodeURIComponent(name)}&vault=${v}`,
    new Blob([bytes as BlobPart]),
    false,
  )) as { id: string; assets: string[] };
  return { ref: (meta.assets[0] ?? '').replace(/^sha256:/, 'sha256-'), id: meta.id };
}

async function forget(id: string) {
  const c = await config();
  await call(c.server, c.key, 'delete', JSON.stringify({ id }), true).catch(() => null);
}

/** **How** a piece comes in, when the selection offers a choice (the chooser by a selection): a
 *  formula as LaTeX or as its picture; a video or recording whole, as its sound, or as a link. */
export type How = 'latex' | 'picture' | 'video' | 'mp3' | 'link';
const HOWS: How[] = ['latex', 'picture', 'video', 'mp3', 'link'];

/** **A selected video or recording, kept by the media tools** (Save media's, on this computer): into
 *  the note chosen, else Music or Videos. Fetching takes a while; the note gains it when it is done. */
async function keepMedia(
  p: Piece,
  how: 'video' | 'mp3',
  aim: { note?: string; noteTitle?: string; fresh?: boolean },
): Promise<Reply> {
  const into = aim.note
    ? { id: aim.note, title: aim.noteTitle || 'your note' }
    : !aim.fresh && front
      ? { id: front.id, title: front.title || 'your note' }
      : null;
  const what = how === 'mp3' ? 'mp3' : p.kind === 'audio' ? 'audio' : 'video';
  const shelf = what === 'video' ? 'Videos' : 'Music';
  try {
    const c = await config();
    await call(
      c.server,
      c.key,
      'media_save',
      JSON.stringify({
        url: p.fetch || p.src || p.pageUrl,
        page: p.pageUrl,
        what,
        quality: 720,
        mp3: 'v2',
        into: into ? 'note' : what === 'video' ? 'videos' : 'music',
        list: '',
        note: into?.id ?? '',
        vault: front?.vault ?? '',
      }),
      true,
    );
    const thing =
      what === 'mp3' ? 'its sound, as MP3' : what === 'audio' ? 'the recording' : 'the video';
    return {
      ok: true,
      title: into?.title ?? shelf,
      note: into?.id,
      say: `Fetching ${thing} into “${into?.title ?? shelf}”: it appears there when it is done.`,
    };
  } catch (e) {
    return { ok: false, error: msg(e) };
  }
}

/** **One pipeline, whatever started it**: what the piece is decides the content or a link. */
export async function bringPiece(
  p: Piece,
  quiet = false,
  aim: { note?: string; noteTitle?: string; fresh?: boolean; how?: How } = {},
): Promise<Reply> {
  p = { ...p, when: p.when ?? `${today()} ${clockNow()}` };
  quietNow = quiet;
  aimNow = aim;
  switch (p.kind) {
    case 'formula': {
      // As its picture (when the page drew one), the image's own way in; else its LaTeX, with the
      // picture as the one alternative.
      if (aim.how === 'picture' && p.src)
        return bringPiece({ ...p, kind: 'image', alt: p.tex ?? '' }, quiet, aim);
      const src = p.src;
      return deliver(
        p,
        formulaPiece(p),
        'the formula',
        src && !/^(blob|data):/.test(src)
          ? {
              label: 'As a picture instead',
              make: async () => {
                const { bytes } = await fetchBytes(src, p.pageUrl);
                return imageKept(
                  { ...p, alt: p.tex ?? '' },
                  (await store(bytes, fileName(src, 'formula.svg'))).ref,
                );
              },
            }
          : undefined,
      );
    }
    case 'text':
      return deliver(p, textPiece(p), 'the passage');
    case 'link':
      return deliver(p, linkPiece(p), 'the link');
    case 'page':
      return deliver(p, pagePiece(p), 'a link to the page');
    case 'image': {
      if (!p.src || /^(blob|mediasource):/.test(p.src))
        return deliver(p, imageLinked(p), 'the image as a link');
      try {
        const { bytes, type } = await fetchBytes(p.src, p.pageUrl);
        if (type && !/^image\//.test(type)) throw new Error('not an image');
        const name = fileName(p.src, 'image');
        if (bytes.length > IMAGE_LIMIT) {
          return deliver(p, imageLinked(p), 'the image as a link', {
            label: `Keep a copy (${mb(bytes.length)})`,
            make: async () => imageKept(p, (await store(bytes, name)).ref),
          });
        }
        const kept = await store(bytes, name);
        return deliver(p, imageKept(p, kept.ref), 'the image', {
          label: 'Link only instead',
          make: async () => {
            await forget(kept.id);
            return imageLinked(p);
          },
        });
      } catch {
        return deliver(p, imageLinked(p), 'the image as a link');
      }
    }
    case 'video':
    case 'audio': {
      if (aim.how === 'video' || aim.how === 'mp3') return keepMedia(p, aim.how, aim);
      const word = p.kind === 'video' ? 'the video' : 'the audio';
      if (streamed(p.src)) return deliver(p, mediaLinked(p), `${word}, at this moment`);
      const src = p.src as string;
      return deliver(p, mediaLinked(p), `${word} as a link`, {
        label: 'Keep the file',
        make: async () => {
          const { bytes } = await fetchBytes(src, p.pageUrl);
          const name = fileName(src, p.kind);
          return mediaKept(p, (await store(bytes, name)).ref, name);
        },
      });
    }
  }
}

/** Shown only when formicaria could not say it itself (it is not open). */
function failed(r: Reply) {
  if (r.ok) return;
  void browser.notifications.create({
    type: 'basic',
    title: 'Not brought in',
    message: r.error ?? '',
    iconUrl: 'icon.svg',
  });
}

/** The selection in a tab (or one of its frames), as HTML and words. */
const SELECTION = `(() => { const s = getSelection(); if (!s || !s.rangeCount) return { html: '', text: '' };
  const d = document.createElement('div'); for (let i = 0; i < s.rangeCount; i++) d.append(s.getRangeAt(i).cloneContents());
  return { html: d.innerHTML, text: s.toString().trim() }; })()`;

async function selectionIn(tab: number, frameId?: number): Promise<{ html: string; text: string }> {
  const [r] = (await (
    browser.tabs.executeScript as (
      t: number,
      d: { code: string; frameId?: number },
    ) => Promise<unknown[]>
  )(tab, { code: SELECTION, frameId }).catch(() => [null])) as ({
    html: string;
    text: string;
  } | null)[];
  return r ?? { html: '', text: '' };
}

/** What the tab shows now (`tabs.captureVisibleTab`), taken once, at the snip. */
function capture(windowId: number | undefined): Promise<string> {
  return (
    browser.tabs as unknown as {
      captureVisibleTab(w: number | undefined, o: { format: string }): Promise<string>;
    }
  ).captureVisibleTab(windowId, { format: 'png' });
}

/** A captured view waiting on the snip page (`shot.html`), for a tab the add-on cannot enter. */
let pendingShot: { png: string; page: { url: string; title: string } } | null = null;

/** **Snip, in any tab** (2026-11-17): in the page when the add-on is there; otherwise (a PDF in
 *  Firefox's viewer, a page closed to add-ons) what the tab shows, captured now, on the snip page
 *  beside it, where the area is dragged. */
async function snipIn(t: {
  id?: number;
  windowId?: number;
  url?: string;
  title?: string;
  index?: number;
}) {
  if (t.id == null) return;
  const inPage = await browser.tabs.sendMessage(t.id, { type: 'snip' }).then(
    () => true,
    () => false,
  );
  if (inPage) return;
  try {
    // The tab named must be the one captured: in front, and painted.
    await browser.tabs.update(t.id, { active: true });
    await new Promise((r) => setTimeout(r, 200));
    const png = await capture(t.windowId);
    pendingShot = { png, page: { url: t.url ?? '', title: t.title ?? '' } };
    await browser.tabs.create({ url: '/shot.html', index: (t.index ?? 0) + 1 });
  } catch (e) {
    failed({ ok: false, error: msg(e) });
  }
}

/** An area of a captured view, as PNG bytes. */
async function crop(
  shot: Promise<string>,
  area: { x: number; y: number; width: number; height: number },
  dpr: number,
): Promise<Uint8Array> {
  const img = new Image();
  img.src = await shot;
  await img.decode();
  const c = document.createElement('canvas');
  c.width = Math.max(1, Math.round(area.width * dpr));
  c.height = Math.max(1, Math.round(area.height * dpr));
  c.getContext('2d')!.drawImage(
    img,
    area.x * dpr,
    area.y * dpr,
    c.width,
    c.height,
    0,
    0,
    c.width,
    c.height,
  );
  const blob = await new Promise<Blob>((ok, no) =>
    c.toBlob((b) => (b ? ok(b) : no(new Error('no picture'))), 'image/png'),
  );
  return new Uint8Array(await blob.arrayBuffer());
}

/** **Snip**: everything in the area, each as itself, in reading order, as one insert ending with its
 *  source; an area with nothing readable in it is a picture of the area. */
async function snip(
  p: Piece,
  blocks: Block[],
  shot: (() => Promise<Uint8Array>) | null,
  sub: (area: NonNullable<Block['rect']>) => Promise<Uint8Array>,
): Promise<Reply> {
  const asPicture = async () => {
    if (!shot) throw new Error('no area to picture');
    const bytes = await shot();
    const kept = await store(bytes, 'snip.png');
    return `![Part of ${p.pageTitle.replace(/[[\]]/g, '') || 'a page'}](asset:${kept.ref})\n\n${source(p)}`;
  };
  if (!blocks.some((b) => b.kind !== 'picture')) {
    if (!blocks.length && shot) {
      // Nothing readable: the picture of the area, at once.
      return deliver(p, await asPicture(), 'a picture of the area');
    }
  }
  const out: string[] = [];
  let run: string[] = [];
  let firstWords = '';
  const flush = () => {
    if (!run.length) return;
    const md = markdown(run.join(''));
    out.push(
      md
        .split('\n')
        .map((l) => (l.trim() ? `> ${l}` : '>'))
        .join('\n'),
    );
    run = [];
  };
  for (const b of blocks) {
    if (b.kind === 'text') {
      if (!firstWords && b.text) firstWords = b.text.split(/\s+/).slice(0, 12).join(' ');
      run.push(b.html ?? '');
      continue;
    }
    flush();
    if (b.kind === 'block') out.push(markdown(b.html ?? ''));
    else if (b.kind === 'image' && b.src) {
      try {
        const { bytes, type } = await fetchBytes(b.src, p.pageUrl);
        if ((type && !/^image\//.test(type)) || bytes.length > IMAGE_LIMIT) throw new Error('link');
        const kept = await store(bytes, fileName(b.src, 'image'));
        out.push(`![${(b.alt ?? '').replace(/[[\]]/g, '')}](asset:${kept.ref})`);
      } catch {
        out.push(`[Image${b.alt ? `: ${b.alt.replace(/[[\]]/g, '')}` : ''}](${b.src})`);
      }
    } else if (b.kind === 'video' || b.kind === 'audio') {
      out.push(
        mediaLinked({ ...p, kind: b.kind, src: b.src, time: b.time }).replace(/ — \[.*$/, ''),
      );
    } else if (b.kind === 'picture' && b.rect) {
      const kept = await store(await sub(b.rect), 'figure.png');
      out.push(`![](asset:${kept.ref})`);
    }
  }
  flush();
  const text = `${out.join('\n\n')}\n\n${source(p, firstWords || undefined)}`;
  return deliver(
    p,
    text,
    shot ? 'the area' : 'it',
    shot ? { label: 'As a picture instead', make: asPicture } : undefined,
  );
}

/** Messages the app sends up the bridge: the note in front, an alternative chosen, a drop. */
async function fromApp(m: Record<string, unknown>) {
  if (m.up === 'front') {
    front =
      typeof m.id === 'string' && m.id
        ? {
            id: m.id,
            title: String(m.title ?? ''),
            vault: String(m.vault ?? ''),
            assistant: String(m.assistant ?? ''),
          }
        : null;
    void storage.local.set({ front: front?.title ?? '' });
    void menus.update('formicaria-bring', { title: bringTitle() }).catch(() => {});
  } else if (
    m.up === 'open-beside' &&
    typeof m.url === 'string' &&
    /^https:\/\/(mail|calendar)\.google\.com\//.test(m.url)
  ) {
    // **A hand-off, beside the note** (workspace-design §13.6): Gmail (a reply, a new mail) or Google
    // Calendar (a new event), opened with the note's content in it; the person finishes there. Those
    // two addresses only.
    await browser.tabs.create({ url: m.url });
  } else if (
    m.up === 'open-tab' &&
    typeof m.address === 'string' &&
    /^(note|view):/.test(m.address)
  ) {
    // **Open as a tab**: the item's own page (`/f/…`).
    await openItem(m.address);
  } else if (m.up === 'after' && ['offer', 'bring', 'nothing'].includes(String(m.value))) {
    // Settings → Browser: what follows a selection on a page (`select.ts` reads it).
    void storage.local.set({ after: String(m.value) });
  } else if (m.up === 'alt' && typeof m.token === 'string') {
    const make = alternatives.get(m.token);
    if (!make) return;
    alternatives.delete(m.token);
    try {
      const to = await make();
      await replaceIn(m.note, m.from, to);
    } catch (e) {
      failed({ ok: false, error: msg(e) });
    }
  } else if (m.up === 'drop') {
    const [t] = await browser.tabs.query({ active: true, currentWindow: true });
    const tt = (t ?? {}) as { url?: string; title?: string };
    const page = { url: tt.url ?? '', title: tt.title ?? '' };
    const html = String(m.html ?? '');
    const text = String(m.text ?? '');
    const uri = String(m.uri ?? '');
    let p = classify(html, text, page);
    if (p.kind === 'page' && uri) p = { ...p, kind: 'link', src: uri, alt: text || uri };
    failed(await bringPiece(p));
  }
}

// --- the APIs used below, typed where they are used ---------------------------------------------

const pageAction = (
  browser as unknown as {
    pageAction: {
      setIcon(d: { tabId: number; path: string }): Promise<void>;
      setTitle(d: { tabId: number; title: string }): Promise<void>;
    };
  }
).pageAction;
const storage = (
  browser as unknown as { storage: { local: { set(d: Record<string, unknown>): Promise<void> } } }
).storage;
const menus = browser.contextMenus as unknown as {
  create(d: { id: string; title: string; contexts: string[] }): void;
  update(id: string, d: { title: string }): Promise<void>;
  removeAll(): Promise<void>;
  onClicked: {
    addListener(
      f: (info: unknown, tab?: { id?: number; url?: string; title?: string }) => void,
    ): void;
  };
};

// --- "this page is in your notes" -------------------------------------------------------------

export interface PageNote {
  id: string;
  title: string;
  context: string;
}
const seen = new Map<number, PageNote[]>();

/** The notes naming the page in `tab`, and the count on the toolbar button. Asked when a tab is shown
 *  or finishes loading, never polled. */
export async function pageNotes(tabId: number, url: string): Promise<PageNote[]> {
  let found: PageNote[] = [];
  if (/^https?:/.test(url)) {
    try {
      const c = await config();
      if (!url.startsWith(`${c.server}/`)) {
        found = (await call(c.server, c.key, 'page_notes', JSON.stringify({ url }), true)) ?? [];
      }
    } catch {
      found = [];
    }
  }
  seen.set(tabId, found);
  // **The page's ant, in the address bar** (research §11.3, rule 4): filled when notes name the page.
  const n = found.length;
  void pageAction
    .setIcon({ tabId, path: n ? 'icon-page-in.svg' : 'icon-page.svg' })
    .catch(() => {});
  void pageAction
    .setTitle({
      tabId,
      title: n
        ? `In ${n === 1 ? 'one' : n} of your notes · Snip what to bring in, or save the page`
        : 'Snip what to bring into formicaria, or save the page (Alt+Shift+S)',
    })
    .catch(() => {});
  return found;
}

const tabsEvents = browser.tabs as unknown as {
  onActivated: { addListener(f: (i: { tabId: number }) => void): void };
  onUpdated: {
    addListener(
      f: (id: number, c: { status?: string; url?: string }, t: { url?: string }) => void,
    ): void;
  };
  onRemoved: { addListener(f: (id: number) => void): void };
  get(id: number): Promise<{ url?: string }>;
};
tabsEvents.onActivated.addListener(({ tabId }) => {
  void tabsEvents.get(tabId).then((t) => {
    void pageNotes(tabId, t.url ?? '');
    void noteInFront(t as { url?: string; title?: string });
  });
});
async function wholeAppToNotes(id: number, url: string) {
  const c = await config().catch(() => null);
  if (!c) return;
  const rest = url.startsWith(c.server) ? url.slice(c.server.length) : null;
  if (rest === '/' || rest === '' || rest?.startsWith('/#') || rest?.startsWith('/?')) {
    await (
      browser.tabs as unknown as { update(id: number, o: { url: string }): Promise<unknown> }
    ).update(id, {
      url: `${c.server}/f/group/Library`,
    });
  }
}

/** **The note in front is the note tab looked at last** (its `/f/note/<id>` page): where *Bring in*
 *  goes unless another note is chosen. */
async function noteInFront(t: { url?: string; title?: string }) {
  const c = await config().catch(() => null);
  const m =
    c && t.url
      ? new RegExp(
          `^${c.server.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}/f/note/([0-9A-Z]{26})`,
        ).exec(t.url)
      : null;
  if (!m) return;
  const title = (t.title ?? '').replace(/ — formicaria$/, '');
  front = { id: m[1], title, vault: '', assistant: front?.assistant ?? '' };
  void storage.local.set({ front: title });
  void menus.update('formicaria-bring', { title: bringTitle() }).catch(() => {});
}
tabsEvents.onUpdated.addListener((id, change, t) => {
  // **The old whole-app page in a tab becomes Notes** (`decisions.md` 2026-11-17, *one frame*): the
  // app with its own bars and panes is not shown in formicaria's browser any more.
  if (typeof change.url === 'string') void wholeAppToNotes(id, change.url);
  if (change.status === 'complete') {
    void pageNotes(id, t.url ?? '');
    if ((t as { active?: boolean }).active) void noteInFront(t as { url?: string; title?: string });
  }
});
tabsEvents.onRemoved.addListener((id) => seen.delete(id));

// --- `fm` in the address bar: the vault's search ----------------------------------------------

const omnibox = (
  browser as unknown as {
    omnibox: {
      setDefaultSuggestion(d: { description: string }): void;
      onInputChanged: {
        addListener(
          f: (t: string, suggest: (s: { content: string; description: string }[]) => void) => void,
        ): void;
      };
      onInputEntered: { addListener(f: (t: string) => void): void };
    };
  }
).omnibox;
omnibox.setDefaultSuggestion({ description: 'Search the library: every section of formicaria' });
let lastHits: { id: string; title: string }[] = [];
omnibox.onInputChanged.addListener((text, suggest) => {
  void config()
    .then((c) => call(c.server, c.key, 'search', JSON.stringify({ query: text }), true))
    .then(
      (
        hits:
          { id: string; title?: string; tags?: string[]; props?: Record<string, unknown> }[] | null,
      ) => {
        // **The library's search** (workspace-design §11, D4): each hit with its section.
        const top = (hits ?? []).slice(0, 8);
        lastHits = top.map((h) => ({ id: h.id, title: h.title || 'Untitled' }));
        suggest(
          top.map((h) => {
            const s = sectionOf(h);
            return {
              content: `note:${h.id}`,
              description: `${s.mark} ${s.label} · ${h.title || 'Untitled'}`,
            };
          }),
        );
      },
    )
    .catch(() => suggest([]));
});
omnibox.onInputEntered.addListener((text) => {
  const id = text.startsWith('note:') ? text.slice(5) : lastHits[0]?.id;
  if (!id) return;
  // Opened as a tab of the collection, beside the pages (workspace-design §11).
  const title = lastHits.find((h) => h.id === id)?.title ?? 'formicaria';
  void fromApp({ up: 'open-tab', address: `note:${id}`, title });
});

(
  browser.runtime.onMessage as unknown as {
    addListener(
      f: (
        m: Record<string, unknown> & { type?: string; tab?: number; server?: string; id?: string },
        sender: { tab?: { id?: number; windowId?: number; url?: string; title?: string } },
      ) => Promise<unknown> | undefined,
    ): void;
  }
).addListener((m, sender) => {
  const d = m as { type?: string; where?: Where; p?: Pending };
  const st = sender?.tab;
  const here = { url: st?.url ?? '', title: st?.title ?? '' };
  // --- from the page script (`select.ts`, `snip.ts`): the bubble on the spot shows the result ---
  if (m?.type === 'snip-info' && st?.id != null) {
    const id = st.id;
    return pageNotes(id, here.url).then((notes) => ({
      front: front?.title ?? '',
      assistant: front?.assistant ?? '',
      notes,
    }));
  }
  if (m?.type === 'snip' && st?.id != null) {
    const area = m.area as { x: number; y: number; width: number; height: number } | null;
    const dpr = Number(m.dpr) || 1;
    const p: Piece = {
      kind: 'text',
      pageUrl: here.url,
      pageTitle: here.title,
      when: `${today()} ${clockNow()}`,
    };
    // The view is captured once, now, before anything moves (or the bubble appears); a picture is
    // cropped from it only when one is wanted.
    const shot = capture(st.windowId);
    shot.catch(() => {});
    const cut = (a: { x: number; y: number; width: number; height: number }) => crop(shot, a, dpr);
    quietNow = true;
    return snip(p, (m.blocks as Block[]) ?? [], area ? () => cut(area) : null, cut).finally(
      () => (quietNow = false),
    );
  }
  // Snip in a tab named (the ant's panel opened as a page, `usability-check`), as for Save page.
  if (m?.type === 'snip-tab' && typeof m.tab === 'number') {
    return browser.tabs.get(m.tab).then((t) => snipIn(t));
  }
  // The snip page (`shot.html`): what was captured, then the area dragged on it.
  if (m?.type === 'shot-get') return Promise.resolve(pendingShot);
  if (m?.type === 'shot-bring' && pendingShot) {
    const s = pendingShot;
    const area = m.area as { x: number; y: number; width: number; height: number };
    const p: Piece = {
      kind: 'text',
      pageUrl: s.page.url,
      pageTitle: s.page.title,
      when: `${today()} ${clockNow()}`,
    };
    const cut = (a: { x: number; y: number; width: number; height: number }) =>
      crop(Promise.resolve(s.png), a, 1);
    return snip(p, [], () => cut(area), cut);
  }
  if (m?.type === 'ask-assistant') {
    const blocks = (m.blocks as Block[]) ?? [];
    const text = blocks
      .map((b) => (b.html ? markdown(b.html) : (b.text ?? '')))
      .filter(Boolean)
      .join('\n\n')
      .slice(0, 12000);
    return ask({ type: 'ask', text, page: { title: here.title, url: here.url } }).catch((e) => ({
      ok: false,
      error: msg(e),
    }));
  }
  // From Snip's bubble, in the page it is about: that page. A message naming its tab is answered
  // below, for that tab (the ant's panel opened as a page of its own is not the page to keep).
  if (m?.type === 'save-page' && typeof m.tab !== 'number' && st?.id != null) {
    const id = st.id;
    return savePage(id).then((k) => ({ ok: k.ok, title: k.title, error: k.error, open: k.id }));
  }
  if (m?.type === 'alt' && typeof m.token === 'string') {
    const make = alternatives.get(m.token);
    if (!make) return Promise.resolve({ ok: false, error: 'It can no longer be changed.' });
    alternatives.delete(m.token);
    return make()
      .then((to) => replaceIn(m.note, m.from, to))
      .catch((e) => ({ ok: false, error: msg(e) }));
  }
  if (m?.type === 'undo') {
    return replaceIn(m.note, m.from, '').catch((e) => ({ ok: false, error: msg(e) }));
  }
  if (m?.type === 'open' && typeof m.id === 'string') {
    const id = m.id;
    return config().then((c) => show(c.server, id));
  }
  if (m?.type === 'link-instead-here' && typeof m.id === 'string' && st) {
    return forget(m.id).then(() => bookmark(st));
  }
  if (d?.type === 'download-choice' && d.where && d.p) return choose(d.where, d.p);
  if (m?.type === 'keep' && typeof m.tab === 'number') return keep(m.tab);
  if (m?.type === 'show' && m.server && m.id) return show(m.server, m.id);
  if (m?.type === 'server') return config().then((c) => ({ server: c.server }));
  // *Just the link instead*, after a save: the saved note goes, a bookmark comes.
  if (m?.type === 'link-instead' && typeof m.tab === 'number' && m.id) {
    const tab = m.tab;
    const was = m.id;
    return forget(was).then(() =>
      tabsEvents.get(tab).then((t) => bookmark(t as { url?: string; title?: string })),
    );
  }
  // From the selection chip (`select.ts`) and Pick (`pick.ts`), in the tab that sent it.
  const from = m as unknown as {
    type?: string;
    html?: string;
    text?: string;
    piece?: Partial<Piece>;
    note?: unknown;
    noteTitle?: unknown;
    fresh?: unknown;
    how?: unknown;
  };
  if (from?.type === 'bring-selection' || from?.type === 'bring-piece') {
    return browser.tabs.query({ active: true, currentWindow: true }).then(async ([t]) => {
      const tt = (t ?? {}) as { url?: string; title?: string };
      const page = { url: tt.url ?? '', title: tt.title ?? '' };
      const p: Piece =
        from.type === 'bring-selection'
          ? classify(from.html ?? '', from.text ?? '', page)
          : ({ ...from.piece, pageUrl: page.url, pageTitle: page.title } as Piece);
      const aim = {
        note: typeof from.note === 'string' ? from.note : undefined,
        noteTitle: typeof from.noteTitle === 'string' ? from.noteTitle : undefined,
        fresh: from.fresh === true,
        how: HOWS.includes(from.how as How) ? (from.how as How) : undefined,
      };
      const r = await bringPiece(p, from.type === 'bring-selection', aim);
      if (from.type === 'bring-piece') failed(r);
      return r;
    });
  }
  // **The ant's options** (`launcher.ts`): a page of formicaria's server (Settings, the manual).
  if (
    m?.type === 'open-path' &&
    typeof m.path === 'string' &&
    /^\/(f\/settings|f\/privacy|manual\/)/.test(m.path)
  ) {
    return config().then((c) => browser.tabs.create({ url: c.server + m.path }));
  }
  // **Save media** and **Save page** from the ant's panel: the page in front, or the tab named (a
  // panel opened as a page of its own, as `ci/usability-check.py` drives it).
  if (m?.type === 'save-media' || m?.type === 'save-page') {
    const act = (id: number): Promise<unknown> =>
      m.type === 'save-media' ? saveMedia(id) : savePage(id);
    if (typeof m.tab === 'number') return act(m.tab);
    return browser.tabs
      .query({ active: true, currentWindow: true })
      .then(([t]) => (t?.id != null ? act(t.id) : undefined))
      .then(() => undefined);
  }
  // **The ant's panel** (`launcher.ts`): a feature, a note or a new note, in a tab of its own.
  if (m?.type === 'open-item' && typeof m.address === 'string') {
    return openItem(m.address).then(() => ({ ok: true }));
  }
  // **The chooser's search** (formicaria's icon by a selection): the notes a word finds, to bring into.
  if (m?.type === 'search-notes' && typeof m.q === 'string' && m.q.trim()) {
    return config()
      .then((c) => call(c.server, c.key, 'search', JSON.stringify({ query: m.q }), true))
      .then((hits: { id: string; title?: string }[] | null) =>
        (hits ?? []).slice(0, 6).map((h) => ({ id: h.id, title: h.title || 'Untitled' })),
      )
      .catch(() => []);
  }
  if (m?.type === 'notes' && typeof m.tab === 'number') {
    const tab = m.tab;
    return tabsEvents.get(tab).then((t) => pageNotes(tab, t.url ?? ''));
  }
  return undefined;
});

// **Two verbs** (research §11.3): *Bring into "…"* names the note it goes to; *Save page*; and *Pick*
// for what a selection cannot reach. Made at every start (the background page is persistent).
function bringTitle() {
  // `&` marks an access key in a menu's title; doubled, it is shown.
  return front
    ? `Bring into “${front.title.replace(/&/g, '&&') || 'your note'}”`
    : 'Bring into a new note';
}
void menus.removeAll().then(() => {
  menus.create({
    id: 'formicaria-bring',
    title: bringTitle(),
    contexts: ['selection', 'image', 'link', 'video', 'audio', 'page'],
  });
  menus.create({ id: 'formicaria-save', title: 'Save page in formicaria', contexts: ['page'] });
  menus.create({ id: 'formicaria-pick', title: 'Pick from this page…', contexts: ['page'] });
  menus.create({
    id: 'formicaria-media',
    title: 'Save media in formicaria…',
    contexts: ['page', 'video', 'audio'],
  });
});

/** **The ant in the address bar starts a snip** (its bar offers *Save the whole page* too). */
(
  browser as unknown as {
    pageAction: { onClicked: { addListener(f: (t: { id?: number }) => void): void } };
  }
).pageAction.onClicked.addListener((t) => {
  void snipIn(t as Parameters<typeof snipIn>[0]);
});

/** **Save page**: the page as its own note; formicaria says what it became. */
/** **Save media** (`decisions.md` 2026-11-17): what the page holds, in formicaria's sheet, in a tab
 *  beside it (`/f/media`): the choosing, fetching and keeping are the server's. */
export async function saveMedia(tab: number) {
  try {
    const c = await config();
    await browser.tabs.executeScript(tab, { file: '/content.js' });
    const [found] = (await browser.tabs.executeScript(tab, {
      code: 'formicaria_content.media()',
    })) as { url: string; kind: string; label: string }[][];
    const t = (await tabsEvents.get(tab)) as { url?: string; index?: number };
    const page = t.url ?? '';
    if (!/^https?:/.test(page)) throw new Error('Only a web page holds media to keep.');
    const at = `${c.server}/f/media#${encodeURIComponent(JSON.stringify({ page, c: found ?? [] }))}`;
    await browser.tabs.create({ url: at, index: (t.index ?? 0) + 1 });
  } catch (e) {
    failed({ ok: false, error: msg(e) });
  }
}

/** **The ant says a page is being kept** (`…` on it, its tooltip saying so): a long article's
 *  pictures are fetched into its copy, half a minute for Wikipedia's *Ant* (`usability-check`), and
 *  nothing else shows until the answer. */
function keeping(tab: number, on: boolean) {
  const ba = (
    browser as unknown as {
      browserAction?: {
        setBadgeText(d: { text: string; tabId?: number }): Promise<void>;
        setTitle(d: { title: string | null; tabId?: number }): Promise<void>;
      };
    }
  ).browserAction;
  if (!ba) return;
  void ba.setBadgeText({ text: on ? '…' : '', tabId: tab }).catch(() => {});
  void ba
    .setTitle({ title: on ? 'formicaria: saving this page…' : null, tabId: tab })
    .catch(() => {});
}

export async function savePage(tab: number) {
  keeping(tab, true);
  const k = await keep(tab).finally(() => keeping(tab, false));
  const text = k.ok
    ? `Saved “${k.title}” as ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}.`
    : `Not saved: ${k.error ?? ''}`;
  const said = await ask({ type: 'toast', text, id: k.id }).catch(() => null);
  if (!said?.ok) {
    void browser.notifications.create({
      type: 'basic',
      title: k.ok ? 'Saved in formicaria' : 'Not saved',
      message: k.ok ? `${k.title} — ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}.` : (k.error ?? ''),
      iconUrl: 'icon.svg',
    });
  }
  if (k.ok) void pageNotes(tab, ((await tabsEvents.get(tab)) as { url?: string }).url ?? '');
  return k;
}

menus.onClicked.addListener((info, tab) => {
  if (tab?.id == null) return;
  const id = tab.id;
  const i = info as {
    menuItemId?: string;
    selectionText?: string;
    linkUrl?: string;
    linkText?: string;
    srcUrl?: string;
    mediaType?: string;
    pageUrl?: string;
    frameId?: number;
    targetElementId?: number;
  };
  if (i.menuItemId === 'formicaria-save') return void savePage(id);
  if (i.menuItemId === 'formicaria-media') return void saveMedia(id);
  if (i.menuItemId === 'formicaria-pick') {
    return void browser.tabs
      .executeScript(id, { file: '/pick.js' })
      .catch((e) => failed({ ok: false, error: msg(e) }));
  }
  if (i.menuItemId !== 'formicaria-bring') return;
  const page = { url: tab.url ?? i.pageUrl ?? '', title: tab.title ?? '' };
  const base = { pageUrl: page.url, pageTitle: page.title };
  void (async () => {
    let p: Piece;
    if (i.selectionText) {
      // A page the add-on cannot read (a PDF in Firefox's viewer): the words the menu was given.
      const s = await selectionIn(id, i.frameId).catch(() => ({ html: '', text: '' }));
      p = classify(s.html, s.text || i.selectionText, page);
    } else if (i.mediaType === 'image' && i.srcUrl) {
      p = { ...base, kind: 'image', src: i.srcUrl };
    } else if ((i.mediaType === 'video' || i.mediaType === 'audio') && i.srcUrl !== undefined) {
      // Where it was playing, from the element itself (Firefox's `menus.getTargetElement`).
      const [at] = (await (
        browser.tabs.executeScript as (
          t: number,
          d: { code: string; frameId?: number },
        ) => Promise<unknown[]>
      )(id, {
        code: `(() => { const el = browser.menus.getTargetElement(${Number(i.targetElementId)}); return el ? { t: el.currentTime, src: el.currentSrc || el.src } : null; })()`,
        frameId: i.frameId,
      }).catch(() => [null])) as ({ t: number; src: string } | null)[];
      p = { ...base, kind: i.mediaType, src: at?.src || i.srcUrl, time: at?.t };
    } else if (i.linkUrl) {
      p = { ...base, kind: 'link', src: i.linkUrl, alt: i.linkText };
    } else {
      p = { ...base, kind: 'page' };
    }
    failed(await bringPiece(p));
  })();
});

// --- 0.32.0: the shortcut, downloads, and formicaria's colours -------------------------------

/* The rest of the WebExtension API this uses, typed where it is used. */
const more = browser as unknown as {
  commands: { onCommand: { addListener(f: (c: string) => void): void } };
  downloads: {
    onCreated: { addListener(f: (item: Parameters<typeof onCreated>[0]) => void): void };
  };
  theme: { update(t: { colors: Record<string, string> }): Promise<void> };
  runtime: { onStartup: { addListener(f: () => void): void } };
};

/** **Alt+Shift+A brings the selection (or the page's link) in; Alt+Shift+K saves the page.**
 *  (Alt+Shift+F, formicaria's panel, belongs to the browser itself: `formicaria.cfg`.) */
more.commands.onCommand.addListener((c) => {
  void browser.tabs.query({ active: true, currentWindow: true }).then(async ([t]) => {
    if (t?.id == null) return;
    const tt = t as { id: number; url?: string; title?: string };
    if (c === 'keep-page') return void savePage(tt.id);
    if (c === 'snip') return void snipIn(tt as Parameters<typeof snipIn>[0]);
    if (c !== 'add-to-note') return;
    const page = { url: tt.url ?? '', title: tt.title ?? '' };
    if (!/^https?:/.test(page.url)) return failed({ ok: false, error: 'Only from a web page.' });
    const s = await selectionIn(tt.id);
    failed(await bringPiece(classify(s.html, s.text, page)));
  });
});

/** Every download asks where it goes; formicaria is the default (`downloads.ts`). */
more.downloads.onCreated.addListener((item) => void onCreated(item));

/** **formicaria's colours** — its own palette (`ui/src/app.css`), dark or light as the system is,
 *  so the window reads as formicaria's and not as a stock Firefox. */
const DARK = {
  frame: '#16161d',
  frame_inactive: '#16161d',
  tab_background_text: '#a2a2b5',
  tab_selected: '#23232e',
  tab_text: '#e9e9f1',
  tab_line: '#dc2626',
  toolbar: '#1c1c25',
  toolbar_text: '#e9e9f1',
  toolbar_field: '#23232e',
  toolbar_field_text: '#e9e9f1',
  toolbar_field_border_focus: '#dc2626',
  toolbar_top_separator: '#2c2c39',
  toolbar_bottom_separator: '#2c2c39',
  icons: '#e9e9f1',
  icons_attention: '#dc2626',
  popup: '#1c1c25',
  popup_text: '#e9e9f1',
  popup_border: '#2c2c39',
  sidebar: '#1c1c25',
  sidebar_text: '#e9e9f1',
  sidebar_border: '#2c2c39',
  ntp_background: '#16161d',
  ntp_text: '#e9e9f1',
  button_background_active: '#2a2a37',
  button_background_hover: '#2a2a37',
};
const LIGHT = {
  frame: '#e6e5e0',
  frame_inactive: '#ecebe7',
  tab_background_text: '#5b5b66',
  tab_selected: '#ffffff',
  tab_text: '#1d1d22',
  tab_line: '#dc2626',
  toolbar: '#f7f6f3',
  toolbar_text: '#1d1d22',
  toolbar_field: '#ffffff',
  toolbar_field_text: '#1d1d22',
  toolbar_field_border_focus: '#dc2626',
  toolbar_top_separator: '#e6e5e0',
  toolbar_bottom_separator: '#e6e5e0',
  icons: '#1d1d22',
  icons_attention: '#dc2626',
  popup: '#ffffff',
  popup_text: '#1d1d22',
  popup_border: '#e6e5e0',
  sidebar: '#f7f6f3',
  sidebar_text: '#1d1d22',
  sidebar_border: '#e6e5e0',
  ntp_background: '#f7f6f3',
  ntp_text: '#1d1d22',
  button_background_active: '#e6e5e0',
  button_background_hover: '#ecebe7',
};
function paint() {
  const dark = globalThis.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
  void more.theme.update({ colors: dark ? DARK : LIGHT }).catch(() => {});
}
paint();
globalThis.matchMedia?.('(prefers-color-scheme: dark)').addEventListener?.('change', paint);
more.runtime.onStartup.addListener(paint);

// --- 2026-11-19: cookie pop-ups on more sites, without an extension ------------------------------

/** **DuckDuckGo's cookie rules on every page while *More sites* is on** (`privacy-plan-2026-11-19.md`
 *  5.1): this device's switches read from formicaria (as the startup script reads them), and the
 *  page script (`consent.ts`) registered or taken back to match; sites with protections off are left
 *  out. Read at the start and every 20 s: a few bytes from this machine. */
let consentScript: { unregister(): Promise<void> } | null = null;
let consentFor = '';
async function syncConsent() {
  try {
    const c = await config();
    const p = (await call(c.server, c.key, 'privacy', '{}', true)) as {
      cookies?: boolean;
      more_sites?: boolean;
      sites_off?: string[];
    } | null;
    const want = !!(p?.cookies && p.more_sites);
    const off = (p?.sites_off ?? []).filter((s) => /^[a-z0-9.-]+$/i.test(s));
    const key = want ? JSON.stringify(off) : '';
    if (key === consentFor) return;
    await consentScript?.unregister().catch(() => {});
    consentScript = null;
    consentFor = key;
    if (!want) return;
    const cs = (
      browser as unknown as {
        contentScripts: {
          register(o: Record<string, unknown>): Promise<{ unregister(): Promise<void> }>;
        };
      }
    ).contentScripts;
    consentScript = await cs.register({
      matches: ['http://*/*', 'https://*/*'],
      excludeMatches: ['http://127.0.0.1/*', ...off.flatMap((s) => [`*://${s}/*`, `*://*.${s}/*`])],
      js: [{ file: '/consent.js' }],
      runAt: 'document_start',
      allFrames: false,
    });
  } catch {
    /* formicaria not reachable yet: asked again shortly */
  }
}
void syncConsent();
setInterval(() => void syncConsent(), 20000);
