// Back walks what you opened — `decisions.md` 2026-10-09, *Back walks what you opened, and panels
// close first*.
//
// Every navigation (a view or note shown) pushes **one** browser-history entry carrying where you
// were; Back (Android's key, the browser's button, the mouse's back button, iOS swipe-back) pops it
// and `App` puts that place back. Panels and menus are **layers**: opening one pushes an entry, and
// Back closes the topmost layer before it moves navigation. At the first screen there is nothing
// left, so Back leaves the app — what Android's `AppPlugin` does once the WebView has no history.
//
// **Recorded synchronously, where the move happens** (`App`'s `focusOrOpen`, `closePane`, …), not in
// an effect. `NotePanel` keeps two entries of its own (editing, full-screen board) and pops them in
// its own effects; if a navigation lands while one of those is on top, it **replaces** it rather
// than stacking. That way NotePanel's check — "is my entry still on top?" — answers no, and its
// `history.back()` never pops the place just opened.

export type Target = {
  kind: string;
  noteId?: string | null;
  viewName?: string | null;
};

type State = { fm: 1; nav: Target; layer?: string };

function same(a: Target | undefined, b: Target): boolean {
  return (
    !!a &&
    a.kind === b.kind &&
    (a.noteId ?? null) === (b.noteId ?? null) &&
    (a.viewName ?? null) === (b.viewName ?? null)
  );
}

function current(): State | null {
  try {
    const s = history.state as State | null;
    return s && s.fm === 1 ? s : null;
  } catch {
    return null;
  }
}

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

/** Stamp the first screen, so Back from the next one has somewhere to land. */
export function start(t: Target) {
  try {
    history.replaceState({ fm: 1, nav: t } satisfies State, '');
  } catch {
    /* no history in this environment — Back simply leaves */
  }
}

/** A view or note is now showing. Records it, unless it is where we already are. */
export function visit(t: Target) {
  if (restoring) return;
  const top = current();
  if (top && !top.layer && same(top.nav, t)) return;
  try {
    const raw = history.state as Record<string, unknown> | null;
    // One of NotePanel's own entries (editing, full-screen board) is on top: replace it.
    const transient = !!raw && ('fmEditing' in raw || 'boardFull' in raw);
    // **A menu's entry is on top: replace it too.** Picking a place from a menu (the phone's window
    // list, the views menu) closes the menu *and* moves. Pushed on top of the menu's entry, the move
    // would leave that entry beneath it, and the menu's own close would then step back onto the place
    // just left — which is exactly the v0.6.2 bug: on the phone, picking a window did nothing.
    if (transient || top?.layer) history.replaceState({ fm: 1, nav: t } satisfies State, '');
    else history.pushState({ fm: 1, nav: t } satisfies State, '');
    // Its entry is gone, so the layer has nothing to take back when its owner closes it.
    layers.length = 0;
  } catch {
    /* no history — nothing to record */
  }
}

// ── layers ───────────────────────────────────────────────────────────────────────────────────

type Layer = { name: string; close: () => void };
const layers: Layer[] = [];

/** A panel or menu opened: Back closes it first. Idempotent per name. */
export function openLayer(name: string, close: () => void) {
  if (layers.some((l) => l.name === name)) return;
  layers.push({ name, close });
  try {
    const top = current();
    history.pushState({ fm: 1, nav: top?.nav ?? { kind: '' }, layer: name } satisfies State, '');
  } catch {
    /* no history */
  }
}

/** A panel or menu closed by its own ✕ or backdrop: take its entry back if it is still on top. */
export function closeLayer(name: string) {
  const i = layers.findIndex((l) => l.name === name);
  if (i === -1) return;
  layers.splice(i, 1);
  try {
    if (current()?.layer === name) {
      ownBack = true;
      history.back();
    }
  } catch {
    /* nothing to take back */
  }
}

/** Set while the app steps back itself to drop a closed menu's entry. That step must never move the
 *  person anywhere — they closed a menu, they did not press Back. */
let ownBack = false;

/** Back (or forward) landed on `state`. Closes the layers above it; answers where to navigate. */
export function popped(state: unknown): Target | null {
  if (ownBack) {
    ownBack = false;
    return null;
  }
  const s =
    state && typeof state === 'object' && (state as State).fm === 1 ? (state as State) : null;
  // Close every layer that is not the one this entry belongs to, topmost first.
  const keep = s?.layer ?? null;
  for (let i = layers.length - 1; i >= 0; i--) {
    if (layers[i].name === keep) break;
    const [l] = layers.splice(i, 1);
    l.close();
  }
  return s && !s.layer ? s.nav : null;
}

/** For tests. */
export function reset() {
  layers.length = 0;
  restoring = false;
  ownBack = false;
}
