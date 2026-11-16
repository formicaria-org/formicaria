// **The page side of "Bring in"** (`data-sources-research-2026-11-15.md` §11). On every web page, top
// frame only, idle until the person acts:
//
// - **After a selection**, what the person chose in Settings → Browser: `offer` (the starting choice) shows
//   **formicaria's icon, floating right where the pointer let go** (owner, 2026-11-16: *"it should be
//   closer to the selection … not a text but a floating formicaria icon"*), its tooltip naming the
//   note it goes into; `bring` brings every selection in at once (Alt held skips; a double-click on a
//   word never brings); `nothing`.
// - **Snip** (`snip.ts`): Ctrl+Shift+drag, or the ant in the address bar, or Alt+Shift+S.
//
// What happened is said in a bubble on the spot, with Undo. It reads only what the person selected or
// drew, never acts on what the page says, and draws in closed shadow roots the page cannot see into.
import { bubble, snipping, startSnip } from './snip';

declare const browser: {
  storage: {
    local: { get(k: string[]): Promise<Record<string, unknown>> };
    onChanged: {
      addListener(f: (c: Record<string, { newValue?: unknown }>, area: string) => void): void;
    };
  };
  runtime: {
    sendMessage<T>(m: unknown): Promise<T>;
    onMessage: { addListener(f: (m: { type?: string }) => void): void };
  };
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
browser.runtime.onMessage.addListener((m) => {
  if (m?.type === 'snip') startSnip();
});

// formicaria's mark: the ant on its red square (`browser/kit/branding/about-logo.svg`).
const LOGO = `<svg viewBox="0 0 512 512" aria-hidden="true"><rect width="512" height="512" rx="112" fill="#dc2626"/><g fill="none" stroke="#1a1205" stroke-width="22" stroke-linecap="round"><path d="M240 122 C 212 88, 198 76, 178 66"/><path d="M272 122 C 300 88, 314 76, 334 66"/><path d="M232 214 C 188 196, 150 196, 118 214"/><path d="M228 240 C 180 240, 140 252, 110 278"/><path d="M232 266 C 190 286, 156 308, 138 338"/><path d="M280 214 C 324 196, 362 196, 394 214"/><path d="M284 240 C 332 240, 372 252, 402 278"/><path d="M280 266 C 322 286, 356 308, 374 338"/></g><g fill="#1a1205"><circle cx="256" cy="150" r="40"/><circle cx="256" cy="232" r="32"/><ellipse cx="256" cy="346" rx="54" ry="66"/></g></svg>`;

let host: HTMLElement | null = null;
let face: HTMLElement | null = null;

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
  if (!text && !div.querySelector('img, video, audio')) return null;
  return { html: div.innerHTML, text };
}

function inEditable(): boolean {
  const n = getSelection()?.anchorNode;
  const e = n instanceof Element ? n : n?.parentElement;
  return !!e?.closest('input, textarea, [contenteditable=""], [contenteditable="true"]');
}

interface Result {
  ok: boolean;
  title?: string;
  error?: string;
  note?: string;
  inserted?: string;
  alt?: { label: string; token: string };
}

async function bring(s: { html: string; text: string }, at: { x: number; y: number }) {
  const r = await browser.runtime
    .sendMessage<Result>({ type: 'bring-selection', html: s.html, text: s.text })
    .catch((e) => ({ ok: false, error: String(e) }) as Result);
  bubble(at, r ?? { ok: false, error: 'no answer' });
}

/** formicaria's icon, floating where the pointer let go of the selection. */
function chip(at: { x: number; y: number }, s: { html: string; text: string }) {
  if (!host) {
    host = document.createElement('formicaria-chip');
    const root = host.attachShadow({ mode: 'closed' });
    root.innerHTML = `<style>
      :host { all: initial; position: fixed; z-index: 2147483647; }
      button { all: initial; display: block; width: 28px; height: 28px; cursor: pointer; border-radius: 8px;
        box-shadow: 0 2px 8px rgba(0,0,0,.3); transition: transform .12s; }
      button:hover { transform: scale(1.15); }
      svg { display: block; width: 28px; height: 28px; }
    </style><button type="button">${LOGO}</button>`;
    face = root.querySelector('button');
    // Pressing it must not clear the selection it is about to bring.
    host.addEventListener('mousedown', (e) => e.preventDefault());
  }
  const h = host;
  const tip = front ? `Bring into “${front}”` : 'Bring into a new note';
  face?.setAttribute('title', tip);
  face?.setAttribute('aria-label', tip);
  h.onclick = (e) => {
    e.preventDefault();
    e.stopPropagation();
    hide();
    void bring(s, { x: at.x, y: at.y + 4 });
  };
  h.style.left = `${Math.max(4, Math.min(at.x, innerWidth - 36))}px`;
  h.style.top = `${Math.max(4, Math.min(at.y, innerHeight - 36))}px`;
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
  if (snipping() || after === 'nothing' || inEditable()) return hide();
  const s = selected();
  if (!s) return hide();
  const sel = getSelection()!;
  const range = sel.getRangeAt(sel.rangeCount - 1);
  // Right beside where it ended: the pointer, or (from the keyboard) the end of the last line.
  const lines = range.getClientRects();
  const last = lines[lines.length - 1] ?? range.getBoundingClientRect();
  const at =
    e instanceof MouseEvent ? { x: e.clientX + 10, y: e.clientY + 12 } : { x: last.right + 6, y: last.bottom + 4 };
  if (after === 'bring') {
    if (e.altKey || (e instanceof MouseEvent && e.detail === 2)) return;
    flash(range.getBoundingClientRect());
    void bring(s, at);
    return;
  }
  chip(at, s);
}

addEventListener(
  'mousedown',
  (e) => {
    if (host && !e.composedPath().includes(host)) hide();
    // **Ctrl+Shift+drag snips** an area, from where the press began.
    if (e.button === 0 && e.ctrlKey && e.shiftKey && !snipping()) {
      e.preventDefault();
      e.stopPropagation();
      startSnip({ x: e.clientX, y: e.clientY });
    }
  },
  true,
);
addEventListener('mouseup', (e) => {
  if (host && e.composedPath().includes(host)) return;
  setTimeout(() => settled(e), 0);
});
addEventListener('keyup', (e) => {
  if (e.shiftKey || e.key === 'Shift') setTimeout(() => settled(e), 0);
});
addEventListener('scroll', hide, { passive: true });
addEventListener('keydown', (e) => {
  if (e.key === 'Escape') hide();
});

export {};
