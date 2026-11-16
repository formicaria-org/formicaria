// **Out of the library, by hand-off** (`docs/context/workspace-design-2026-11-15.md` §13.6;
// `decisions.md` 2026-11-16, *one library of notes*). A note's content reaches a service outside
// formicaria by opening that service **with the content already in it**; the person finishes there
// (sends the mail, saves the event). formicaria writes nothing outside its notebook, and mail stays
// read-only (`gmail.readonly`). These are the addresses and files it opens.
import { addDays } from './civil';

/** A note's `start` / `due` (`2026-11-20` or `2026-11-20T14:30`) as a calendar's dates: an all-day
 *  span, or a timed one (an hour when no end is given), in the calendar's own zone. */
export function eventDates(start: string, end?: string | null): string | null {
  const day = /^(\d{4})-(\d{2})-(\d{2})$/;
  const timed = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/;
  if (day.test(start)) {
    // A calendar's all-day span ends the day after its last day (`civil.ts`: days, no clock read).
    const last = end && day.test(end) ? end : start;
    const flat = (d: string) => d.replace(/-/g, '');
    return `${flat(start)}/${flat(addDays(last, 1))}`;
  }
  const t = timed.exec(start);
  if (!t) return null;
  const from = `${t[1]}${t[2]}${t[3]}T${t[4]}${t[5]}00`;
  const u = end ? timed.exec(end) : null;
  if (u) return `${from}/${u[1]}${u[2]}${u[3]}T${u[4]}${u[5]}00`;
  const h = +t[4] + 1;
  const sameDay = h < 24;
  const to = sameDay
    ? `${t[1]}${t[2]}${t[3]}T${String(h).padStart(2, '0')}${t[5]}00`
    : `${t[1]}${t[2]}${t[3]}T235900`;
  return `${from}/${to}`;
}

/** Google Calendar's new-event page, prefilled. `null` when the note has no date to give it. */
export function calendarEvent(e: {
  title: string;
  start: string;
  end?: string | null;
  details?: string;
  location?: string;
}): string | null {
  const dates = eventDates(e.start, e.end);
  if (!dates) return null;
  const q = new URLSearchParams({ action: 'TEMPLATE', text: e.title, dates });
  if (e.details) q.set('details', e.details);
  if (e.location) q.set('location', e.location);
  return `https://calendar.google.com/calendar/render?${q.toString()}`;
}

/** The same event as an `.ics` file, for any other calendar. */
export function icsEvent(e: {
  title: string;
  start: string;
  end?: string | null;
  details?: string;
  uid: string;
}): string | null {
  const dates = eventDates(e.start, e.end);
  if (!dates) return null;
  const [a, b] = dates.split('/');
  const allDay = a.length === 8;
  const esc = (s: string) =>
    s.replace(/\\/g, '\\\\').replace(/;/g, '\;').replace(/,/g, '\\,').replace(/\n/g, '\\n');
  const kind = allDay ? ';VALUE=DATE' : '';
  return [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//formicaria//EN',
    'BEGIN:VEVENT',
    `UID:${e.uid}`,
    `DTSTART${kind}:${a}`,
    `DTEND${kind}:${b}`,
    `SUMMARY:${esc(e.title)}`,
    ...(e.details ? [`DESCRIPTION:${esc(e.details)}`] : []),
    'END:VEVENT',
    'END:VCALENDAR',
    '',
  ].join('\r\n');
}
