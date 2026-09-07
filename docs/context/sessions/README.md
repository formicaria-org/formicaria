# sessions/ — dated snapshots, never updated

**Every file here is a record of what was believed on the day in its filename. None of them is
maintained, and where one disagrees with the code, the code wins.**

That contract had never been written down, which is the only reason it needs a file. 62 dated
narratives sat beside four documents that *are* kept current, with nothing on the outside to tell
them apart — so a reader following `overview.md`'s "the dated log of how we got here" could land on
a 2026-07 session and read a status claim as though it still held.

## What that means in practice

- **Do not fix a session file when the code moves past it.** A snapshot corrected after the fact is
  no longer a snapshot; it becomes a second, worse copy of the current docs. The value of
  `2026-07-19-the-phone.md` is precisely that it says what was true and believed *that day*.
- **Do not cite one as evidence that something is still so.** For current fact:
  [`features.md`](../features.md) for status, [`known-issues.md`](../known-issues.md) for gaps and
  traps, [`outstanding.md`](../outstanding.md) for the queue, [`decisions.md`](../decisions.md) for
  the *why*. Those four are the ones under the reconciliation rule in
  [`README.md`](../README.md).
- **Distil upward instead.** A durable lesson from a session belongs in `known-issues.md`'s traps or
  as a dated `decisions.md` entry — which is what `README.md` has always said. The session keeps the
  story; the durable layer keeps the conclusion.
- **Writing one is optional.** Some weeks earn a narrative and most do not. An absent session file
  is not a gap; a session file that has quietly become the only record of a decision is.

## The one exception

Privacy redactions. These files are public, and a few carried an exact device model, an absolute
home path, or a link between the maintainer's accounts. Those were removed on 2026-09-05 without
touching anything technical — the measurement each one supported is intact. A snapshot is a record
of what was believed, not a licence to publish someone's hardware serial.

## Range

`2026-07-15` through `2026-08-29`. The log stops before the last week of decisions; that gap is
real, and `decisions.md` — which runs to the present — is the place that does not have it.
