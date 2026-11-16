// **The app's end of the browser's bridge** (`decisions.md` 2026-11-15, late night). formicaria's
// browser add-on reaches the app open in the browser's formicaria panel through a content script
// (`ui/addon/src/bridge.ts`) that posts into this window. A message is accepted **only when this
// window posted it** (`event.source === window`, same origin): a page formicaria opened holds a
// reference to this window and could post to it, but its messages come from its own window and are
// dropped, so no web page can write into a note.
//
// From the add-on: `open` a note; `insert` what was brought in into the note in front (or a new note
// named after the page when none is); `replace` it by its alternative; `toast`, a line to show.
// An insert is handed to the note's own pane (`NotePanel`), which adds it to its text and saves it the
// way typing does, so it lands in the editor and through the same conflict checks. Up to the add-on
// (`up`): the note in front (where *Bring in* goes), an alternative chosen, a drop.

export interface Answer {
  ok: boolean;
  title?: string;
  error?: string;
  note?: string;
}
export interface Insert {
  id: string;
  text: string;
  done: (a: Answer) => void;
}
export interface Replace {
  id: string;
  from: string;
  to: string;
  done: (a: Answer) => void;
}

/** What waits for a note's pane to take it: an insert, a replacement (an alternative, or Undo). */
export const inbox = $state<{ insert: Insert | null; replace: Replace | null }>({
  insert: null,
  replace: null,
});

/** The line formicaria shows after a bring-in or a save: what happened, Undo, the one alternative. */
export interface Toast {
  text: string;
  /** The note it went into, and what went in (for Undo and the alternative). */
  note?: string;
  inserted?: string;
  alt?: { label: string; token: string };
  /** A saved page: its note, to open. */
  open?: string;
}
export const toast = $state<{ now: Toast | null }>({ now: null });

export interface Handlers {
  open(id: string): void;
  /** The note an insert goes to: the one in front, or `null` when no note is open. */
  target(): { id: string; title: string } | null;
  /** Make a note named after the page and open it, for a bring-in with no note in front. */
  create(title: string): Promise<{ id: string; title: string } | null>;
  /** Ask the assistant which parts of a page matter for the note in front (a discussion it answers
   *  with proposals the person accepts). Answers the discussion's id. */
  ask(page: { title: string; url: string }, text: string): Promise<string | null>;
}

const ID = /^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/;

type Accepted =
  | { n: number; type: 'open'; id: string }
  | {
      n: number;
      type: 'insert';
      text: string;
      what: string;
      page: { title: string; url: string };
      alt?: { label: string; token: string };
      /** Said on the page already (the bubble): formicaria shows no second line. */
      quiet: boolean;
      /** **The note chosen on the page** (the chooser by formicaria's icon), else the one in front. */
      note?: string;
      noteTitle?: string;
      /** A new note, named after the page, chosen on the page. */
      fresh?: boolean;
    }
  | { n: number; type: 'replace'; id: string; from: string; to: string }
  | { n: number; type: 'toast'; text: string; id?: string }
  | { n: number; type: 'ask'; text: string; page: { title: string; url: string } };

const str = (v: unknown, max = 100_000) => (typeof v === 'string' && v.length <= max ? v : null);

/** Whether `e` is a request the app should act on (exported for its test). */
export function accepted(
  e: { source: unknown; origin: string; data: unknown },
  self: unknown,
  origin: string,
): Accepted | null {
  if (e.source !== self || e.origin !== origin) return null;
  const m = (e.data as { formicaria_bridge?: unknown } | null)?.formicaria_bridge as
    Record<string, unknown> | undefined;
  if (!m || typeof m.n !== 'number') return null;
  const n = m.n;
  if (m.type === 'open' && typeof m.id === 'string' && ID.test(m.id))
    return { n, type: 'open', id: m.id.toUpperCase() };
  if (m.type === 'insert') {
    const text = str(m.text);
    if (!text?.trim()) return null;
    const page = (m.page ?? {}) as Record<string, unknown>;
    const alt = m.alt as Record<string, unknown> | undefined;
    return {
      n,
      type: 'insert',
      text,
      what: str(m.what, 200) ?? 'it',
      page: { title: str(page.title, 500) ?? '', url: str(page.url, 4000) ?? '' },
      alt:
        alt && str(alt.label, 100) && str(alt.token, 40)
          ? { label: alt.label as string, token: alt.token as string }
          : undefined,
      quiet: m.quiet === true,
      ...(typeof m.note === 'string' && ID.test(m.note) ? { note: m.note.toUpperCase() } : {}),
      ...(str(m.noteTitle, 500) ? { noteTitle: str(m.noteTitle, 500)! } : {}),
      ...(m.fresh === true ? { fresh: true } : {}),
    };
  }
  if (m.type === 'replace' && typeof m.note === 'string' && ID.test(m.note)) {
    const from = str(m.from);
    const to = str(m.to);
    if (from == null || to == null || !from) return null;
    return { n, type: 'replace', id: m.note.toUpperCase(), from, to };
  }
  if (m.type === 'ask') {
    const text = str(m.text, 20_000);
    const page = (m.page ?? {}) as Record<string, unknown>;
    if (!text?.trim()) return null;
    return {
      n,
      type: 'ask',
      text,
      page: { title: str(page.title, 500) ?? '', url: str(page.url, 4000) ?? '' },
    };
  }
  if (m.type === 'toast') {
    const text = str(m.text, 500);
    if (!text) return null;
    const id = typeof m.id === 'string' && ID.test(m.id) ? m.id.toUpperCase() : undefined;
    return { n, type: 'toast', text, id };
  }
  return null;
}

