# CLAUDE.md — read before working in this repo

**First action in any session: read [`docs/context/README.md`](./docs/context/README.md)
and then [`docs/context/overview.md`](./docs/context/overview.md).**

`docs/context/` is this project's working memory — a compact synthesis of what
formicarium is, what we built and *why*, and what is *not* working. It exists so
you can reconstruct the repo's context **without a large context window**. Skim
the file there that matches your task (`decisions.md` before you change a design,
`known-issues.md` for gaps + toolchain traps) before you start.

The canonical design spec is [`formicarium/MASTERPLAN.md`](./formicarium/MASTERPLAN.md);
`docs/src/` is the mdBook **user** manual. `docs/context/` is the fast-recall
maintainer layer over both.

## Keep the context current

When you finish substantive work, **update `docs/context/`** before ending: add a
`sessions/YYYY-MM-DD-<slug>.md` entry and reconcile `overview.md` /
`decisions.md` / `known-issues.md` with reality (delete fixed items, add new
gaps, bump the "Last verified" line). A stale synthesis is worse than none.

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
