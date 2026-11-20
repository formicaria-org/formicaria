// **What a page tells the usage diary** (`fm_app::diary`, `decisions.md` 2026-11-20): which of
// formicaria's pages opened and how long it took, and errors, with what could name a person's things
// taken out here and again on the other side (`diary::from_pages`). Commands are counted where they
// are answered; this adds what only the page sees. Sent in batches, at most once a minute and as the
// page goes, never in the way: a failure is dropped.
import { diaryNote } from './ipc';

type Event = Record<string, string | number>;

const queue: Event[] = [];
let timer: ReturnType<typeof setTimeout> | null = null;

/** Anything quoted, an address, a path, an email or a long number, taken out; short. */
export function clean(s: string, max = 160): string {
  return s
    .replace(/(["'`“‘«])[^"'`”’»]*["'`”’»]/g, '…')
    .split(/\s+/)
    .map((w) =>
      w.includes('://') || w.startsWith('/') || w.includes('@') || w.includes('www.')
        ? '…'
        : (w.match(/\d/g)?.length ?? 0) >= 4
          ? '#'
          : w,
    )
    .join(' ')
    .trim()
    .slice(0, max);
}

export function send() {
  if (timer) clearTimeout(timer);
  timer = null;
  if (!queue.length) return;
  const events = queue.splice(0, queue.length);
  void diaryNote(events).catch(() => {});
}

function add(e: Event) {
  queue.push(e);
  if (queue.length >= 50) send();
  else if (!timer) timer = setTimeout(send, 60_000);
}

/** A page opened: its kind (a feature's name, "note", "settings"), and how long since the page began. */
export function pageOpened(page: string) {
  add({ kind: 'page', page: page.slice(0, 40), ms: Math.round(performance.now()) });
}

/** Something went wrong in the page. */
export function error(message: string, where = 'ui') {
  add({ kind: 'error', where, message: clean(message) });
}

let watching = false;
/** Errors the page did not catch, and the last batch sent as the page goes. Once per page. */
export function watch() {
  if (watching || typeof window === 'undefined') return;
  watching = true;
  window.addEventListener('error', (e) => error(String(e.message ?? e.error ?? 'error')));
  window.addEventListener('unhandledrejection', (e) =>
    error(String((e.reason as Error)?.message ?? e.reason ?? 'rejected')),
  );
  window.addEventListener('pagehide', send);
}

/** For tests: what is waiting. */
export const waiting = () => [...queue];
