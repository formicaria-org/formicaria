// The toolbar button's panel: it keeps the page at once (one press), then says what it became and
// offers to open the note in formicaria.
import type { Kept } from './background';

declare const browser: {
  tabs: { query(q: { active: boolean; currentWindow: boolean }): Promise<{ id?: number }[]> };
  runtime: { sendMessage<T>(m: unknown): Promise<T>; getBackgroundPage?: unknown };
};

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
  const k = await browser.runtime.sendMessage<Kept>({ type: 'keep', tab: tab.id });
  if (k.ok) {
    $('head').textContent = 'Kept in formicaria';
    $('what').textContent =
      `“${k.title}”, as ${KIND_WORDS[k.kind ?? ''] ?? 'a note'}, with a copy of the page.`;
    const open = $('open') as HTMLButtonElement;
    open.hidden = !k.id || !k.server;
    open.onclick = async () => {
      await browser.runtime.sendMessage({ type: 'show', server: k.server, id: k.id });
      window.close();
    };
  } else {
    $('head').textContent = 'Not kept';
    $('what').textContent = k.error ?? '';
    $('what').className = 'err';
  }
}
void run();
