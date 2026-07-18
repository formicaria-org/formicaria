// The UI mirror of Rust's `fm_model::Stamp`: `start`/`due` arrive over the wire
// as either `YYYY-MM-DD` (all day) or `YYYY-MM-DDTHH:MM` (timed). Every module
// that needs to read one of those halves goes through here rather than slicing
// the string itself — the old code slice()'d and string-concatenated in four
// different places, and one of them silently produced NaN.

export interface Stamp {
  /** The calendar day, always present: `YYYY-MM-DD`. */
  day: string;
  /** Wall-clock `HH:MM`, or null for an all-day stamp. */
  time: string | null;
}

/** Split a wire value. Returns null for null/empty/unparseable input, so callers
 *  can distinguish "no stamp" from "a stamp at midnight".
 *
 *  Anchored at both ends ON PURPOSE: this must NOT match an offset-aware RFC3339
 *  instant like `2026-07-14T09:00:00Z` (the `created` field). Those name a point
 *  in time, not a wall clock, and their local day can differ from the UTC day
 *  printed in the string — so they have to go through `Date`, not through here.
 *  A loose prefix match would hand back the UTC day and shift `created` by a day
 *  for anyone east of UTC. */
export function parseStamp(raw: string | null | undefined): Stamp | null {
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})(?:[T ](\d{2}:\d{2}))?$/.exec(raw.trim());
  if (!m) return null;
  return { day: m[1], time: m[2] ?? null };
}

/** The calendar day of a naive stamp, or '' if the value isn't one. This is what
 *  the calendar grid and the urgency bands key on — both are day-granular. */
export function dayOf(raw: string | null | undefined): string {
  return parseStamp(raw)?.day ?? '';
}

/** Compose the wire value from a date input and an optional time input. An empty
 *  day clears the property (an empty string is the documented clear gesture);
 *  a time without a day is meaningless and also clears. */
export function toStamp(day: string, time: string | null | undefined): string {
  if (!day) return '';
  return time ? `${day}T${time.slice(0, 5)}` : day;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Human-facing short form for a card or list row: `20 Jul`, or `20 Jul 14:30`
 *  when a time is set. Deliberately not `toLocaleDateString` — that varies with
 *  the browser locale and would make the snapshot-ish tests machine-dependent. */
export function formatStamp(raw: string | null | undefined): string {
  const s = parseStamp(raw);
  if (!s) return raw ?? '';
  const [, mo, d] = s.day.split('-');
  const label = `${Number(d)} ${MONTHS[Number(mo) - 1] ?? mo}`;
  return s.time ? `${label} ${s.time}` : label;
}

/** A compact "how long ago" for an ISO timestamp — "just now", "5m", "3h", "2d", "6w", or a
 *  `D Mon` date past ~a year. `now` is injectable so it's testable. Used by the "edited by"
 *  label and the activity stream. */
export function relativeTime(iso: string, now: number = Date.now()): string {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return '';
  const s = Math.max(0, Math.round((now - then) / 1000));
  if (s < 45) return 'just now';
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.round(h / 24);
  if (d < 7) return `${d}d ago`;
  const w = Math.round(d / 7);
  if (w < 52) return `${w}w ago`;
  return formatStamp(iso.slice(0, 10));
}

/** The `HH:MM` window a note occupies on its day, when both ends are timed —
 *  what a meeting reads as. Empty when neither end carries a time. */
export function timeRange(start: string | null, due: string | null): string {
  const a = parseStamp(start)?.time ?? null;
  const b = parseStamp(due)?.time ?? null;
  if (a && b) return `${a}–${b}`;
  return a ?? b ?? '';
}
