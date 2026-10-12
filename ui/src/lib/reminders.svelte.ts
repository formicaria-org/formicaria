// Reminders: the moments the core works out, handed to whatever can show them
// (`decisions.md#ui`, 2026-10-13, *a reminder is the note's own date, said at its time*).
//
// - On the phone, Android sets an alarm for each and shows it with the app closed (`__fmRemind`).
// - Wherever the app is open, this page shows it: a line in the app, and the system's own
//   notification where the person has allowed the browser to.
//
// Nothing is kept here but whether this device wants them: the moments are asked for again
// whenever the app starts or something changes, so a date changed elsewhere is right at the
// next ask.
import { reminders as ask } from './ipc';
import { today } from './units';
import type { Reminder } from './types';

type Bridge = { set(json: string): number; allowed(): boolean; ask(): void };
const bridge = (): Bridge | null => (globalThis as { __fmRemind?: Bridge }).__fmRemind ?? null;

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
/** A wall-clock moment as this device's time. */
export const msOf = (at: string): number => {
  const [d, t] = at.split('T');
  const [y, m, day] = d.split('-').map(Number);
  const [h, min] = (t ?? '09:00').split(':').map(Number);
  return new Date(y, m - 1, day, h, min).getTime();
};

/** Can the system show a notification here at all, and has the person said yes? */
export function allowed(): boolean {
  const b = bridge();
  if (b) return b.allowed();
  return typeof Notification !== 'undefined' && Notification.permission === 'granted';
}

/** Turn reminders on for this device, asking the system's own permission where there is one. */
export async function allow(): Promise<void> {
  const b = bridge();
  if (b) b.ask();
  else if (typeof Notification !== 'undefined' && Notification.permission === 'default') {
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
  // On the phone Android has shown it already.
  if (!bridge() && typeof Notification !== 'undefined' && Notification.permission === 'granted') {
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
      const b = bridge();
      if (!state.on) {
        b?.set('[]');
        return done();
      }
      try {
        const list = await ask(today(), 14);
        const now = Date.now();
        b?.set(
          JSON.stringify(
            list.map((r) => ({ title: r.title || 'A note', text: textOf(r), at: msOf(r.at) })),
          ),
        );
        for (const r of list) {
          const wait = msOf(r.at) - now;
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
