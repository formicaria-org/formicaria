// **Pick**: for what a selection cannot reach (an image in a carousel, a table, a video). Injected only
// when the person asks (right-click → *Pick from this page…*), and gone when they leave it. The block
// under the pointer lights up with what it is; the wheel (or two fingers on a touchpad) widens it to
// its parent or narrows it back; a click brings it in, Shift+click brings it and goes on picking; Esc
// stops. Drawn in a closed shadow root; reads only the block the person points at.
declare const browser: { runtime: { sendMessage(m: unknown): Promise<unknown> } };

(() => {
  const w = window as unknown as { __formicariaPick?: () => void };
  if (w.__formicariaPick) return w.__formicariaPick();

  const host = document.createElement('formicaria-pick');
  const root = host.attachShadow({ mode: 'closed' });
  root.innerHTML = `<style>
    :host { all: initial; }
    .box { position: fixed; z-index: 2147483646; pointer-events: none; border: 2px solid #dc2626;
      background: rgba(220,38,38,.08); border-radius: 3px; }
    .tag { position: fixed; z-index: 2147483647; pointer-events: none; font: 12px/1 system-ui, sans-serif;
      color: #fff; background: #dc2626; padding: 3px 6px; border-radius: 3px; }
    .bar { position: fixed; z-index: 2147483647; left: 50%; top: 8px; transform: translateX(-50%);
      font: 13px/1.3 system-ui, sans-serif; color: #fff; background: #1d1d22; padding: 6px 12px;
      border-radius: 999px; box-shadow: 0 2px 8px rgba(0,0,0,.25); pointer-events: none; }
  </style><div class="box"></div><div class="tag"></div><div class="bar">Click what to bring in · scroll to widen or narrow · Shift+click for more · Esc to stop</div>`;
  const box = root.querySelector('.box') as HTMLElement;
  const tag = root.querySelector('.tag') as HTMLElement;
  document.documentElement.append(host);

  let base: Element | null = null;
  let up = 0;

  const kind = (el: Element): string => {
    const t = el.tagName;
    if (t === 'IMG' || t === 'PICTURE' || t === 'SVG') return 'image';
    if (t === 'VIDEO') return 'video';
    if (t === 'AUDIO') return 'audio';
    if (t === 'A') return 'link';
    if (t === 'TABLE') return 'table';
    if (t === 'PRE' || t === 'CODE') return 'code';
    if (t === 'UL' || t === 'OL') return 'list';
    if (/^H[1-6]$/.test(t)) return 'heading';
    if (t === 'P' || t === 'BLOCKQUOTE') return 'paragraph';
    if (t === 'FIGURE') return 'figure';
    return 'section';
  };
  const target = (): Element | null => {
    let el = base;
    for (let i = 0; i < up && el?.parentElement && el.parentElement !== document.body; i++)
      el = el.parentElement;
    return el;
  };
  const draw = () => {
    const el = target();
    if (!el) return;
    const r = el.getBoundingClientRect();
    Object.assign(box.style, {
      left: `${r.left}px`,
      top: `${r.top}px`,
      width: `${r.width}px`,
      height: `${r.height}px`,
    });
    tag.textContent = kind(el);
    Object.assign(tag.style, { left: `${r.left}px`, top: `${Math.max(0, r.top - 20)}px` });
  };
  const move = (e: MouseEvent) => {
    host.style.display = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    host.style.display = '';
    if (el && el !== base) {
      base = el;
      up = 0;
    }
    draw();
  };
  const wheel = (e: WheelEvent) => {
    e.preventDefault();
    up = Math.max(0, up + (e.deltaY < 0 ? 1 : -1));
    draw();
  };
  const piece = (el: Element) => {
    const media = el.matches('img, video, audio') ? el : el.querySelector('img, video, audio');
    const only = media && !(el.textContent ?? '').trim();
    if (el.matches('video, audio') || (only && media!.matches('video, audio'))) {
      const m = (el.matches('video, audio') ? el : media) as HTMLMediaElement;
      return {
        kind: m.tagName === 'VIDEO' ? 'video' : 'audio',
        src: m.currentSrc || m.src,
        time: m.currentTime,
      };
    }
    if (el.matches('img') || (only && media!.matches('img'))) {
      const i = (el.matches('img') ? el : media) as HTMLImageElement;
      return { kind: 'image', src: i.currentSrc || i.src, alt: i.alt };
    }
    if (el.matches('a[href]')) {
      return {
        kind: 'link',
        src: (el as HTMLAnchorElement).href,
        alt: (el.textContent ?? '').trim(),
      };
    }
    return { kind: 'text', html: el.outerHTML, text: (el.textContent ?? '').trim() };
  };
  const click = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const el = target();
    if (!el) return;
    void browser.runtime.sendMessage({ type: 'bring-piece', piece: piece(el) });
    box.style.background = 'rgba(220,38,38,.3)';
    setTimeout(() => (box.style.background = ''), 400);
    if (!e.shiftKey) stop();
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Escape') stop();
  };
  const stop = () => {
    removeEventListener('mousemove', move, true);
    removeEventListener('wheel', wheel, true);
    removeEventListener('click', click, true);
    removeEventListener('keydown', key, true);
    host.remove();
    delete w.__formicariaPick;
  };
  addEventListener('mousemove', move, true);
  addEventListener('wheel', wheel, { capture: true, passive: false });
  addEventListener('click', click, true);
  addEventListener('keydown', key, true);
  w.__formicariaPick = stop;
})();
export {};
