# Repo context — start here

This folder is the **project's working memory**: a compact synthesis of what
formicarium is, what we implemented and *why*, and what is *not* working — enough
to reconstruct the context of the repo **without loading a large context
window**. It is written for a future Claude (or human) picking the project up
cold.

> **If you are an assistant working in this repo: read [overview.md](./overview.md)
> first, then skim the file below that matches your task, before you start.**
> When you finish substantive work, update these files (see "Update discipline").

This is separate from the mdBook user manual (`docs/src/`) — that explains how to
*use* the app; this explains how to *understand and maintain* it. It is also
distinct from the canonical spec, `formicarium/MASTERPLAN.md`, which stays the
authoritative design document; these files are the fast-recall layer over it.

## The files

| File | Read it when you need… |
|------|------------------------|
| [overview.md](./overview.md) | the 5-minute mental model: what it is, how it runs, architecture, seams, status. **Always read first.** |
| [decisions.md](./decisions.md) | *why* something is the way it is before you change it (pivots, reversals, load-bearing constraints). |
| [known-issues.md](./known-issues.md) | what's broken/rough/deferred, and the traps (toolchain, sandbox, CI greps) that will bite you. |
| [sessions/](./sessions/) | the narrative history — one append-only entry per working session, newest kept. |

## Update discipline

Keep this folder **true and small** — a stale synthesis is worse than none.

1. **After substantive work**, before ending: add a `sessions/YYYY-MM-DD-<slug>.md`
   entry (what you did, decisions, what you left not-working), and reconcile
   `overview.md` / `decisions.md` / `known-issues.md` with reality — delete fixed
   entries, add new gaps, bump the "Last verified" line + commit hash.
2. **Prune, don't append forever.** overview/decisions/known-issues are
   *current-state* documents, not logs — the log is `sessions/`. Fold durable
   outcomes up into the three top-level files and keep session entries lean.
3. **The code wins.** When a file here disagrees with the code, fix the file.
   If a note names a file/function/flag, verify it still exists before relying
   on it.
4. **No secrets, no vault content** — this ships in the repo.
