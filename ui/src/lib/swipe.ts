// **A swipe steps between a group's views on a touch screen** (the owner, 2026-10-29: *"a swipe left
// right in a view can be used to switch between subviews on the phone"*; `decisions.md#ui`).
//
// 2026-08-30 left it out because a board's columns already scroll sideways on a phone, and two
// sideways gestures over one place make both feel broken. So a swipe here never starts on anything
// that answers a sideways move itself: a box that scrolls sideways (the board's columns, a strip of
// chips), a field, the note's text, or anything marked `data-no-swipe`. It must also be clearly
// sideways (twice as far across as down) and long enough (60 px) not to be a tap or a scroll that
// wandered. Touch only: a mouse has its tabs.

const FAR = 60;
const QUICK_MS = 700;

export type SwipeOptions = {
  /** A swipe to the left: the next view. */
  onnext?: () => void;
  /** A swipe to the right: the previous view. */
  onprev?: () => void;
  enabled?: boolean;
};

/** Whether a touch that began at `target` belongs to something that handles sideways moves. */
export function claimed(target: EventTarget | null, root: Element): boolean {
  for (
    let el = target instanceof Element ? target : null;
    el && el !== root.parentElement;
    el = el.parentElement
  ) {
    if (el.matches('input, textarea, select, [contenteditable], [data-no-swipe], .editor'))
      return true;
    if (el.scrollWidth > el.clientWidth + 1) {
      const x = getComputedStyle(el).overflowX;
      if (x === 'auto' || x === 'scroll') return true;
    }
  }
  return false;
}

export function swipe(node: HTMLElement, options: SwipeOptions = {}) {
  let opts = options;
  let start: { x: number; y: number; at: number } | null = null;
  const begin = (e: TouchEvent) => {
    start = null;
    if (opts.enabled === false || e.touches.length !== 1 || claimed(e.target, node)) return;
    const t = e.touches[0];
    start = { x: t.clientX, y: t.clientY, at: Date.now() };
  };
  const end = (e: TouchEvent) => {
    const s = start;
    start = null;
    const t = e.changedTouches[0];
    if (!s || !t || Date.now() - s.at > QUICK_MS) return;
    const dx = t.clientX - s.x;
    const dy = t.clientY - s.y;
    if (Math.abs(dx) < FAR || Math.abs(dx) < 2 * Math.abs(dy)) return;
    if (dx < 0) opts.onnext?.();
    else opts.onprev?.();
  };
  const cancel = () => (start = null);
  // **A sideways move that is ours is claimed**, so the browser does not act on it too: a phone
  // browser's own swipe to go back fired on the same right swipe and left the group (seen in a
  // phone-sized Chrome, 2026-10-29). A vertical move is never claimed: the page scrolls.
  const move = (e: TouchEvent) => {
    const t = e.touches[0];
    if (!start || !t) return;
    const dx = t.clientX - start.x;
    if (Math.abs(dx) > 10 && Math.abs(dx) > 2 * Math.abs(t.clientY - start.y) && e.cancelable)
      e.preventDefault();
  };
  node.addEventListener('touchstart', begin, { passive: true });
  node.addEventListener('touchmove', move, { passive: false });
  node.addEventListener('touchend', end, { passive: true });
  node.addEventListener('touchcancel', cancel, { passive: true });
  return {
    update(next: SwipeOptions = {}) {
      opts = next;
    },
    destroy() {
      node.removeEventListener('touchstart', begin);
      node.removeEventListener('touchmove', move);
      node.removeEventListener('touchend', end);
      node.removeEventListener('touchcancel', cancel);
    },
  };
}

/** The view a step of `by` from `current` lands on in `views`, or `null` past either end. */
export function stepped(
  views: string[],
  current: string | null | undefined,
  by: 1 | -1,
): string | null {
  const at = current ? views.indexOf(current) : -1;
  if (at < 0) return null;
  return views[at + by] ?? null;
}
