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

/** Show the kept note in formicaria: the tab it is already open in, or a new one. */
export async function show(server: string, id: string) {
  const url = `${server}/#note=${id}`;
  const open = await browser.tabs.query({ url: `${server}/*` });
  const t = open[0];
  if (t?.id != null) {
    await browser.tabs.update(t.id, { url, active: true });
    if (t.windowId != null) await browser.windows.update(t.windowId, { focused: true });
  } else {
    await browser.tabs.create({ url });
  }
}

browser.runtime.onMessage.addListener((m) => {
  const d = m as { type?: string; where?: Where; p?: Pending };
  if (d?.type === 'download-choice' && d.where && d.p) return choose(d.where, d.p);
  if (m?.type === 'keep' && typeof m.tab === 'number') return keep(m.tab);
  if (m?.type === 'show' && m.server && m.id) return show(m.server, m.id);
  return undefined;
});

// Once per install: the menu outlives the (non-persistent) background page.
browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.create({
    id: 'formicaria-keep',
    title: 'Keep in formicaria',
    contexts: ['page', 'selection', 'link', 'image'],
  });
});
browser.contextMenus.onClicked.addListener((_info, tab) => {
  if (tab?.id == null) return;
  void keep(tab.id).then((k) =>
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

/** **Alt+Shift+K keeps the page** (the sidebar's Alt+Shift+F is Firefox's own, from the manifest). */
more.commands.onCommand.addListener((c) => {
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
