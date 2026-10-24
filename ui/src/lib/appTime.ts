// **The app's own clock** (2026-10-24). A moment — an RFC3339 time with its offset — is shown in the
// zone `appZone()` answers. Today that is the device's own zone, which is what *every device shows the
// agenda on its own zone* (`decisions.md`, 2026-10-23) rules; `setAppZone` is how a zone chosen in
// the app will take over once that ruling is revisited.
//
// One place on purpose: a date formatted here and another formatted elsewhere with another zone
// would put the same moment on two different days.

import { deviceZone } from './zoneLabel';

let zone: string | null = null;

/** The zone moments are shown in. */
export function appZone(): string {
  return zone ?? deviceZone();
}

/** Set by the app once it has read the person's choice. An offset (`+09:00`) is accepted. */
export function setAppZone(z: string | null | undefined) {
  zone = z && z.trim() ? z.trim() : null;
}

/** An IANA name for `Intl`, which takes `Asia/Tokyo` or `UTC` but no bare offset. */
function intlZone(z: string): string | undefined {
  const m = /^([+-])(\d{2}):?(\d{2})$/.exec(z);
  if (!m) return z;
  // `Etc/GMT` signs are inverted, and only whole hours exist there; anything else falls back.
  if (m[3] !== '00') return undefined;
  const h = Number(m[2]);
  return h === 0 ? 'UTC' : `Etc/GMT${m[1] === '+' ? '-' : '+'}${h}`;
}

/** The day of a moment, in the app's zone: `24 Oct 2026`. Empty for anything that is not a time. */
export function formatDay(iso: string | null | undefined): string {
  if (!iso) return '';
  const t = new Date(iso);
  if (Number.isNaN(t.getTime())) return '';
  try {
    return new Intl.DateTimeFormat('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      timeZone: intlZone(appZone()),
    }).format(t);
  } catch {
    return new Intl.DateTimeFormat('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    }).format(t);
  }
}
