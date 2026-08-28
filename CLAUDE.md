# CLAUDE.md — read before working in this repo

**First action in any session: read the always-read layer —
[`docs/context/overview.md`](./docs/context/overview.md) and
[`docs/context/features.md`](./docs/context/features.md).** `overview.md` ends with a **router**:
before you touch a hot area (a seam, git/sync, the phone, the agent), open the on-demand file it
names first. [`docs/context/README.md`](./docs/context/README.md) explains the two-layer design.

`docs/context/` is this project's working memory. The two files above are read every session; the
rest — `decisions.md` (grep its `#subject` index for the *why*), `known-issues.md` (gaps + traps),
`outstanding.md` (the queue), and the topic docs — are pulled **on demand via the router**, so the
always-read layer stays small.

The canonical design spec is [`formicaria/MASTERPLAN.md`](./formicaria/MASTERPLAN.md);
`docs/src/` is the mdBook **user** manual. `docs/context/` is the fast-recall
maintainer layer over both.

## Keep the context current

When you finish substantive work, **before ending** (full protocol in `docs/context/README.md`):
- **`features.md`** — bump the feature's status/line if it changed.
- **The on-demand doc** — delete a fixed `known-issues.md` entry / add a new gap; and if you made or
  reversed a design decision, **append** a dated entry to `decisions.md` (with a `#subject`) — a
  decision is superseded, never edited away.
- **`overview.md`** — only if the model or a seam changed; add a router row for any new on-demand
  doc. Do **not** grow a "what shipped when" narrative here — that is what `sessions/` is for.

Prune the always-read layer (a stale synthesis is worse than none); append the decision record.
Keep the two layers together under ~400 lines — `ci/checks.sh` fails on a dead router pointer.

## Before you add anything — four questions

This repo has ruled on a lot, and the rulings are easy to contradict by accident: every addition is
justified locally, and nothing here checks the sum. So before writing code that **adds** something —
a dependency, a file in the release archive, a relaxed guard, a changed default:

1. **What subject is this?** Grep `decisions.md`'s index: `#seams` `#git` `#sync` `#track-m` `#ui`
   `#vault` `#data` `#toolchain` `#agent`.
2. **Read the entries it names — and read past them.** The index is a starting point, not a
   contents page. (Found the hard way: the decisive ruling on desktop libgit2 is filed under
   `#track-m`, while the index lists the question under `#git`.)
3. **Classify honestly: permitted / extends an exception / contradicts a standing decision.** If
   everything you ever check comes back "permitted", you are not applying this.
4. **Does it widen what ships?** If yes it earns a `decisions.md` entry whatever the verdict.

**If it contradicts, write the dated reversal first** — and put a `> SUPERSEDED` banner on the entry
it supersedes. Writing it first forces the argument while changing course is still cheap; writing it
afterwards is how a project stops being what it said it was, one reasonable step at a time.

## Non-negotiable house rules (full detail in docs/context/)

- **Toolchain is pixi-only.** `cargo/node/pnpm/mdbook/restic` are not on PATH —
  use `pixi run <task>` (`serve`, `test`, `test-ui`, `ci`, `docs`, …). `pixi run
  ci` is the single gate; keep it green.
- **`fm-query` must never touch `std::fs`/a db crate/paths** — compile-time +
  CI-grep enforced. This is the core seam.
- **Renderers stay literal-free** — no `todo/doing/done` or scheduling literals
  in `ui/src/renderers/` (CI grep).
- **Files-as-truth**: one Markdown note = one file; the atom is the file.
- **Git**: solo repo, work on `main`, no branch/PR ceremony — but commit **only
  when the user asks**.
- **Remote (claude.ai / web) sessions cannot open PRs or push here.** The GitHub
  account authenticated in a remote run is **not** the `nazeer` account that owns
  this repo, so `gh pr create` and any push will fail. In a remote session: do
  **not** attempt a PR or a push. Commit locally if useful, then **finish by
  producing a patch to apply on the owner's machine** — e.g.
  `git format-patch origin/main -o <scratchpad>` or `git diff > <scratchpad>/changes.patch`
  — and hand back the patch path plus a one-line `git apply` / `git am` instruction.
  Publishing is the owner's local step. *(This constraint holds for every repo on
  the owner's machine; the durable home is the global `~/.claude/CLAUDE.md`, which a
  remote session cannot edit — so it is restated here per-repo.)*
- In this environment, shell commands need `dangerouslyDisableSandbox: true`
  (sandboxed Bash hits a seccomp/`setgroups` error).
