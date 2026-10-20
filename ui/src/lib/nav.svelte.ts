// Back walks what you opened — `decisions.md` 2026-10-09, *Back walks what you opened, and panels
// close first*, and 2026-10-20, *Back is one step, everywhere*.
//
// Every navigation (a view or note shown) pushes **one** browser-history entry carrying where you
// were; Back (Android's key, the browser's button, the mouse's back button, iOS swipe-back) pops it
// and `App` puts that place back. Panels, menus and the levels inside them (a Settings category, a
// recipe in the log sheet) are **layers**: opening one pushes an entry, and Back closes the topmost
// layer before it moves navigation. At the first screen there is nothing left, so Back leaves the
// app — what Android's `AppPlugin` does once the WebView has no history.
//
// **This module keeps its own copy of the stack** (`trail`, every entry numbered by `i`), so it
// always knows what Back would land on. The first version knew only the entry on top, and that is
// where the dead Backs came from: a menu closed while a panel opened over it left its entry
// beneath; a layer closed from below its child stepped back once and left its own; a flag for "this
// step back is ours" stayed set when the step never came, and swallowed the person's next Back.
//
// - **Moves are recorded synchronously**, where they happen (`App`'s `focusOrOpen`, `closePane`, …).
// - **Layers are settled once a tick** (`settle`), after the screen has changed: which are open now,
//   which entries the stack holds. A menu closing as a panel opens is then one swap, not a step back
//   and a push racing each other in the browser.
// - **A step back of the app's own** knows where it will land (`expect`); a `popstate` that lands
//   there is ours and moves nobody. Anything else is the person's.
// - **An entry whose layer is already gone is passed over**: Back never lands on nothing.
//
// NotePanel's two entries of its own (editing, full-screen board) go through `pushOwn`/`backIfTop`
// here, so they are on the copy of the stack too.

export type Target = {
  kind: string;
  noteId?: string | null;
  viewName?: string | null;
};

/** `layer` names the layer an entry belongs to; `lid` is that opening of it — a menu opened again
 *  later is another `lid`, so its old entry is never taken for the new one. */
type State = { fm: 1; i: number; nav: Target; layer?: string; lid?: number } & Record<
  string,
  unknown
>;

function same(a: Target | undefined, b: Target): boolean {
  return (
    !!a &&
    a.kind === b.kind &&
    (a.noteId ?? null) === (b.noteId ?? null) &&
    (a.viewName ?? null) === (b.viewName ?? null)
  );
}

const ours = (s: unknown): s is State =>
  !!s && typeof s === 'object' && (s as State).fm === 1 && typeof (s as State).i === 'number';

/** NotePanel's entries: editing a note, a board on the full screen. A move replaces one. */
const transient = (s: State | undefined) => !!s && ('fmEditing' in s || 'boardFull' in s);

// ── the copy of the stack ────────────────────────────────────────────────────────────────────

/** The entries this app made, by position. Unknown below a reload's first entry. */
let trail: (State | undefined)[] = [];
/** Where the person is in `trail`. */
let at = 0;

function write(s: Omit<State, 'i'>, how: 'push' | 'replace'): State {
  const i = how === 'push' ? at + 1 : at;
  const full = { ...s, i } as State;
  try {
    if (how === 'push') history.pushState(full, '');
    else history.replaceState(full, '');
  } catch {
    /* no history in this environment — Back simply leaves */
  }
  if (how === 'push') trail.length = at + 1;
  trail[i] = full;
  at = i;
  return full;
}

const top = (): State | undefined => trail[at];

// ── the app's own steps back ─────────────────────────────────────────────────────────────────

/** Where a step back of the app's own will land, while one is on its way. */
let expect: number | null = null;
let expectTimer: ReturnType<typeof setTimeout> | undefined;
/** What waits for that step to land: a push now would race it in the browser. */
let waiting: (() => void)[] = [];

function stepBack(n: number) {
  if (n <= 0) return;
  expect = at - n;
  clearTimeout(expectTimer);
  // **A step that never lands is forgotten**, rather than left to swallow the person's next Back
  // (the manual's frame adds entries this copy cannot see; a WebView may drop a step).
  expectTimer = setTimeout(landed, 1500);
  try {
    history.go(-n);
  } catch {
    landed();
  }
}

function landed() {
  expect = null;
  clearTimeout(expectTimer);
  const run = waiting;
  waiting = [];
  for (const f of run) f();
}

function whenSteady(f: () => void) {
  if (expect === null) f();
  else waiting.push(f);
}

// ── places ───────────────────────────────────────────────────────────────────────────────────

/** True while `App` is putting a place back from history, so that move is not recorded again. */
let restoring = false;
export function restore(fn: () => void) {
  restoring = true;
  try {
    fn();
  } finally {
    restoring = false;
  }
}

/** Stamp the first screen, so Back from the next one has somewhere to land. A reload keeps the
 *  entry's number, so what is below it stays reachable. */
