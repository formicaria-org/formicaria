// **After a selection, formicaria's choice** (`data-sources-research-2026-11-15.md` §11.3). On every web
// page, top frame only, and idle until the person finishes selecting with the pointer or the keyboard.
// What happens then is the person's setting (the address-bar ant's panel):
//
// - `offer` (the default): a small ant chip beside the selection, naming the note it goes into;
// - `bring`: selecting *is* bringing in (a reading mode, as Readwise Reader highlights on selection);
//   holding Alt while selecting skips it, and a double-click on a word never brings it;
// - `nothing`: the page behaves as in any Firefox.
//
// It reads only the selection the person made, never acts on what the page says, and draws its chip
// in a closed shadow root so the page can neither see inside nor restyle it. A few KB, no libraries:
// the background turns the selection into the note's text.
declare const browser: {
  storage: {
    local: { get(k: string[]): Promise<Record<string, unknown>> };
    onChanged: { addListener(f: (c: Record<string, { newValue?: unknown }>, area: string) => void): void };
  };
  runtime: { sendMessage(m: unknown): Promise<unknown> };
};

type After = 'offer' | 'bring' | 'nothing';
let after: After = 'offer';
let front = '';

void browser.storage.local.get(['after', 'front']).then((s) => {
  if (s.after === 'bring' || s.after === 'nothing') after = s.after;
  front = typeof s.front === 'string' ? s.front : '';
});
browser.storage.onChanged.addListener((c, area) => {
  if (area !== 'local') return;
  if (c.after) after = (c.after.newValue as After) ?? 'offer';
  if (c.front) front = (c.front.newValue as string) ?? '';
});

let host: HTMLElement | null = null;
let label: HTMLElement | null = null;

function hide() {
  host?.remove();
}

/** The selection as the background needs it: its HTML and its words. */
function selected(): { html: string; text: string } | null {
  const sel = getSelection();
  if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
  const text = sel.toString().trim();
  const div = document.createElement('div');
  for (let i = 0; i < sel.rangeCount; i++) div.append(sel.getRangeAt(i).cloneContents());
  const html = div.innerHTML;
  if (!text && !div.querySelector('img, video, audio')) return null;
  return { html, text };
}

function inEditable(): boolean {
  const n = getSelection()?.anchorNode;
  const el = n instanceof Element ? n : n?.parentElement;
  return !!el?.closest('input, textarea, [contenteditable=""], [contenteditable="true"]');
}

function bring(s: { html: string; text: string }) {
  void browser.runtime.sendMessage({ type: 'bring-selection', html: s.html, text: s.text });
}

function chip(rect: DOMRect, s: { html: string; text: string }) {
  if (!host) {
    host = document.createElement('formicaria-chip');
    const root = host.attachShadow({ mode: 'closed' });
    root.innerHTML = `<style>
      :host { all: initial; position: fixed; z-index: 2147483647; }
      button { all: initial; display: flex; align-items: center; gap: 6px; cursor: pointer;
        font: 13px/1.2 system-ui, sans-serif; color: #fff; background: #1d1d22; border-radius: 999px;
        padding: 5px 10px 5px 7px; box-shadow: 0 2px 8px rgba(0,0,0,.25); }
      button:hover { background: #dc2626; }
      svg { width: 16px; height: 16px; }
    </style><button type="button" title="Bring this into formicaria"><svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="6" r="3"/><circle cx="12" cy="12" r="3.4"/><ellipse cx="12" cy="18.5" rx="4" ry="4.6"/></svg><span></span></button>`;
    label = root.querySelector('span');
    // The whole chip is the button, so pressing anywhere on it brings the selection in.
    host.addEventListener('mousedown', (e) => e.preventDefault());
  }
  const h = host;
  h.onclick = (e) => {
    e.preventDefault();
    e.stopPropagation();
    bring(s);
    if (label) label.textContent = 'Brought in';
    setTimeout(hide, 900);
  };
  if (label) label.textContent = front ? `Bring into “${front}”` : 'Bring into a new note';
  const x = Math.min(innerWidth - 220, Math.max(8, rect.right - 20));
  const y = rect.bottom + 8 > innerHeight - 40 ? Math.max(8, rect.top - 36) : rect.bottom + 8;
  h.style.left = `${x}px`;
  h.style.top = `${y}px`;
  document.documentElement.append(h);
}

function flash(rect: DOMRect) {
  const f = document.createElement('formicaria-flash');
  f.style.cssText = `all:initial;position:fixed;z-index:2147483647;pointer-events:none;left:${rect.left}px;top:${rect.top}px;width:${rect.width}px;height:${rect.height}px;background:rgba(220,38,38,.18);border-radius:3px;transition:opacity .6s`;
  document.documentElement.append(f);
  setTimeout(() => (f.style.opacity = '0'), 250);
  setTimeout(() => f.remove(), 900);
}

function settled(e: MouseEvent | KeyboardEvent) {
  if (after === 'nothing' || inEditable()) return hide();
  const s = selected();
  if (!s) return hide();
  const sel = getSelection()!;
  const rect = sel.getRangeAt(sel.rangeCount - 1).getBoundingClientRect();
  if (after === 'bring') {
    // Alt held, or a double-click on a word (looking it up, not keeping it): not brought in.
    if (e.altKey || (e instanceof MouseEvent && e.detail === 2)) return;
    bring(s);
    flash(rect);
    return;
  }
  chip(rect, s);
}

addEventListener('mouseup', (e) => {
  if (host && e.composedPath().includes(host)) return;
  setTimeout(() => settled(e), 0);
});
addEventListener('keyup', (e) => {
  if (e.shiftKey || e.key === 'Shift') setTimeout(() => settled(e), 0);
});
addEventListener('mousedown', (e) => {
  if (host && !e.composedPath().includes(host)) hide();
});
addEventListener('scroll', hide, { passive: true });
addEventListener('keydown', (e) => {
  if (e.key === 'Escape') hide();
});
export {};
