// **The app's end of the browser's bridge** (`decisions.md` 2026-11-15, late night). formicaria's
// browser add-on reaches the app open in the browser's formicaria panel through a content script
// (`ui/addon/src/bridge.ts`) that posts into this window. A message is accepted **only when this
// window posted it** (`event.source === window`, same origin): a page formicaria opened holds a
// reference to this window and could post to it, but its messages come from its own window and are
// dropped, so no web page can write into a note.
//
// Two requests: `open` a note, and `insert` a passage into the note that is open. An insert is handed
// to the note's own pane (`NotePanel`), which adds it to its text and saves it the way typing does,
// so it lands in the editor, in the undo history and through the same conflict checks.

export interface Answer {
  ok: boolean;
  title?: string;
  error?: string;
}
export interface Insert {
  id: string;
  text: string;
  done: (a: Answer) => void;
}

/** The insert waiting for its note's pane to take it. */
export const inbox = $state<{ insert: Insert | null }>({ insert: null });

export interface Handlers {
  open(id: string): void;
  /** The note an insert goes to: the one in front, or `null` when no note is open. */
  target(): { id: string; title: string } | null;
}

const ID = /^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/;

/** Whether `e` is a request the app should act on (exported for its test). */
export function accepted(
  e: { source: unknown; origin: string; data: unknown },
  self: unknown,
  origin: string,
) {
  if (e.source !== self || e.origin !== origin) return null;
  const m = (e.data as { formicaria_bridge?: unknown } | null)?.formicaria_bridge as
    { n?: unknown; type?: unknown; id?: unknown; text?: unknown } | undefined;
  if (!m || typeof m.n !== 'number') return null;
  if (m.type === 'open' && typeof m.id === 'string' && ID.test(m.id))
    return { n: m.n, type: 'open' as const, id: m.id.toUpperCase() };
  if (
    m.type === 'insert' &&
    typeof m.text === 'string' &&
    m.text.trim() &&
    m.text.length <= 100_000
  )
    return { n: m.n, type: 'insert' as const, text: m.text };
  return null;
}

export function listen(h: Handlers): () => void {
  const reply = (n: number, a: Answer) =>
    window.postMessage({ formicaria_reply: { n, ...a } }, location.origin);
  const onMessage = (e: MessageEvent) => {
    const m = accepted(e, window, location.origin);
    if (!m) return;
    if (m.type === 'open') {
      h.open(m.id);
      reply(m.n, { ok: true });
      return;
    }
    const t = h.target();
    if (!t) {
      reply(m.n, {
        ok: false,
        error: 'Open a note in formicaria first (Alt+Shift+F), then add to it.',
      });
      return;
    }
    let answered = false;
    const insert: Insert = {
      id: t.id,
      text: m.text,
      done: (a) => {
        if (answered) return;
        answered = true;
        reply(m.n, { title: t.title, ...a });
      },
    };
    inbox.insert = insert;
    setTimeout(() => {
      if (inbox.insert === insert) inbox.insert = null;
      insert.done({
        ok: false,
        error: 'The note is not on screen in formicaria; open it, then add to it.',
      });
    }, 4000);
  };
  window.addEventListener('message', onMessage);
  return () => window.removeEventListener('message', onMessage);
}

/** `text` added at the end of `body`, after a blank line. */
export function appended(body: string, text: string): string {
  const t = text.trim();
  if (!body.trim()) return `${t}\n`;
  const sep = body.endsWith('\n\n') ? '' : body.endsWith('\n') ? '\n' : '\n\n';
  return `${body}${sep}${t}\n`;
}
