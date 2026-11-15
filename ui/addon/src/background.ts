// **formicaria's browser add-on: keeping a page** (`decisions.md` 2026-11-15).
//
// One flow, from the toolbar button or the page's right-click menu: read the page (`content.ts`),
// ask formicaria to read a recipe out of it if it publishes one (`recipe_from_page`, no fetch), build
// the note with the same code the app uses (`src/lib/web/clip.ts`), and keep it with a copy of the
// page (`clip_save`). The key and formicaria's address come from Firefox's managed storage, which
// formicaria writes into its browser's policies; no other add-on can read them.
import type { WebRecipe } from '../../src/lib/types';
import { buildClip, kindOf, type PageFacts } from '../../src/lib/web/clip';
import { choose, onCreated, type Pending, type Where } from './downloads';
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
    create(d: { url: string }): Promise<unknown>;
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
    const draft = buildClip(facts, today(), recipe);
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

/** Show a note in formicaria's panel: through the bridge when the app is open, otherwise by opening
 *  its address, which formicaria's browser hands to the panel (`formicaria.cfg` §4). */
export async function show(server: string, id: string) {
  const r = await ask({ type: 'open', id }).catch(() => null);
  if (r?.ok) return;
  await browser.tabs.create({ url: `${server}/#note=${id}` });
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
        const r = m as Reply;
        if (typeof r?.n === 'number') {
          waiting.get(r.n)?.(r);
          waiting.delete(r.n);
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
    }, 5000);
    b.postMessage({ ...m, n });
  });
}

/** The passage to add: the selection quoted with its source, or the link, or the page's link. */
export function passage(p: {
  selection?: string;
  title?: string;
  url?: string;
  link?: string;
  linkText?: string;
}) {
  const clean = (t: string) => t.replace(/[[\]]/g, '').trim();
  const title = clean(p.title || p.url || 'this page');
  const sel = (p.selection ?? '').trim();
  if (sel) {
    const quoted = sel
      .split(/\r?\n/)
      .map((l) => `> ${l}`.trimEnd())
      .join('\n');
    return `${quoted}\n>\n> — [${title}](${p.url})`;
  }
  if (p.link) return `- [${clean(p.linkText || p.link)}](${p.link})`;
  return `- [${title}](${p.url})`;
}

/** **Add to the open note**: the selection (or a link, or the page) goes to the end of the note open
 *  in formicaria, with where it came from. */
