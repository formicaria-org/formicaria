# 2026-07-22 — collaboration completed through the GUI, and a two-user pipeline harness

The goal the owner set: **functional collaboration through the GUI** (the user does everything in the
UI — no CLI, no external editor, especially on the phone — see the `ui-only-no-backend-access` memory),
and **device-agnostic tests that simulate the whole multi-user pipeline**.

## Two GUI gaps closed (backend + UI + tests)

1. **Accept a proposal in the GUI.** Review was read-only; accepting was a manual `git merge` — a
   dead end for a UI-only user. New `accept_proposal` (`commands`/`dispatch`) →
   `fm_core::git::merge_proposal_branch` (no-ff, deletes the branch), **fail-closed**: an unclean merge
   is aborted and `main` left untouched (`Accepted::Conflicted`), never a half-merged tree.
   `ProposalReview.svelte` gained an **Accept & merge** button. Tests: `proposal_cycle.rs`
   (`accept_proposal_the_gui_button_merges_the_branch_into_main`) + a `ProposalReview.svelte.test.ts`
   case. (Cross-*user* accept is still out of scope — a `proposal/<id>` branch only lives on the
   proposer's clone; a push sends `main`, not `proposal/*`.)

2. **Resolve a conflicted / unrenderable note in the GUI.** A frontmatter conflict stays
   "loud-and-absent" (decisions.md #4 / `merge.rs:32-35` — the parser stays strict, **no side-picking
   by fiat**), but `SkippedPanel` now has an **in-app raw editor**: `read_skipped` / `resolve_skipped`
   (same allowlist as `open_skipped`) load the raw file, show a live "has conflict markers?" indicator,
   and save the user's resolution — on **any device**. `open_skipped`→external editor is now a
   desktop-only convenience. Tests in `fm-app/tests/skipped.rs` (read/resolve/parse-status/security) +
   `SkippedPanel.svelte.test.ts`.

**Reverted** an earlier attempt to auto-resolve a frontmatter conflict by taking "ours" — that is the
exact data loss `merge.rs:32-35` and decisions.md #4 forbid. The lesson is in those two places already;
the fix is the UI surface, not a parser change.

Also hardened `Agent::history` to pin the first turn (the original ask) + the latest when the thread
overflows, and shipped the propose→refine→accept cycle tests (`proposal_cycle.rs`) — see that file and
the earlier session entry.

## The two-user pipeline harness — `crates/fm-cli/tests/collaboration_pipeline.rs`

The owner's model, realised: a **bare remote + two clones (Ada, Ravi)**, each a real `FileStore`,
edited through the real `fm_app::commands`, synced by real `git` push/pull. It lives in `fm-cli/tests`
because that is the one place with **both** `CARGO_BIN_EXE_fm` (the merge driver) **and**
`fm_app::commands`. Every scenario runs on **both merge backends**: the subprocess `fm` driver
(desktop) always, and the native in-process merge (phone) under `--features native-git`.

Scenarios (all green on both backends):
- a note one user creates reaches the other after a sync;
- concurrent edits to **different** lines merge cleanly (the manufactured `updated:` collision is a
  non-event — the driver's whole job);
- concurrent edits to the **same** line conflict **visibly and resolvably**: the note still parses and
  renders (not skipped), both sides are kept in the body, it is listed by `conflicts()`, and resolving
  it converges both users.

**The load-bearing trick:** `git::pull`/`commit_all` call `ensure_repo`, which resolves the driver via
`current_exe().parent()/fm`. In a test that directory is `target/debug/deps/`, which has no `fm`, so
the driver was being *cleared* on every real-code-path call and plain merges conflicted on `updated:`.
Fix: copy the built `fm` beside the test binary (`ensure_fm_beside_test_binary`) so the real driver path
works unmodified — rather than side-stepping it with a raw `git merge` the way `merge.rs` does.

CI: the driver backend runs in `pixi run ci` (`test` = `cargo test --workspace`); the native backend is
added to the `test-native-git` task (native-git is a separate opt-in gate, as for `git_native_merge`).

## Not yet done
- **Cross-user proposal review/accept** (needs pushing the `proposal/*` branch, or a different transport).
- More pipeline combinations (access-type matrix: private vs shared vaults; three-way; delete/rename races).
- Both devices were rebuilt + installed with the GUI-collaboration commit (`a1ae28f`).
