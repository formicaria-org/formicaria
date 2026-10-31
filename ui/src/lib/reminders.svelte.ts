// Reminders: the moments the core works out, handed to whatever can show them
// (`decisions.md#ui`, 2026-10-13, *a reminder is the note's own date, said at its time*).
//
// - Wherever the app is open, this page shows it: a line in the app, and the system's own
//   notification where the person has allowed the browser to.
// - On a computer the program itself says it when no page is open (`remindSet`).
// - **On a phone, only the line in the open app** (`decisions.md#track-m`, 2026-10-31): the Android
//   alarms, start-at-boot receiver and notifications came with the Play Protect verdict, did not
//   work reliably, and were taken out.
//
// Nothing is kept here but whether this device wants them: the moments are asked for again
// whenever the app starts or something changes, so a date changed elsewhere is right at the
// next ask.
import { remindSet, reminders as ask } from './ipc';
import { isPhone } from './platform';
import { today } from './units';
import type { Reminder } from './types';

const KEY = 'fm-reminders';
function stored(): boolean {
  try {
    return localStorage.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

const state = $state({
  on: stored(),
  showing: [] as { id: string; title: string; text: string }[],
});
/** Does this device want reminders? */
export const remindersOn = () => state.on;
/** The reminders that came due while the app was open, until each is put away. */
export const showing = () => state.showing;
export const putAway = (id: string) =>
  void (state.showing = state.showing.filter((r) => r.id !== id));

/** What a reminder says under the note's title. */
export function textOf(r: Pick<Reminder, 'what' | 'when' | 'all_day'>): string {
  const verb = r.what === 'due' ? 'Due' : 'Starts';
  if (r.when) return r.when === 'that day' ? `${verb} that day` : `${verb} at ${r.when}`;
  return r.all_day ? `${verb} today` : `${verb} now`;
}
/** **A reminder's moment** (2026-10-24, F8): the server's `at_ms`, worked out on the app's clock —
 *  never `at` read in this browser's zone, which can lag a laptop's change of zone, or be another
 *  device's. Only a server older than 0.19 sends none; its `at` is then read as before. */
export const msOf = (r: Pick<Reminder, 'at' | 'at_ms'>): number => {
  if (typeof r.at_ms === 'number') return r.at_ms;
  return Date.parse(`${r.at.length > 10 ? r.at : `${r.at}T09:00`}:00`);
};

/** Can the system show a notification here at all, and has the person said yes? */
export function allowed(): boolean {
  return typeof Notification !== 'undefined' && Notification.permission === 'granted';
}

/** Turn reminders on for this device, asking the system's own permission where there is one. */
export async function allow(): Promise<void> {
  if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
    try {
      await Notification.requestPermission();
    } catch {
      // A browser that refuses to ask still gets the line in the app.
    }
  }
  set(true);
}
export function set(on: boolean) {
  state.on = on;
  try {
    if (on) localStorage.setItem(KEY, '1');
    else localStorage.removeItem(KEY);
  } catch {
    // A device that keeps nothing has them off next time.
  }
  void sync();
}

let timers: ReturnType<typeof setTimeout>[] = [];
let asking: ReturnType<typeof setTimeout> | null = null;
/** How far ahead the open page keeps its own timers: a long timer is one a sleeping tab drops. */
const AHEAD_MS = 12 * 60 * 60 * 1000;

function fire(r: Reminder) {
  const text = textOf(r);
  if (!state.showing.some((x) => x.id === r.id + r.at))
    state.showing = [...state.showing, { id: r.id + r.at, title: r.title || 'A note', text }];
  if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {
    try {
      new Notification(r.title || 'formicaria', { body: text, tag: r.id + r.at });
    } catch {
      // Some browsers only allow it from a service worker; the line in the app is there.
    }
  }
}

/** Ask for the moments and hand them on. Safe to call often: it waits a moment and asks once. */
export function sync(): Promise<void> {
  return new Promise((done) => {
    if (asking) clearTimeout(asking);
    asking = setTimeout(async () => {
      asking = null;
      for (const t of timers) clearTimeout(t);
      timers = [];
      // On a computer the program itself says a reminder when no page is open, and stays
      // running for it; a browser reaching someone else's computer is refused, which is right.
      const program = (items: { title: string; text: string; at: number }[]) => {
        if (!isPhone()) void remindSet(items).catch(() => {});
      };
      if (!state.on) {
        program([]);
        return done();
      }
      try {
        const list = await ask(today(), 14);
        const now = Date.now();
        const items = list.map((r) => ({
          title: r.title || 'A note',
          text: textOf(r),
          at: msOf(r),
        }));
        program(items.filter((i) => i.at > now));
        for (const r of list) {
          const wait = msOf(r) - now;
          if (wait > 0 && wait < AHEAD_MS) timers.push(setTimeout(() => fire(r), wait));
        }
        // A page left open asks again before the timers it holds run out.
        timers.push(setTimeout(() => void sync(), AHEAD_MS / 2));
      } catch {
        // Never the reason the app fails to open; the next change asks again.
      }
      done();
    }, 800);
  });
}
