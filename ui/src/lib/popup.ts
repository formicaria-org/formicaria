// A Svelte action for **anything that opens from a control** — a menu, a picker, a list of choices, the
// format bar — that keeps it wholly on the screen, on every device.
//
// The owner's standing rule, said "for the nth time" (2026-09-11): no window or option that opens may
// fall outside the visible screen. The popups here broke it each in their own way, so placement lives
// once, in this file, and `ci/checks.sh` refuses a menu or list of choices that does not use it:
//
// - **Clipped by what it sits in.** A pane is a size container (`container-type`), which makes it the
//   containing block even for `position: fixed`, and it clips; a board column scrolls; the touch format
//   bar scrolls sideways. So the popup is lifted into the browser's top layer (the Popover API), which
//   no ancestor can clip or paint over. It stays where it is in the document, so its handlers, its
//   styles and `clickOutside` are untouched. Without the API (iOS before 17, jsdom) it is placed all the
//   same, against whatever contains it — where it can still be clipped.
// - **Placed from a guessed size.** The bottom bar's menus were kept on screen by assuming they were
//   11rem wide, so a long note name in the list of open windows pushed that list off the right edge —
//   reported right after 0.5.6. Here the popup is measured after it renders, and again whenever its
//   content, the screen or a scroll moves things.
// - **Bigger than the room.** Its width is capped at the screen less gutters, and its height at the room
//   beside its control less the safe-area inset (and the navigation-bar floor on touch), scrolling
//   inside beyond that — never looser than a cap its own stylesheet sets.
//
// It opens on the side its caller prefers (below, unless told otherwise) when it fits there, and on the
// side with more room when it does not, with its start or end edge lined up with the control's. The
// control is `anchor`: an element, a box in viewport coordinates, or a function returning one (the
// caret); by default the popup's parent.

export type Box = { top: number; bottom: number; left: number; right: number };

export type PopupOptions = {
  anchor?: HTMLElement | Box | (() => Box) | null;
  align?: 'start' | 'end';
  prefer?: 'below' | 'above';
  /** Space between the control and the popup, in px. */
  gap?: number;
};

const GUTTER = 8;

function boxOf(anchor: PopupOptions['anchor'], node: HTMLElement): Box | null {
  const a = anchor ?? node.parentElement;
  if (!a) return null;
  if (typeof a === 'function') return a();
  if (a instanceof Element) return a.getBoundingClientRect();
  return a;
}

/** A computed length in px, or `fallback` where there is none (`none`, `auto`). */
function length(value: string, fallback: number): number {
  const n = parseFloat(value);
  return Number.isFinite(n) ? n : fallback;
}

export function popup(node: HTMLElement, options: PopupOptions = {}) {
  let opts = options;
  const coarse = window.matchMedia?.('(pointer: coarse)')?.matches ?? false;
  if (typeof node.showPopover === 'function') {
    node.setAttribute('popover', 'manual');
    node.showPopover();
  }

  function place() {
    const r = boxOf(opts.anchor, node);
    if (!r) return;
    const gap = opts.gap ?? 4;
    const vv = window.visualViewport;
    const top0 = vv?.offsetTop ?? 0;
    const left0 = vv?.offsetLeft ?? 0;
    const bottom = vv ? top0 + vv.height : window.innerHeight;
    const right = vv ? left0 + vv.width : window.innerWidth;
    const s = node.style;

    // Its own stylesheet's caps, read with ours cleared.
    s.maxWidth = s.maxHeight = s.minWidth = '';
    const css = getComputedStyle(node);
    const capW = length(css.maxWidth, Infinity);
    const capH = length(css.maxHeight, Infinity);
    const minW = length(css.minWidth, 0);

    // Measured from the corner of whatever contains it: the screen in the top layer, a pane without.
    s.position = 'fixed';
    s.margin = '0';
    s.right = s.bottom = 'auto';
    s.top = s.left = '0px';
    const origin = node.getBoundingClientRect();

    const across = Math.max(0, right - left0 - 2 * GUTTER);
    s.maxWidth = `${Math.min(capW, across)}px`;
    if (minW > across) s.minWidth = `${across}px`;

    // A control partly scrolled off the screen still opens its popup on the screen.
    const aTop = Math.max(top0, Math.min(r.top, bottom));
    const aBottom = Math.max(top0, Math.min(r.bottom, bottom));
    const below = bottom - aBottom - gap;
    const above = aTop - top0 - gap;
    const need = node.offsetHeight;
    const down =
      (opts.prefer ?? 'below') === 'below'
        ? need <= below - GUTTER || below >= above
        : !(need <= above - GUTTER || above >= below);
    const inset = down
      ? `max(var(--safe-bottom, 0px), ${coarse ? 'var(--bar-floor, 0px)' : `${GUTTER}px`})`
      : `max(var(--safe-top, 0px), ${GUTTER}px)`;
    const room = `calc(${Math.max(0, Math.floor(down ? below : above))}px - ${inset})`;
    s.maxHeight = Number.isFinite(capH) ? `min(${capH}px, ${room})` : room;
    s.overflowY = 'auto';

    const h = node.offsetHeight;
    const w = node.offsetWidth;
    const top = down ? aBottom + gap : aTop - gap - h;
    const x = opts.align === 'end' ? r.right - w : r.left;
    const left = Math.max(left0 + GUTTER, Math.min(x, right - GUTTER - w));
    s.top = `${Math.round(top - origin.top)}px`;
    s.left = `${Math.round(left - origin.left)}px`;
    node.dataset.open = down ? 'down' : 'up';
  }

  // Once per frame, however many things moved in it.
  let frame = 0;
  const schedule = () => {
    if (typeof requestAnimationFrame !== 'function') return place();
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(place);
  };
  // A scroll anywhere may move the control — except a scroll inside the popup itself.
  const onScroll = (e: Event) => {
    if (!(e.target instanceof Node && node.contains(e.target))) schedule();
  };

  place();
  const resized = typeof ResizeObserver === 'function' ? new ResizeObserver(schedule) : null;
  resized?.observe(node);
  window.addEventListener('resize', schedule);
  window.visualViewport?.addEventListener('resize', schedule);
  document.addEventListener('scroll', onScroll, true);

  return {
    update(next: PopupOptions = {}) {
      opts = next;
      place();
    },
    destroy() {
      if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(frame);
      resized?.disconnect();
      window.removeEventListener('resize', schedule);
      window.visualViewport?.removeEventListener('resize', schedule);
      document.removeEventListener('scroll', onScroll, true);
      try {
        node.hidePopover();
      } catch {
        /* never shown, or already gone */
      }
    },
  };
}
