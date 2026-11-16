// **formicaria's ant: the lean panel of its features** (`decisions.md` 2026-11-17, *formicaria uses the
// browser*; the owner: *"fm is always there through an icon. If one clicks it they can open new
// features … by selecting one feature they will see a new tab/view"*). Each choice opens one tab: a
// feature's view, a note, a new note, or (only for the web) a web search or address.
declare const browser: {
  runtime: { sendMessage<T>(m: unknown): Promise<T> };
  storage: { local: { get(k: string[]): Promise<Record<string, unknown>>; set(v: Record<string, unknown>): Promise<void> } };
  search: { search(o: { query: string; disposition?: string }): Promise<void> };
  tabs: { create(o: { url: string }): Promise<unknown> };
};

/** The features, as formicaria's addresses (`App.svelte`'s `openAddress`). */
const FEATURES: [mark: string, label: string, address: string][] = [
  ['✎', 'Notes', 'view:group:Library'],
  ['▦', 'Tasks', 'view:group:Tasks'],
  ['◷', 'Agenda', 'view:agenda'],
  ['◍', 'Food journal', 'view:group:Food'],
  ['◆', 'Fitness', 'view:group:Fitness'],
  ['✚', 'Health', 'view:group:Health'],
  ['♪', 'Music', 'view:group:Music'],
];

const q = document.getElementById('q') as HTMLInputElement;
const list = document.getElementById('list') as HTMLUListElement;

const open = (address: string, title: string) =>
  void browser.runtime.sendMessage({ type: 'open-item', address, title }).then(() => window.close());
const web = (query: string) =>
  void browser.search.search({ query, disposition: 'NEW_TAB' }).then(() => window.close());
const go = (url: string) => void browser.tabs.create({ url }).then(() => window.close());

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

/** **Mail is your webmail, in a tab** (`decisions.md` 2026-11-17): Gmail unless another address is
 *  set here; formicaria brings what you select there into your notes, as from any page. */
let webmail = 'https://mail.google.com/';
let editingMail = false;
void browser.storage.local.get(['webmail']).then((s) => {
  if (typeof s.webmail === 'string' && /^https:\/\//.test(s.webmail)) webmail = s.webmail;
  draw();
});

function draw() {
  const t = q.value.trim();
  list.replaceChildren();
  if (!t) {
    list.append(row('New note', () => open('new:', 'New note'), '', '＋'));
    list.append(sep());
    for (const [mark, label, a] of FEATURES.slice(0, 3)) list.append(row(label, () => open(a, label), '', mark));
    list.append(row('Mail', () => go(webmail), '', '✉'));
    for (const [mark, label, a] of FEATURES.slice(3)) list.append(row(label, () => open(a, label), '', mark));
    list.append(sep());
    list.append(row('Search the web', () => q.focus(), 'web', '⌕'));
    if (editingMail) {
      const li = document.createElement('li');
      const input = Object.assign(document.createElement('input'), {
        type: 'url',
        value: webmail,
        placeholder: 'https://…',
      });
      input.setAttribute('aria-label', 'your webmail address');
      input.addEventListener('keydown', (e) => {
        if (e.key !== 'Enter') return;
        const v = input.value.trim();
        if (/^https:\/\/[^\s]+$/.test(v)) {
          webmail = v;
          void browser.storage.local.set({ webmail: v });
        }
        editingMail = false;
        draw();
      });
      li.append(input);
      list.append(li);
      input.focus();
    } else {
      list.append(
        row(`Mail opens ${webmail.replace(/^https:\/\//, '').replace(/\/$/, '')} — change`, () => {
          editingMail = true;
          draw();
        }, 'muted'),
      );
    }
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
