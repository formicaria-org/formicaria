// **The app's clock** (`decisions.md#data`, 2026-10-24, *one app clock*) — the only place the UI asks
// what time it is, what day it is, or which day and hour a moment falls on.
//
// The zone comes from the server (the `clock` command, and every `ping`), never from the browser:
// a browser can lag a laptop's change of zone, and on a paired tablet it is another device's. Until
// the server has answered, the browser's own zone stands in. A fixed offset (`+05:45`) is counted
// exactly; a zone name goes through `Intl` with that zone.
//
// One place on purpose: a date formatted here and another formatted elsewhere with another zone
// would put the same moment on two different days. A CI grep keeps the browser's clock out of the
// rest of `ui/src`.

import { deviceZone } from './zoneLabel';

/** The server's `clock` answer (contract `v: 1`; fields are only ever added). */
export interface ClockAnswer {
  zone: string | null;
  offset_seconds?: number | null;
  offset?: string | null;
  today?: string | null;
  now?: string;
  source?: string;
  chosen?: string;
  system?: string | null;
  /** How far this device's clock was from Google's or GitHub's last answer; `null` until known. */
  skew_seconds?: number | null;
  skew_from?: string | null;
}

let zone: string | null = null;

/** Take the server's clock. Answers whether the zone in force changed. */
export function setAppClock(answer: ClockAnswer | null | undefined): boolean {
  const next = answer?.zone?.trim() || null;
  const changed = next !== zone;
  zone = next;
  return changed;
}

/** Choose a zone by hand (tests, and the moment before the server answers). */
export function setAppZone(z: string | null | undefined) {
  zone = z && z.trim() ? z.trim() : null;
}

/** The zone moments are shown in: the app's, else this browser's. */
export function appZone(): string {
  return zone ?? deviceZone();
}

/** `+05:45` → 20700; anything that is not a bare offset → null. */
function fixedOffset(z: string): number | null {
  if (/^(z|utc)$/i.test(z)) return 0;
  const m = /^([+-])(\d{2}):?(\d{2})$/.exec(z);
  if (!m) return null;
  const s = Number(m[2]) * 3600 + Number(m[3]) * 60;
  return m[1] === '-' ? -s : s;
}

/** A moment's calendar fields on the app's clock. */
export interface Parts {
  year: number;
  month: number; // 1–12
  day: number;
  hour: number;
  minute: number;
}

const formats = new Map<string, Intl.DateTimeFormat>();

/** The fields of `ms` (milliseconds since 1970) on the app's clock. */
export function partsOf(ms: number): Parts {
  const z = appZone();
  const off = fixedOffset(z);
  if (off !== null) {
    const t = new Date(ms + off * 1000);
    return {
      year: t.getUTCFullYear(),
      month: t.getUTCMonth() + 1,
      day: t.getUTCDate(),
      hour: t.getUTCHours(),
      minute: t.getUTCMinutes(),
    };
  }
  let f = formats.get(z);
  if (!f) {
    try {
      f = new Intl.DateTimeFormat('en-US', {
        timeZone: z,
        hourCycle: 'h23',
        year: 'numeric',
        month: 'numeric',
        day: 'numeric',
        hour: 'numeric',
        minute: 'numeric',
      });
    } catch {
      // A zone this browser does not know: its own zone, rather than nothing.
      f = new Intl.DateTimeFormat('en-US', {
        hourCycle: 'h23',
        year: 'numeric',
        month: 'numeric',
        day: 'numeric',
        hour: 'numeric',
        minute: 'numeric',
      });
    }
    formats.set(z, f);
  }
  const get = (type: string) =>
    Number(f.formatToParts(new Date(ms)).find((p) => p.type === type)?.value);
  return {
    year: get('year'),
    month: get('month'),
    day: get('day'),
    hour: get('hour'),
    minute: get('minute'),
  };
}

const pad = (n: number) => String(n).padStart(2, '0');
const msOf = (v: string | number): number => (typeof v === 'number' ? v : new Date(v).getTime());

/** The day of a moment on the app's clock: `YYYY-MM-DD`; '' for anything that is not a moment. */
export function dayOfInstant(v: string | number | null | undefined): string {
  if (v === null || v === undefined || v === '') return '';
  const ms = msOf(v);
  if (Number.isNaN(ms)) return '';
  const p = partsOf(ms);
  return `${p.year}-${pad(p.month)}-${pad(p.day)}`;
}

/** Today, on the app's clock. */
export function appToday(): string {
  return dayOfInstant(Date.now());
}

/** The time of a moment on the app's clock: `HH:MM` (now, without an argument). */
export function clockText(v: string | number = Date.now()): string {
  const ms = msOf(v);
  if (Number.isNaN(ms)) return '';
  const p = partsOf(ms);
  return `${pad(p.hour)}:${pad(p.minute)}`;
}

/** Minutes since midnight now, on the app's clock — where the agenda draws its "now" line. */
export function appNowMinutes(): number {
  const p = partsOf(Date.now());
  return p.hour * 60 + p.minute;
}

/** A moment worded by the reader's locale, on the app's clock (date and time by default). */
export function formatMoment(
  v: string | number | null | undefined,
  opts: Intl.DateTimeFormatOptions = { dateStyle: 'medium', timeStyle: 'short' },
  locale?: string,
): string {
  if (v === null || v === undefined || v === '') return '';
  const ms = msOf(v);
  if (Number.isNaN(ms)) return typeof v === 'string' ? v : '';
  const z = appZone();
  const off = fixedOffset(z);
  if (off !== null) {
    return new Intl.DateTimeFormat(locale, { ...opts, timeZone: 'UTC' }).format(
      new Date(ms + off * 1000),
    );
  }
  try {
    return new Intl.DateTimeFormat(locale, { ...opts, timeZone: z }).format(new Date(ms));
  } catch {
    return new Intl.DateTimeFormat(locale, opts).format(new Date(ms));
  }
}

/** The day of a moment, worded: `24 Oct 2026`. Empty for anything that is not a time. */
export function formatDay(iso: string | null | undefined): string {
  return formatMoment(iso, { day: 'numeric', month: 'short', year: 'numeric' }, 'en-GB');
}
