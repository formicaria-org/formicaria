// **A swipe steps between a group's views on a touch screen** (the owner, 2026-10-29: *"a swipe left
// right in a view can be used to switch between subviews on the phone"*; `decisions.md#ui`).
//
// 2026-08-30 left it out because a board's columns already scroll sideways on a phone, and two
// sideways gestures over one place make both feel broken. So a swipe here never starts on anything
// that answers a sideways move itself: a box that scrolls sideways (the board's columns, a strip of
// chips), a field, the note's text, or anything marked `data-no-swipe`. Touch only: a mouse has its
// tabs.
//
// **The view follows the finger** (the owner, 2026-11-04: *"it is almost as if we are suddenly on the
// new one"*; `decisions.md#ui`). A pane draws one view and each screen fetches its own data, so no
// neighbour is drawn to drag in: its name comes in beside the view, as a page would.
//
// **Nothing stops on the way** (the owner, 2026-11-05: *"an awkward pause with the name of the next
// subview in the center before reaching it, it should be fluid"*). On release the view leaves at an
// even speed instead of slowing into a stop; the next view then comes on in the same direction, and
// the name carries on through the centre and fades as it comes. Its data is asked for at the first
// touch, so it is normally there; only when it is not does the name wait.
//
// Past either end the view gives a little and comes back. A view that arrives some other way (its
// tab, Back) slides in from the side it lies on.
// Everything moved is the direct children of the element, but for those marked `data-swipe-still`
// (its tabs), and only while it moves: a transform left on would break anything `position: fixed`
// inside. With motion off there is none of this, and the step is taken at once.
import { dur, still } from './charts/motion';

const FAR = 60;
const QUICK_MS = 700;
/** How far across, as a share of the view's width, a slow drag has to go. */
const SLOW_SHARE = 0.35;
/** Px/ms back towards the start at the release: the person changed their mind. */
const CHANGED_MIND = 0.3;
/** How far a sideways move has to go before it is ours (and a vertical one before it is not). */
const CLAIM = 10;
/** How much of the finger's move the view follows past either end. */
const GIVE = 0.25;
/** The longest a view waits off to the side for what comes next, before it comes back. */
const WAIT_MS = 2500;
/** How long a view swiped to takes to come on, and how: quick at first, slowing into place. */
const ARRIVE_MS = 220;
const ARRIVE_EASE = 'cubic-bezier(0.2, 0.6, 0.2, 1)';