export function start(t: Target) {
  let now: unknown = null;
  try {
    now = history.state;
  } catch {
    /* no history */
  }
  trail = [];
  at = ours(now) ? now.i : 0;
  write({ fm: 1, nav: t }, 'replace');
}

/** A move made while layer entries are on top waits for `settle`, which knows which stay open. */
let move: Target | null = null;

/** A view or note is now showing. Records it, unless it is where we already are. */
export function visit(t: Target) {
  if (restoring) return;
  move = null;
  const s = top();
  if (s && !s.layer && !transient(s) && same(s.nav, t)) return;
  if (s?.layer || expect !== null) {
    move = t;
    schedule();
    return;
  }
  // One of NotePanel's own entries (editing, full-screen board) is on top: replace it.
  whenSteady(() => write({ fm: 1, nav: t }, transient(top()) ? 'replace' : 'push'));
}

/** The place showing changed **without a move**: rewrite the entry on top instead of adding one.
 *  Stepping between a group's views is this — five taps along its tabs must not be five Backs
 *  (`decisions.md#ui`, 2026-10-11). */
export function replace(t: Target) {
  if (restoring) return;
  const s = top();
  if (s?.layer) return; // a panel's entry is on top; it keeps its own
  write({ ...(s ?? {}), fm: 1, nav: t }, 'replace');
}

/** **Leaving a note is a step back** (`decisions.md#ui`, 2026-10-20). The note's window closed and
 *  `t` is now showing: when the entry beneath the note's is `t`, go back to it rather than push `t`
 *  again — or the next Back reopens the note just closed (or a note just deleted). */
export function leave(noteId: string, t: Target) {
  if (restoring) return;
  let j = at;
  // Past the note's own entries — the note itself, its editing, its board on the full screen — and
  // past the menus over them (the phone's window list stays open while a note in it is closed).
  let menus = false;
  while (j > 0) {
    const s = trail[j];
    if (!s) break;
    if (s.layer) menus = true;
    else if (!transient(s) && !(s.nav.kind === 'note' && s.nav.noteId === noteId)) break;
    j--;
  }
  const below = trail[j];
  if (j < at && below && !below.layer && !transient(below) && same(below.nav, t)) {
    stepBack(at - j);
    // A menu still open gets its entry again, on top of the place it now stands over.
    if (menus) whenSteady(schedule);
    return;
  }
  visit(t);
}

// ── NotePanel's own entries ──────────────────────────────────────────────────────────────────

/** Push an entry of NotePanel's own (`{ fmEditing: token }`, `{ boardFull: true }`). */
export function pushOwn(extra: Record<string, unknown>) {
  whenSteady(() => write({ fm: 1, nav: top()?.nav ?? { kind: '' }, ...extra }, 'push'));
}

/** Take back an entry of NotePanel's own, if it is still on top. */
export function backIfTop(key: string, value: unknown) {
  if (top()?.[key] === value) stepBack(1);
}

/** Is an entry of NotePanel's own still in the stack, at or below where the person is? Back from a
 *  menu opened while editing lands above the editing entry, and must not end the editing. */
export function holds(key: string, value: unknown): boolean {
  // Read where the browser is now, not where `popped` last recorded: NotePanel's own listener may
  // hear the same `popstate` before App's has passed it on.
  let now = at;
  try {
    now = ours(history.state) ? history.state.i : -1;
  } catch {
    /* no history */
  }
  return trail.slice(0, now + 1).some((x) => x?.[key] === value);
}

/** Is an entry of NotePanel's own on top? */
export function onTop(key: string, value: unknown): boolean {
  return top()?.[key] === value;
}

// ── layers ───────────────────────────────────────────────────────────────────────────────────

type Layer = { name: string; lid: number; close: () => void };
/** The layers open now, in the order they opened. */
const layers: Layer[] = [];
let lids = 0;
const openLid = (lid: number | undefined) => layers.some((l) => l.lid === lid);

let scheduled = false;
let settleTimer: ReturnType<typeof setTimeout> | undefined;
/** Settle after the screen has: the effects that open and close layers run in the same tick as
 *  the press that caused them, and only once all have run is it known which stay open. */
function schedule() {
  if (scheduled) return;
  scheduled = true;
  settleTimer = setTimeout(() => {
    scheduled = false;
    whenSteady(settle);
  }, 0);
}

const entry = (nav: Target, l: Layer) => ({ fm: 1 as const, nav, layer: l.name, lid: l.lid });

/** Make the stack hold one entry per open layer, in order, above the place — and record a move
 *  made while layer entries were on top. */
