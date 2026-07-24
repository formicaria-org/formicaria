# Repo context — start here

This folder is the project's **working memory**: enough to reconstruct what formicaria is, what we
built and *why*, and what is *not* working — **without loading a large context window.** It is
written for a future Claude (or human) picking the project up cold. It is separate from the mdBook
user manual (`docs/src/`, how to *use* the app) and from the canonical spec
(`formicaria/MASTERPLAN.md`, the authoritative design); these files are the fast-recall layer over
both.

## Two layers — read this, it is the whole design

The failure this folder kept hitting is **bloat**: a running narrative and never-deleted "fixed"
entries grew it past the point where anyone could read it. The fix (grounded in how Claude Code /
Codex / Cline actually manage agent memory) is **two layers**:

- **Always-read** — [`overview.md`](./overview.md) + [`features.md`](./features.md). Small, current,
  read at the start of any session. `overview.md` carries the philosophy, the architecture, and a
  **router** that names the exact on-demand file to open before you touch a hot area. `features.md`
  is the one-line-per-feature index. Keep the two **together under ~400 lines** — the seam mechanics
  stay in `overview.md` (they are what the bugs came from), but nothing else earns always-loaded
  space unless *removing it would cause a mistake*.
- **On-demand** — everything else, pulled **only when the router or the feature index points you
  there**. Retrieval must be precise: grep a subject, land on one entry, don't load the whole file.

| File | Layer | Read it when… |
|---|---|---|
| [overview.md](./overview.md) | always | the 5-minute model: what it is, how it runs, the 3 seams, the router. **First.** |
| [features.md](./features.md) | always | you need the status of a feature or a pointer to its detail. |
| [decisions.md](./decisions.md) | on-demand | you are about to change something — grep its **subject index** for the `#tag` the router gave you. Append-only, dated; reversals are chains, never edits. |
| [known-issues.md](./known-issues.md) | on-demand | before assuming something works — durable traps + open gaps. |
| [outstanding.md](./outstanding.md) | on-demand | picking up work — the ranked queue, each naming the file and what "done" means. |
| topic docs (`mobile-design.md`, `ai-agents-plan.md`, `*-research-*.md`) | on-demand | the router sends you there for a specific area's receipts. |
| [sessions/](./sessions/) | log | the dated narrative — how we got here. Not always-read; distil durable facts up. |
| [archive/](./archive/) | cold | retired docs kept one `ls` away rather than only in `git log`. |

**Two rules keep it from re-exploding, and `ci/checks.sh` enforces the first:**
1. **Every router pointer resolves.** The always-read layer may only point at files that exist and
   subjects that exist — an on-demand doc nobody is pointed to is invisible. CI fails on a dead
   pointer.
2. **Prune the always-read layer; append the record.** `overview.md`/`features.md` are current-state
   and get *trimmed* (a fixed bug's entry is deleted). `decisions.md` is a record and gets
   *appended* (a decision is superseded, never edited away). Never mix the two contracts in one file.

## Update discipline

When you finish substantive work, **before ending**:
1. **`features.md`** — bump the feature's status/line if it changed.
2. **The relevant on-demand doc** — `known-issues.md` (delete a fixed entry, add a new gap),
   `outstanding.md`, or a topic doc. If you made a design *decision* or reversed one, **append** a
   dated entry to `decisions.md` (with a `#subject` from the index) — do not edit an old one away.
3. **`overview.md`** — only if the model or a seam changed. Add a router row for any new on-demand
   doc. Do **not** grow a "what shipped when" narrative here — that is what `sessions/` is for.
4. **A `sessions/YYYY-MM-DD-<slug>.md`** entry is optional and for the log; the durable facts belong
   up in the layers above, not only in the session file.

**The code wins.** When a file here disagrees with the code, fix the file. If a note names a
file/function/flag, verify it still exists before relying on it. No secrets, no vault content.

## The cold-read test — run it after any ruling, and quarterly

`docs/book.toml` renders `docs/src`, not `docs/context`, so **these files have zero CI coverage**
beyond the router-pointer check. This is the substitute: open a **fresh session**, read only the
always-read layer + whatever the router points you to, and answer from the corpus alone — then check
each against the tree. **On 2026-07-19, six of seven answered wrong.** Record the count each time;
treat any non-zero as work.

1. Was the mobile **transport** ruling reversed? → *No.*
2. Is the `.md` **merge in-process**? → *On the phone yes (`git_native`); the desktop shells out.*
3. Does **`fm-cli` owe a migration** onto `dispatch`? → *No — ruled the other way.*
4. Can a **phone create/accept a proposal**? → *Yes, since 2026-07-24 (`git_native` + `vcs`).*
5. What is **executable today** with no phone and no NDK?
6. What does the corpus say about **viewing a PDF on Android**, and about foreground services?
7. Is **`fm-serve` safe to run on a phone**? → *No. Loopback is not sandboxed on Android.*

Three standing rules, because these are what actually failed:
- **Cite rulings by subject (`decisions.md#tag`), never by number** across document boundaries.
- **Any external claim gets a date and a re-verify command** (`known-issues.md` → "External facts").
- **A mitigation that names a mechanism must name an executor that exists.** "Pin it in CI" is not a
  mitigation while every workflow is `workflow_dispatch`-only.
