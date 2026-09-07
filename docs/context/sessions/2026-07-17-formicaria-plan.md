# 2026-07-17 — Consolidated the formicaria evolution plan

**Outcome:** the forward-looking design (scattered across `roadmap.md`,
`collaboration-design.md`, and stale lines in `MASTERPLAN.md`) is now **one sequenced
program** in new [`plan.md`](../plan.md): the vision (*three pillars, one atom*), the
rename trigger, two owner rulings, and Track S (single-user) + Track C (collaboration)
in order. Docs-only pass — **no code changed**. `pixi run ci` unaffected (the file lives
in `docs/context/`, which is not in the mdBook `SUMMARY.md`).

## Why

The owner asked to refine and propose a *final* plan for evolving formicarium into
**formicaria** — knowledge management + task scheduling + collaboration on owned files —
keeping the founding spirit (simplicity, reusability, maintainability, bounded deps, a
real UI). The thinking was already done but split across three documents that overlapped
and, in places, disagreed with the code. Consolidation, not invention, was the job.

## Decisions made this pass (folded into `decisions.md`)

- **Rename deferred to Phase 2** (owner's call, matching the design doc's own discipline:
  rename when the plural is literally true). `fm-*`/`fm` identifiers don't move.
- **Ruling A — inline meeting actions:** checkboxes stay plain Markdown; a `/promote`
  gesture extracts a line into its own task note (no per-block identity, which voids the
  plan). Body-scan agenda pass rejected.
- **Ruling B — board images:** strip Excalidraw's inline base64 `files` into the blob
  store on save; rehydrate on load. Accept-and-document rejected (git churn per stroke).

## Grounding — verified against the working tree, not just `c9cd1ad`

The second explorer flagged uncommitted in-flight work on `git.rs`/`tests/git.rs`, so I
checked before writing "not built". **Half of Track C Phase 0 is already done:**

- ✅ `commit_all` refuses mid-merge — `git.rs:96-104` (`unmerged` scan of `status
  --porcelain`).
- ✅ `push_squashed` ancestry guard — `git.rs:250` (`is_ancestor(base, HEAD)`), plus
  history restore on a rejected push (`git.rs:291-295`). Design-doc §19 trap closed.
- ❌ Tolerant loader — `FileStore::open` still propagates `reindex(Reindex::Full)`
  (`file.rs:39`); one bad `.md` still bricks startup.
- ❌ Git identity — `ensure_identity` still writes `formicarium@localhost`
  (`git.rs:66-71`).

`plan.md` records this status inline so the next session doesn't re-plan finished work.

## Files touched

- **New:** `docs/context/plan.md`, this session entry.
- **Folded to a pointer:** `docs/context/roadmap.md` (content is now Track S of `plan.md`;
  stub kept so session-log `roadmap.md:NN` links still resolve).
- **Reconciled:** `README.md` (file table: `plan.md` + `collaboration-design.md` rows
  replace the roadmap row), `overview.md` (Last-verified line), `decisions.md` (3 new
  entries), `known-issues.md` (3 forward links repointed to `plan.md`).
- **CLAUDE.md:** new Git house rule — *remote (claude.ai) sessions cannot open PRs/push;
  finish with a patch to apply locally*. (The GitHub account that owns the remotes is not the one
  authenticated in a remote run; the durable home is the global `~/.claude/CLAUDE.md`, which a remote
  session can't edit, so it's restated per-repo.)
- **`collaboration-design.md`:** left as-is — it's the referenced code-audit appendix.

## Deliberately not done

- **No MASTERPLAN prose edits.** Its known-stale lines (auto-commit `:391` vs `:426`
  contradiction; the non-existent `update_body` mtime check `:319`; absent
  pulldown-cmark) are **flagged in `plan.md`'s "the code wins" section** rather than
  churned into a 51 KB canonical doc during a consolidation pass.
- **No feature code, no rename execution.** The rulings and phases are *recorded as
  decided*; implementing Phase 0's two remaining items (tolerant loader, git identity) or
  any Track S item is a later session that `plan.md` now drives.

## Left not-working

Everything `plan.md` lists as pending — most immediately, the two remaining Phase 0
items above, which gate any shared-vault work.