/** Is the app inside formicaria's browser, with its bridge (`ui/addon/src/bridge.ts`)? */
export function bridged(): boolean {
  return (
    typeof document !== 'undefined' &&
    document.documentElement.hasAttribute('data-formicaria-bridge')
  );
}

/** A message of the app's own up to the add-on (through the bridge's content script). */
export function up(m: Record<string, unknown>) {
  window.postMessage({ formicaria_up: m }, location.origin);
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
    if (m.type === 'ask') {
      void h.ask(m.page, m.text).then(
        (id) =>
          reply(
            m.n,
            id ? { ok: true, note: id } : { ok: false, error: 'Open the note it is for first.' },
          ),
        (e) => reply(m.n, { ok: false, error: String(e) }),
      );
      return;
    }
    if (m.type === 'toast') {
      toast.now = { text: m.text, open: m.id };
      reply(m.n, { ok: true });
      return;
    }
    if (m.type === 'replace') {
      inbox.replace = { id: m.id, from: m.from, to: m.to, done: (a) => reply(m.n, a) };
      return;
    }
    void (async () => {
      const fresh = () => h.create(m.page.title || 'From the web');
      // The note chosen on the page is opened, so its pane takes the insert as the one in front would.
      const chosen = m.note
        ? (h.open(m.note), { id: m.note, title: m.noteTitle || 'your note' })
        : null;
      const t = chosen ?? (m.fresh ? await fresh() : (h.target() ?? (await fresh())));
      if (!t) {
        reply(m.n, { ok: false, error: 'formicaria could not make a note for it.' });
        return;
      }
      let answered = false;
      const insert: Insert = {
        id: t.id,
        text: m.text,
        done: (a) => {
          if (answered) return;
          answered = true;
          const title = a.title ?? t.title;
          if (a.ok && !m.quiet) {
            toast.now = {
              text: `Brought ${m.what} into “${title}”`,
              note: t.id,
              inserted: m.text,
              alt: m.alt,
            };
          }
          reply(m.n, { title, note: t.id, ...a });
        },
      };
      inbox.insert = insert;
      // A note just made needs a moment to open before its pane can take the insert.
      setTimeout(() => {
        if (inbox.insert === insert) inbox.insert = null;
        insert.done({
          ok: false,
          error: 'The note is not on screen in formicaria; open it, then bring it in.',
        });
      }, 6000);
    })();
  };
  window.addEventListener('message', onMessage);
  return () => window.removeEventListener('message', onMessage);
}

/** Undo the last bring-in: its text taken back out of the note. */
export function undo(t: Toast) {
  if (!t.note || !t.inserted) return;
  inbox.replace = { id: t.note, from: t.inserted, to: '', done: () => {} };
  toast.now = null;
}

/** The one alternative (*Link only instead*, *Keep the file*): the add-on makes it and replaces. */
export function alternative(t: Toast) {
  if (!t.alt || !t.note || !t.inserted) return;
  up({ up: 'alt', token: t.alt.token, note: t.note, from: t.inserted });
  toast.now = { text: 'Changing it…' };
}

/** `text` added at the end of `body`, after a blank line. */
export function appended(body: string, text: string): string {
  const t = text.trim();
  if (!body.trim()) return `${t}\n`;
  const sep = body.endsWith('\n\n') ? '' : body.endsWith('\n') ? '\n' : '\n\n';
  return `${body}${sep}${t}\n`;
}

/** `body` with the last `from` replaced by `to` (empty `to` takes it out, with its blank line). */
export function replaced(body: string, from: string, to: string): string | null {
  const f = from.trim();
  const at = body.lastIndexOf(f);
  if (at < 0) return null;
  if (to.trim()) return body.slice(0, at) + to.trim() + body.slice(at + f.length);
  const before = body.slice(0, at).replace(/\n+$/, '\n');
  const after = body.slice(at + f.length).replace(/^\n+/, '');
  const out = (before + after).replace(/\n{3,}/g, '\n\n');
  return out.trim() ? out : '';
}
