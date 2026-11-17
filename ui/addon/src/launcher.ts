// **formicaria's ant: its options** (Settings, Help) and a search of your notes and the web; the
// features come from + (`decisions.md` 2026-11-17, *one frame*). Formerly the panel of its features (`decisions.md` 2026-11-17, *formicaria uses the
// browser*; the owner: *"fm is always there through an icon. If one clicks it they can open new
// features … by selecting one feature they will see a new tab/view"*). Each choice opens one tab: a
// feature's view, a note, a new note, or (only for the web) a web search or address.
declare const browser: {
  runtime: { sendMessage<T>(m: unknown): Promise<T> };
  search: { search(o: { query: string; disposition?: string }): Promise<void> };
  tabs: { create(o: { url: string }): Promise<unknown> };
};

const q = document.getElementById('q') as HTMLInputElement;
const list = document.getElementById('list') as HTMLUListElement;

const open = (address: string, title: string) =>
  void browser.runtime
    .sendMessage({ type: 'open-item', address, title })
    .then(() => window.close());
const web = (query: string) =>
  void browser.search.search({ query, disposition: 'NEW_TAB' }).then(() => window.close());
const go = (url: string) => void browser.tabs.create({ url }).then(() => window.close());
/** A page of formicaria's server, in a tab of its own. */
const openPath = (path: string) =>
  void browser.runtime.sendMessage({ type: 'open-path', path }).then(() => window.close());

function row(label: string, act: () => void, cls = '', mark = '') {
  const li = document.createElement('li');
  const b = document.createElement('button');
  b.type = 'button';
  if (cls) b.className = cls;
  if (mark) {
    const m = document.createElement('span');
    m.className = 'mark';
    m.textContent = mark;
    b.append(m);
  }
  b.append(label);
  b.addEventListener('click', act);
  li.append(b);
  return li;
}
const sep = () => Object.assign(document.createElement('li'), { className: 'sep' });

/** What was typed, as a web address, if it reads as one. */
function address(t: string): string | null {
  if (!t || /\s/.test(t)) return null;
  if (/^https?:\/\//i.test(t)) return t;
  return /^[\w-]+(\.[\w-]+)+(:\d+)?(\/.*)?$/.test(t) ? `https://${t}` : null;
}

let hits: { id: string; title: string }[] = [];
let seq = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

function draw() {
  const t = q.value.trim();
  list.replaceChildren();
  if (!t) {
    // **formicaria's options** (`decisions.md` 2026-11-17, *one frame*): the features come from +.
    list.append(
      row(
        'Save media on this page',
        () => void browser.runtime.sendMessage({ type: 'save-media' }).then(() => window.close()),
        '',
        '⤓',
      ),
    );
    list.append(row('Privacy', () => openPath('/f/privacy'), '', '🛡'));
    list.append(row('Settings', () => openPath('/f/settings'), '', '⚙'));
    list.append(row('Help', () => openPath('/manual/'), '', '?'));
    return;
  }
  for (const h of hits) list.append(row(h.title, () => open(`note:${h.id}`, h.title), '', '✎'));
  const url = address(t);
  if (url) list.append(row(`Go to ${url.replace(/^https:\/\//, '')}`, () => go(url), 'web', '→'));
  list.append(row(`Search the web for “${t}”`, () => web(t), 'web', '⌕'));
}

q.addEventListener('input', () => {
  clearTimeout(timer);
  const t = q.value.trim();
  hits = [];
  draw();
  if (!t) return;
  timer = setTimeout(async () => {
    const mine = ++seq;
    const got = await browser.runtime
      .sendMessage<{ id: string; title: string }[]>({ type: 'search-notes', q: t })
      .catch(() => []);
    if (mine !== seq) return;
    hits = got ?? [];
    draw();
  }, 180);
});
q.addEventListener('keydown', (e) => {
  if (e.key !== 'Enter') return;
  const t = q.value.trim();
  if (!t) return;
  if (hits[0]) return open(`note:${hits[0].id}`, hits[0].title);
  const url = address(t);
  if (url) return go(url);
  web(t);
});
draw();

export {};
