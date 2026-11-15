// The page's ant in the address bar: **Save page** at once (one press), then what it became and a way
// to open it; or, when notes already name the page, those notes first. Below, the one setting of
// "Bring in": what happens after a selection (research §11.3).
import type { Kept, PageNote } from './background';

declare const browser: {
  tabs: { query(q: { active: boolean; currentWindow: boolean }): Promise<{ id?: number }[]> };
  runtime: { sendMessage<T>(m: unknown): Promise<T>; getBackgroundPage?: unknown };
  storage: {
    local: {
      get(k: string[]): Promise<Record<string, unknown>>;
      set(d: Record<string, unknown>): Promise<void>;
    };
  };
};

/** What happens after a selection on a page: offer the chip, bring it in at once, or nothing. */
async function afterSetting() {
  const { after } = await browser.storage.local.get(['after']);
  const now = after === 'bring' || after === 'nothing' ? after : 'offer';
  for (const input of Array.from(document.querySelectorAll<HTMLInputElement>('input[name=after]'))) {
    input.checked = input.value === now;
    input.onchange = () => void browser.storage.local.set({ after: input.value });
  }
}
void afterSetting();

const KIND_WORDS: Record<string, string> = {
  recipe: 'a recipe — its ingredients can be matched in Food',
  paper: 'a paper, with its citation',
  video: 'a video link',
  song: 'a song link',
  article: 'an article',
};

const $ = (id: string) => document.getElementById(id) as HTMLElement;

async function run() {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  if (tab?.id == null) return;
  // **Already in your notes?** Then say where, rather than keeping a second copy unasked.
  const found = await browser.runtime
    .sendMessage<PageNote[]>({ type: 'notes', tab: tab.id })
    .catch(() => [] as PageNote[]);
  if (found?.length) {
    $('head').textContent =
      found.length === 1 ? 'In one of your notes' : `In ${found.length} of your notes`;
    const list = $('notes');
    list.hidden = false;
    const { server } = await browser.runtime
      .sendMessage<{ server?: string }>({ type: 'server' })
      .catch(() => ({}) as { server?: string });
    for (const n of found) {
      const li = document.createElement('li');
      const a = document.createElement('a');
      a.href = '#';
      a.textContent = n.title || 'Untitled';
      a.onclick = async (e) => {
        e.preventDefault();
        await browser.runtime.sendMessage({ type: 'show', server, id: n.id });
        window.close();
      };
      const ctx = document.createElement('div');
      ctx.className = 'muted';
      ctx.textContent = n.context;
      li.append(a, ctx);
      list.append(li);
    }
    const again = $('again') as HTMLButtonElement;
    again.hidden = false;
    again.onclick = () => {
      again.hidden = true;
      list.hidden = true;
      $('head').textContent = 'Saving this page…';
      void keepNow(tab.id!);
    };
    return;
  }
  await keepNow(tab.id);
}

async function keepNow(tabId: number) {
  const k = await browser.runtime.sendMessage<Kept>({ type: 'keep', tab: tabId });
  if (k.ok) {
    $('head').textContent = 'Saved in formicaria';
    $('what').textContent =
      `“${k.title}”, as ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}, with a copy of the page.`;
    const open = $('open') as HTMLButtonElement;
    open.hidden = !k.id || !k.server;
    open.onclick = async () => {
      await browser.runtime.sendMessage({ type: 'show', server: k.server, id: k.id });
      window.close();
    };
    // **The one alternative** (research §11.3, rule 2): just the link, as a bookmark, instead.
    const link = $('link') as HTMLButtonElement;
    link.hidden = !k.id;
    link.onclick = async () => {
      link.hidden = true;
      const b = await browser.runtime.sendMessage<Kept>({ type: 'link-instead', tab: tabId, id: k.id });
      $('what').textContent = b.ok
        ? `Just the link to “${b.title}”, on the Bookmarks shelf.`
        : (b.error ?? 'Not changed.');
    };
  } else {
    $('head').textContent = 'Not saved';
    $('what').textContent = k.error ?? '';
    $('what').className = 'err';
  }
}
void run();