function settle() {
  // The run of layer entries on top of the stack.
  let base = at;
  while (base > 0 && trail[base]?.layer) base--;
  const run = trail.slice(trail[base]?.layer ? base : base + 1, at + 1) as State[];
  const below = new Set(
    trail.slice(0, at + 1 - run.length).flatMap((x) => (x?.lid !== undefined ? [x.lid] : [])),
  );
  // Kept: the open layers' entries from the bottom of the run up, until one is closed. A move
  // keeps none — it takes the place of the run.
  let keep = 0;
  if (!move) while (keep < run.length && openLid(run[keep].lid)) keep++;
  const kept = new Set(run.slice(0, keep).map((x) => x.lid));
  // Then every open layer that has no entry anywhere: these are pushed.
  const fresh = layers.filter((l) => !kept.has(l.lid) && !below.has(l.lid));
  const drop = run.length - keep;
  const m = move;
  move = null;
  const nav = () => m ?? top()?.nav ?? { kind: '' };
  const add = (from: number) => {
    for (const l of fresh.slice(from)) write(entry(nav(), l), 'push');
  };
  if (drop === 0) {
    if (m) write({ fm: 1, nav: m }, transient(top()) ? 'replace' : 'push');
    return add(0);
  }
  // **A hand-over** — one layer's entry goes as a move or another layer comes: rewrite the entry on
  // top instead of stepping back and pushing, which the browser may not do in that order.
  if (drop === 1 && (m || fresh.length)) {
    if (m) {
      write({ fm: 1, nav: m }, 'replace');
      return add(0);
    }
    write(entry(nav(), fresh[0]), 'replace');
    return add(1);
  }
  if (m) {
    // Several layer entries under a move: back to the lowest of them, which the move rewrites.
    stepBack(drop - 1);
    whenSteady(() => {
      write({ fm: 1, nav: m }, 'replace');
      add(0);
    });
    return;
  }
  stepBack(drop);
  whenSteady(() => add(0));
}

/** A panel, menu or level opened: Back closes it first. Idempotent per name. */
export function openLayer(name: string, close: () => void) {
  if (layers.some((l) => l.name === name)) return;
  layers.push({ name, lid: ++lids, close });
  schedule();
}

/** A panel, menu or level closed by its own ✕, backdrop or button: its entry goes, and with it the
 *  entries of any layer opened from it. */
export function closeLayer(name: string) {
  const i = layers.findIndex((l) => l.name === name);
  if (i === -1) return;
  // Closing a level closes what was opened from it — a category's sub-screen with the category.
  // **Not what opened in the same moment**: a menu closing as the panel it chose opens is a
  // hand-over, and that panel has no entry yet. Only a layer with an entry was opened from this one.
  const above = layers.splice(i).slice(1);
  const pushed = new Set(trail.flatMap((x) => (x?.lid !== undefined ? [x.lid] : [])));
  layers.push(...above.filter((l) => !pushed.has(l.lid)));
  for (const l of above.filter((l) => pushed.has(l.lid)).reverse()) l.close();
  schedule();
}

/** **A level inside something already open** — a Settings category, a recipe in the log sheet —
 *  as one line in a component: open while `isOpen()`, and `close` takes it up one level. Closed with
 *  the component. */
export function level(name: () => string, open: () => boolean, close: () => void) {
  $effect(() => {
    const n = name();
    if (open()) openLayer(n, close);
    else closeLayer(n);
  });
  $effect(() => {
    const n = name();
    return () => closeLayer(n);
  });
}

/** Back (or forward) landed on `state`. Closes the layers above it; answers where to navigate. */
export function popped(state: unknown): Target | null {
  if (!ours(state)) {
    // Not an entry of this app's: the person went back past everything it made. Nothing to put
    // back, and nothing of the app's is under them any more.
    at = -1;
    if (expect !== null) landed();
    for (const l of layers.splice(0).reverse()) l.close();
    return null;
  }
  const was = at;
  at = state.i;
  trail[at] = state;
  if (expect !== null) {
    const mine = state.i === expect;
    landed();
    if (mine) return null;
  }
  // Close every layer whose entry is above the one landed on, topmost first. (One opened this very
  // moment, whose entry is not pushed yet, stays.)
  const still = new Set(
    trail.slice(0, at + 1).flatMap((x) => (x?.lid !== undefined ? [x.lid] : [])),
  );
  const pushed = new Set(trail.flatMap((x) => (x?.lid !== undefined ? [x.lid] : [])));
  for (let k = layers.length - 1; k >= 0; k--) {
    if (still.has(layers[k].lid) || !pushed.has(layers[k].lid)) continue;
    const [l] = layers.splice(k, 1);
    l.close();
  }
  // **An entry whose layer is gone is passed over**, in the direction the person went.
  if (state.lid !== undefined && !openLid(state.lid)) {
    try {
      if (at < was) history.back();
      else history.forward();
    } catch {
      /* nowhere further */
    }
    return null;
  }
  return state.layer || transient(state) ? null : state.nav;
}

/** For tests: wait until nothing is settling and no step of the app's own is on its way — a step
 *  the browser has been asked for cannot be called off, and would land in the next test. */
export async function idle(ms = 2000) {
  const end = Date.now() + ms;
  while ((scheduled || expect !== null) && Date.now() < end)
    await new Promise((r) => setTimeout(r, 10));
}

/** For tests. */
export function reset() {
  layers.length = 0;
  restoring = false;
  move = null;
  scheduled = false;
  clearTimeout(settleTimer);
  waiting = [];
  expect = null;
  clearTimeout(expectTimer);
  trail = [];
  at = 0;
}
