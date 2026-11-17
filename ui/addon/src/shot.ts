// **Snip where the add-on cannot enter** (a PDF in Firefox's viewer, a page closed to add-ons; owner,
// 2026-11-17: *"think of a pdf opened in the browser tab"*). The background captured what the tab
// showed; this page shows it, the person drags the area, and the background crops it and brings it
// into the note in front as a picture, with where it came from. Then this page closes.
declare const browser: { runtime: { sendMessage(m: unknown): Promise<unknown> } };

interface Shot {
  png: string;
  page: { url: string; title: string };
}
interface Reply {
  ok?: boolean;
  title?: string;
  error?: string;
}

const $ = (id: string) => document.getElementById(id) as HTMLElement;
const img = $('shot') as HTMLImageElement;
const box = $('area');
const say = $('say');
const go = $('bring') as HTMLButtonElement;
let area: { x: number; y: number; width: number; height: number } | null = null;

void (browser.runtime.sendMessage({ type: 'shot-get' }) as Promise<Shot | null>).then((s) => {
  if (!s) {
    say.textContent = 'Nothing to snip: start Snip again from the page.';
    return;
  }
  img.src = s.png;
  document.title = `Snip: ${s.page.title || s.page.url}`;
});

/** Where the pointer is, in the picture's own pixels (it may be shown smaller). */
function at(e: PointerEvent) {
  const r = img.getBoundingClientRect();
  const k = img.naturalWidth / r.width || 1;
  return {
    x: Math.max(0, Math.min(r.width, e.clientX - r.left)) * k,
    y: Math.max(0, Math.min(r.height, e.clientY - r.top)) * k,
    k,
  };
}

let from: { x: number; y: number } | null = null;
$('stage').addEventListener('pointerdown', (e) => {
  e.preventDefault();
  from = at(e);
  (e.target as Element).setPointerCapture?.(e.pointerId);
});
$('stage').addEventListener('pointermove', (e) => {
  if (!from) return;
  const p = at(e);
  area = {
    x: Math.min(from.x, p.x),
    y: Math.min(from.y, p.y),
    width: Math.abs(p.x - from.x),
    height: Math.abs(p.y - from.y),
  };
  Object.assign(box.style, {
    display: 'block',
    left: `${area.x / p.k}px`,
    top: `${area.y / p.k}px`,
    width: `${area.width / p.k}px`,
    height: `${area.height / p.k}px`,
  });
  go.disabled = area.width < 4 || area.height < 4;
});
$('stage').addEventListener('pointerup', () => (from = null));

async function bring() {
  if (!area || go.disabled) return;
  go.disabled = true;
  say.textContent = 'Bringing it in…';
  const r = (await browser.runtime.sendMessage({ type: 'shot-bring', area })) as Reply | null;
  say.textContent = r?.ok
    ? `Into “${r.title ?? 'your note'}”.`
    : `Not brought in: ${r?.error ?? 'no answer'}`;
  if (r?.ok) setTimeout(() => window.close(), 1200);
  else go.disabled = false;
}
go.addEventListener('click', () => void bring());
$('cancel').addEventListener('click', () => window.close());
addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void bring();
  if (e.key === 'Escape') window.close();
});