export type SwipeOptions = {
  /** A swipe to the left: the next view. */
  onnext?: () => void;
  /** A swipe to the right: the previous view. */
  onprev?: () => void;
  enabled?: boolean;
  /** Which pane this is: its element may be built anew when its view changes. */
  key?: string;
  /** The next view's name, to come in beside this one; `null` when this is the last. */
  next?: string | null;
  /** The previous view's name; `null` when this is the first. */
  prev?: string | null;
  /** Which view is drawn now, so a change of it can arrive from its side. */
  shown?: string;
  /** The group's views, in order: which side a view lies on. */
  order?: string[];
  /** A touch has begun that may become a swipe: what comes next can get ready. */
  onbegin?: () => void;
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

/**
 * Whether a released move steps to the next view. It must be 60 px across and, unless it was already
 * taken as sideways, twice as far across as down. Then either quick, or a third of the way over; and
 * not on its way back to where it began.
 */
export function steps(m: {
  dx: number;
  dy: number;
  ms: number;
  /** Px/ms across at the release. */
  speed: number;
  width: number;
  sideways: boolean;
}): boolean {
  if (Math.abs(m.dx) < FAR) return false;
  if (!m.sideways && Math.abs(m.dx) < 2 * Math.abs(m.dy)) return false;
  if (m.speed * m.dx < 0 && Math.abs(m.speed) > CHANGED_MIND) return false;
  return m.ms <= QUICK_MS || (m.width > 0 && Math.abs(m.dx) >= m.width * SLOW_SHARE);
}

/** How long a view takes over what is left of its way: at about the finger's speed, within reason. */
export function settleMs(left: number, speed: number): number {
  const at = Math.abs(speed) > 0.05 ? Math.abs(left / speed) : Infinity;
  return Math.round(Math.min(220, Math.max(120, at)));
}

/** What each pane showed last, kept here because its element may not outlive the change. */
const lastShown = new Map<string, string>();
/** The panes whose next view was swiped to: which way it is moving, and the name in the gap. */
const swipedTo = new Map<string, { dir: 1 | -1; name: string }>();

export function swipe(node: HTMLElement, options: SwipeOptions = {}) {
  let opts = options;
  let start: { x: number; y: number; at: number } | null = null;
  let sideways = false;
  let last = { x: 0, at: 0, speed: 0 };
  let peek: HTMLElement | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const set = (x: number, fade = 1) => {
    node.style.setProperty('--swipe-x', `${x}px`);
    node.style.setProperty('--swipe-fade', String(fade));
  };
  const rest = () => {
    clearTimeout(timer);
    timer = undefined;
    node.classList.remove('swipe-moving', 'swipe-settling', 'swipe-arriving');
    for (const name of [
      '--swipe-x',
      '--swipe-fade',
      '--swipe-ms',
      '--swipe-dir',
      '--swipe-from',
      '--swipe-ease',
    ])
      node.style.removeProperty(name);
    peek?.remove();
    peek = null;
  };
  /** Move to `x` over `ms`, then do `then`. `ease` for a move that must not slow down at its end. */
  const settle = (x: number, fade: number, ms: number, then: () => void, ease?: string) => {
    node.style.setProperty('--swipe-ms', `${ms}ms`);
    if (ease) node.style.setProperty('--swipe-ease', ease);
    node.classList.add('swipe-settling');
    set(x, fade);
    clearTimeout(timer);
    timer = setTimeout(then, ms);
  };
  /** The next view's name, as an element of this view. */
  const showName = (name: string, dir: 1 | -1) => {
    node.style.setProperty('--swipe-dir', String(dir));
    if (peek) return;
    peek = document.createElement('div');
    peek.className = 'swipe-peek';
    peek.dataset.swipeStill = '';
    peek.setAttribute('aria-hidden', 'true');
    peek.textContent = name;
    peek.style.top = `${window.innerHeight * 0.4 - node.getBoundingClientRect().top + node.scrollTop}px`;
    node.append(peek);
  };
  const follow = (dx: number) => {
    const go = dx < 0 ? opts.next : opts.prev;
    node.classList.add('swipe-moving');
    if (go === null) return set(dx * GIVE);
    if (go) showName(go, dx < 0 ? 1 : -1);
    set(dx);
  };

  /** The view changed: bring it in, from its side unless its name already came from there. */
  const arrive = () => {
    const key = opts.key;
    const shown = opts.shown;
    if (!key || !shown || !opts.order?.length) return;
    const before = lastShown.get(key);
    lastShown.set(key, shown);
    if (!before || before === shown) return;
    const swiped = swipedTo.get(key);
    swipedTo.delete(key);
    if (still()) return;
    const from = opts.order.indexOf(before);
    const to = opts.order.indexOf(shown);
    // From another group: no side for it to come from.
    if (from < 0 || to < 0) return;
    rest();
    // An animation and not a transition: what arrives is often new elements, and a transition does
    // not run on an element that has only just been made.
    const ms = dur(swiped ? ARRIVE_MS : 200);
    const side = swiped
      ? swiped.dir * Math.min(72, node.clientWidth * 0.2)
      : Math.sign(to - from) * Math.min(56, node.clientWidth * 0.15);
    node.style.setProperty('--swipe-from', `${side}px`);
    node.style.setProperty('--swipe-ms', `${ms}ms`);
    if (swiped) {
      // Starts quickly, so it takes over from the view that left rather than starting from rest.
      node.style.setProperty('--swipe-ease', ARRIVE_EASE);
      // The name was in the middle of the screen and moving: it goes on, and goes.
      if (swiped.name) {
        showName(swiped.name, swiped.dir);
        peek?.classList.add('swipe-peek-going');
      }
    }
    node.classList.add('swipe-arriving');
    timer = setTimeout(rest, ms);
  };

  const begin = (e: TouchEvent) => {
    start = null;
    sideways = false;
    if (opts.enabled === false || e.touches.length !== 1 || claimed(e.target, node)) {
      // A second finger is not a swipe: what was moving goes back.
      if (node.classList.contains('swipe-moving') && !timer) settle(0, 1, dur(160), rest);
      return;
    }
    if (timer) rest();
    const t = e.touches[0];
    start = { x: t.clientX, y: t.clientY, at: Date.now() };
    last = { x: t.clientX, at: start.at, speed: 0 };
    opts.onbegin?.();
  };
  const end = (e: TouchEvent) => {
    const s = start;
    start = null;
    const t = e.changedTouches[0];
    if (!s || !t) return;
    const dx = t.clientX - s.x;
    const dir = dx < 0 ? 1 : -1;
    const go = dx < 0 ? opts.next : opts.prev;
    const step =
      go !== null &&
      steps({
        dx,
        dy: t.clientY - s.y,
        ms: Date.now() - s.at,
        speed: last.speed,
        width: node.clientWidth,
        sideways,
      });
    const take = () => (dx < 0 ? opts.onnext?.() : opts.onprev?.());
    if (still() || !sideways) {
      if (step) take();
      return;
    }
    if (!step) return settle(0, 1, settleMs(dx, last.speed), rest);
    const width = node.clientWidth;
    settle(
      -dir * width,
      1,
      settleMs(width - Math.abs(dx), last.speed),
      () => {
        if (opts.key) swipedTo.set(opts.key, { dir, name: go ?? '' });
        timer = setTimeout(() => {
          if (opts.key) swipedTo.delete(opts.key);
          settle(0, 1, dur(200), rest);
        }, WAIT_MS);
        take();
      },
      'linear',
    );
  };
  const cancel = () => {
    start = null;
    if (sideways) settle(0, 1, dur(160), rest);
  };
  // **A sideways move that is ours is claimed**, so the browser does not act on it too: a phone
  // browser's own swipe to go back fired on the same right swipe and left the group (seen in a
  // phone-sized Chrome, 2026-10-29). A vertical move is never claimed: the page scrolls.
  const move = (e: TouchEvent) => {
    const t = e.touches[0];
    if (!start || !t) return;
    const dx = t.clientX - start.x;
    const dy = t.clientY - start.y;
    if (!sideways && Math.abs(dy) > CLAIM && Math.abs(dx) <= 2 * Math.abs(dy)) {
      start = null;
      return;
    }
    if (!sideways && Math.abs(dx) > CLAIM && Math.abs(dx) > 2 * Math.abs(dy)) sideways = true;
    if (!sideways) return;
    if (e.cancelable) e.preventDefault();
    const now = Date.now();
    if (now > last.at)
      last = { x: t.clientX, at: now, speed: (t.clientX - last.x) / (now - last.at) };
    if (!still()) follow(dx);
  };
  /** The browser pans up and down and zooms here; across is ours. Where something inside scrolls
   *  sideways it keeps its own: it is the box that scrolls which answers for that gesture. */
  const claimAcross = () => {
    node.style.touchAction = opts.enabled === false ? '' : 'pan-y pinch-zoom';
  };
  node.addEventListener('touchstart', begin, { passive: true });
  node.addEventListener('touchmove', move, { passive: false });
  node.addEventListener('touchend', end, { passive: true });
  node.addEventListener('touchcancel', cancel, { passive: true });
  claimAcross();
  arrive();
  return {
    update(next: SwipeOptions = {}) {
      opts = next;
      claimAcross();
      arrive();
    },
    destroy() {
      rest();
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
