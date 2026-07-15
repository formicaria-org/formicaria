# 2026-07-15 — A time on `start`/`due` (and a plan for meetings)

**Outcome:** `start`/`due` now carry an **optional wall-clock time**. New
`fm_model::Stamp { date, time: Option<Time> }` replaces `Option<Date>` end to
end; `PropertyValue::Date` → `PropertyValue::Stamp`. `pixi run ci` exit 0 (82
Rust tests, 97 UI tests), svelte-check 0 errors, verified live against the real
HTTP API and the owner's real vault. Also planned the wider meeting/calendar work
in the new [roadmap.md](../roadmap.md).

## Why one `Stamp` and not two variants

See [decisions.md](../decisions.md) for the full entry. Short version: reusing
`PropertyValue::Date` + `DateTime` would have sorted **every all-day item before
every timed item** regardless of the day — `PropertyValue` derives `Ord` from
variant order first. That's a silent agenda-corrupting trap, so the type is one
variant whose `Ord` is (day, then `None` before `Some(time)`).

Naive by design (no UTC offset): 14:30 means 14:30 where you are, and the file is
the truth. `created`/`updated` stay `OffsetDateTime` — they're *instants*.

## Three silent bugs found and fixed on the way

The change was mapped before it was written, which surfaced three latent failures
that would have shipped as "it just doesn't work sometimes":

1. **`urgency.ts` — notes vanishing.** `daysUntil` built `` `${due}T00:00:00` ``
   by concatenation. A timed `due` gave `2026-07-20T14:30T00:00:00` → `NaN` →
   urgency `'none'` → **the note dropped out of every agenda band, silently.**
   Now narrowed via `dayOf` first; regression tests pin timed == bare-day.
2. **`dto.rs:value_string` → `display()` — the drag erasing times.** The board's
   drag write-back echoes `display()` into `apply_property`; the old `DateTime`
   display dropped the time. `Stamp::Display`/`FromStr` are now inverses, with a
   Rust test asserting exactly that round-trip.
3. **`calendar.ts:116`** matched days by exact string equality — `-1` for any
   timed value. Now day-narrowed before the geometry.

Bonus: `parseStamp` (the JS mirror) is **anchored at both ends** so it can't
prefix-match an RFC3339 `created` instant and hand back the UTC day — that would
have shifted Timeline grouping a day for anyone east of UTC. Test pins it.

## Shape

- **Rust:** new `crates/fm-model/src/stamp.rs` (Display/FromStr inverses, minute
  granularity, its own round-trip suite). `frontmatter.rs` and `edit.rs` both go
  through it, which **removed the duplicated date-format literal** across crates.
  `in_date_range` narrows `Stamp` → day (`DateRange` stays day-granular).
- **UI:** new `ui/src/lib/stamp.ts` (`parseStamp`/`dayOf`/`toStamp`/`formatStamp`/
  `timeRange`) — the one module that knows the wire format. `NotePanel` uses a
  **date input + a separate time input** (not `datetime-local`, which would force
  a time on every deadline and make "sometime Tuesday" unexpressible); the time
  is disabled until a date exists, and clearing the date clears both. Cards and
  the agenda render `formatStamp` (`20 Jul 14:30`) instead of dumping the raw
  value; calendar bars label their window (`14:30–15:00`).
- **Mock parity:** `mock.ts` now *rejects* bad stamps like `apply_property` does,
  so UI tests can't pass against a backend that doesn't exist.

## Verified (beyond the suites)

Live `fm-serve`: `get`/`set_property`/`agenda` all carry `2026-08-01T16:45`
correctly and refuse `next tuesday` with a 500 + a readable message. Via `fm`:
a **legacy all-day note stays bare on disk** (no vault-wide churn — the
idempotence invariant holds), a hand-written `2026-08-05 09:00:00` normalizes to
`2026-08-05T09:00`, and a rejected write leaves the field untouched. The owner's
4 real notes all still load, unmodified.

## Left not-working / next

- Urgency bands are **day-granular by choice** — a 14:00 meeting doesn't change
  band at 14:01. If the owner wants "starts in 30 min", that's a new concept.
- The calendar grid is still day-granular; a **time-of-day week view** is now
  unlocked but not built.
- Everything else in [roadmap.md](../roadmap.md): calendar ICS import/export
  (decided: NUS M365 + Google, merged, one-way in + one-way out; two-way
  rejected), whiteboard-embed + print-to-PDF, and the owner's `Source` module
  idea with local extraction (**the model proposes, never writes**).