export async function addToNote(text: string, url: string): Promise<Reply> {
  if (!/^https?:/.test(url)) return { ok: false, error: 'Only from a web page.' };
  try {
    return await ask({ type: 'insert', text, url });
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

function told(r: Reply) {
  void browser.notifications.create({
    type: 'basic',
    title: r.ok ? 'Added to your note' : 'Not added',
    message: r.ok ? `Added to “${r.title ?? 'the open note'}”.` : (r.error ?? ''),
    iconUrl: 'icon.svg',
  });
}

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
  const action = (
    browser as unknown as {
      browserAction: {
        setBadgeText(d: { tabId: number; text: string }): Promise<void>;
        setBadgeBackgroundColor(d: { color: string }): Promise<void>;
      };
    }
  ).browserAction;
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
  void action.setBadgeBackgroundColor({ color: '#dc2626' }).catch(() => {});
  void action
    .setBadgeText({ tabId, text: found.length ? String(found.length) : '' })
    .catch(() => {});
  return found;
}

const tabsEvents = browser.tabs as unknown as {
  onActivated: { addListener(f: (i: { tabId: number }) => void): void };
  onUpdated: {
    addListener(f: (id: number, c: { status?: string }, t: { url?: string }) => void): void;
  };
  onRemoved: { addListener(f: (id: number) => void): void };
  get(id: number): Promise<{ url?: string }>;
};
tabsEvents.onActivated.addListener(({ tabId }) => {
  void tabsEvents.get(tabId).then((t) => pageNotes(tabId, t.url ?? ''));
});
tabsEvents.onUpdated.addListener((id, change, t) => {
  if (change.status === 'complete') void pageNotes(id, t.url ?? '');
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
omnibox.setDefaultSuggestion({ description: 'Search your formicaria notes' });
let lastHits: { id: string; title: string }[] = [];
omnibox.onInputChanged.addListener((text, suggest) => {
  void config()
    .then((c) => call(c.server, c.key, 'search', JSON.stringify({ query: text }), true))
    .then((hits: { id: string; title?: string }[] | null) => {
      lastHits = (hits ?? []).slice(0, 8).map((h) => ({ id: h.id, title: h.title || 'Untitled' }));
      suggest(lastHits.map((h) => ({ content: `note:${h.id}`, description: h.title })));
    })
    .catch(() => suggest([]));
});
omnibox.onInputEntered.addListener((text) => {
  const id = text.startsWith('note:') ? text.slice(5) : lastHits[0]?.id;
  if (!id) return;
  void config().then((c) => show(c.server, id));
});

browser.runtime.onMessage.addListener((m) => {
  const d = m as { type?: string; where?: Where; p?: Pending };
  if (d?.type === 'download-choice' && d.where && d.p) return choose(d.where, d.p);
  if (m?.type === 'keep' && typeof m.tab === 'number') return keep(m.tab);
  if (m?.type === 'show' && m.server && m.id) return show(m.server, m.id);
  if (m?.type === 'server') return config().then((c) => ({ server: c.server }));
  if (m?.type === 'notes' && typeof m.tab === 'number') {
    const tab = m.tab;
    return tabsEvents.get(tab).then((t) => pageNotes(tab, t.url ?? ''));
  }
  return undefined;
});

// Once per install: the menu outlives the (non-persistent) background page.
browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.create({
    id: 'formicaria-keep',
    title: 'Keep in formicaria',
    contexts: ['page', 'selection', 'link', 'image'],
  });
  browser.contextMenus.create({
    id: 'formicaria-add',
    title: 'Add to the open formicaria note',
    contexts: ['page', 'selection', 'link'],
  });
  browser.contextMenus.create({
    id: 'formicaria-bookmark',
    title: 'Bookmark in formicaria',
    contexts: ['page'],
  });
});
browser.contextMenus.onClicked.addListener((info, tab) => {
  if (tab?.id == null) return;
  const i = info as {
    menuItemId?: string;
    selectionText?: string;
    linkUrl?: string;
    linkText?: string;
    pageUrl?: string;
  };
  const which = i.menuItemId;
  if (which === 'formicaria-add') {
    const t = tab as { url?: string; title?: string };
    const url = t.url ?? i.pageUrl ?? '';
    const text = passage({
      selection: i.selectionText,
      title: t.title,
      url,
      link: i.linkUrl,
      linkText: i.linkText,
    });
    void addToNote(text, url).then(told);
    return;
  }
  const run =
    which === 'formicaria-bookmark'
      ? bookmark(tab as { url?: string; title?: string })
      : keep(tab.id);
  void run.then((k) =>
    browser.notifications.create({
      type: 'basic',
      title: k.ok ? 'Kept in formicaria' : 'Not kept',
      message: k.ok
        ? `${k.title} — kept as ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}.`
        : (k.error ?? ''),
      iconUrl: 'icon.svg',
    }),
  );
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

/** **Alt+Shift+K keeps the page; Alt+Shift+A adds the selection to the open note.** (Alt+Shift+F,
 *  formicaria's panel, belongs to the browser itself: `formicaria.cfg`.) */
more.commands.onCommand.addListener((c) => {
  if (c === 'add-to-note') {
    void browser.tabs.query({ active: true, currentWindow: true }).then(async ([t]) => {
      if (t?.id == null) return;
      const tt = t as { id: number; url?: string; title?: string };
      const [selection] = (await browser.tabs
        .executeScript(tt.id, { code: 'String(getSelection() ?? "")' })
        .catch(() => [''])) as string[];
      void addToNote(passage({ selection, title: tt.title, url: tt.url }), tt.url ?? '').then(told);
    });
    return;
  }
  if (c !== 'keep-page') return;
  void browser.tabs.query({ active: true, currentWindow: true }).then(async ([t]) => {
    if (t?.id == null) return;
    const k = await keep(t.id);
    void browser.notifications.create({
      type: 'basic',
      title: k.ok ? 'Kept in formicaria' : 'Not kept',
      message: k.ok
        ? `${k.title} — kept as ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}.`
        : (k.error ?? ''),
      iconUrl: 'icon.svg',
    });
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
