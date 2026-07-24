# 2026-07-24 — proposals on the phone: the libgit2 port, and two designs that were wrong

## What was broken

A phone `/transcribe` failed with **"could not run git (is it installed?): Permission denied (os
error 13)"**. Android has no `git` binary (W^X), which is the whole reason `git_native` exists —
but notes and replies worked while *every proposal* died.

The cause: `vcs.rs`'s own doc comment says **"Always call through this, never `git`/`git_native`
directly"**, and the entire proposal lifecycle ignored it. `fm-app/src/commands.rs` named
`fm_core::git::…` at **15 lines / 11 distinct functions**, and `git_native` implemented **none** of
them. So `/transcribe`, `/propose` and `/research` all failed identically — on the one device where
a proposal is the *only* way a UI-only user merges anything.

This was the **second wave** of the same trap. The 2026-07-19 pass fixed the history call sites and
`overview.md` was written as though `vcs` were now the only route. It was not. A unit test cannot
catch a caller that stops calling, so `ci/checks.sh` now greps for it.

## What shipped

Ten functions ported to `git_native` (`create_proposal_branch`, `revise_proposal_branch`,
`proposal_load`, `push_branch`, `delete_branch`, `branch_open`, `file_on_branch`, `branch_diff`,
`merge_proposal_branch`, plus the private `write_proposal_branch`/`resolve_proposal_ref`), nine new
`route!` arms in `vcs.rs`, 15 call sites flipped, a CI grep, and 8 new tests.

Nothing was cut: every function was checked against the phone journey (create → view diff → edit
the proposed body → accept, and reject). Degrading accept to desktop-only fails *"from any device"*
— and `known-issues.md` already recorded "no in-app accept" as **fixed** when the fix was in fact
desktop-only.

## Two designs that were wrong, and why they looked right

The plan was audited by three agents before implementation. Two load-bearing errors survived
drafting and were caught there.

### 1. `commit(Some("refs/heads/<b>"))` for a revise — would not have worked at all

Reads as the natural `update-ref` equivalent. libgit2 requires the first parent to be that ref's
current tip, but a revise **re-parents on `HEAD`** while the branch still points at the previous
revision, so it is refused. Correct form is `commit(None)` + `reference(force)` — still one atomic
ref move, which is what the "an interrupted revise never loses the prior revision" guarantee
actually rests on. Loud and cheap; would have failed on first run.

### 2. Commit-then-`checkout_head(force)` for accept — quiet and expensive

The first design computed the merge in memory, committed, then `checkout_head(force)`, and claimed
to be *"strictly more fail-closed than the desktop"*. **Both halves were false.**

- **`checkout_head(force)` is not scoped to the merge.** Baseline and target are both the `HEAD`
  tree, so every delta is `UNMODIFIED` and only the *working tree* comparison drives action
  (`CHECKOUT_ACTION_IF(FORCE, UPDATE_BLOB, NONE)`). Every tracked file in the vault differing from
  `HEAD` is overwritten and every user-deleted file resurrected — merged or not. A user mid-edit in
  an **unrelated** note, debounce not yet fired, would silently lose it by tapping Accept. Exec
  preserves that file, and *refuses* when a merged or untracked path would be clobbered.
- **The argument for avoiding `repo.merge()` was inverted.** It was rejected as "the shape most
  likely to leave a phone mid-merge" — but that state is **detectable and abortable**
  (`MERGE_HEAD`, `conflicts()`, `repo.state()`). Commit-then-checkout leaves `HEAD` moved over a
  stale index with **no marker at all**, after which the next debounced `commit_all` writes a tree
  from that stale index and silently commits a **revert of the whole accepted proposal**,
  indistinguishable from a user edit. *An undetectable half-state is worse than a detectable one.*

**The shipped ordering: decide in memory → write working tree + index (path-scoped) → move `HEAD`
last.** No ordering is crash-free; this is the one whose failure mode is recoverable. Killed
between the checkout and the commit, the vault holds merged content with `HEAD` unmoved — the next
auto-commit records it as an ordinary commit, losing the merge *parent* and no bytes, and
re-accepting is idempotent.

Path-scoping the checkout also closed a residual desync: a merged path whose working tree already
equalled the merged content takes action `NONE` under a whole-tree checkout, leaving the index on
the pre-merge blob — and the next `commit_all` commits a revert of that one file. Reachable,
because `create_proposal` derives branch content from the live store object.

## The `merge=fm` driver does not exist on the phone

Verified in libgit2's source: `merge_driver_lookup_with_wildcard` knows only `text`/`union`/
`binary` and returns `NULL` for `fm`, after which `merge.c` sets `fallback = true` and re-invokes
the **built-in text driver**. Silently. So without compensation, essentially every accept of a note
that also moved on `main` would report a conflict on the manufactured `updated:` collision — the
exact bug the desktop driver exists to fix.

`git_native::pull` already solved this. The shared piece is now `merged_text()` — **the decision
only** (`manifest.json` → `Manifest::merge`, else `merge_texts(.., 7)`). The staging deliberately is
*not* shared: `pull` writes the file and `add_path`s into the repo-backed index, which the
ownerless index from `merge_trees` rejects outright — and copying it anyway would produce an accept
that **writes resolved notes to disk before deciding it is `Conflicted`**, mutating the vault on
the one path the design advertises as inert.

## Traps worth remembering

- **`Index::add` preserves stage bits.** It masks only `NAMEMASK` out of `flags`, so re-adding a
  conflict-derived entry re-inserts a stage-2/3 conflict and `write_tree_to` then fails
  `GIT_EUNMERGED`. Build the resolution entry fresh: `flags: 0`, `mode: 0o100644`.
- **`create_updated` returns an `Oid`, not a `Tree`.**
- **`pixi run ci` does not compile `git_native.rs` at all.** `test-native-git` is a separate opt-in
  task (it builds libgit2 + OpenSSL from source) and **stays out of `ci`** by design. The gate for
  this work is `ci` **+** `test-native-git` **+** `android-check`.
- **A fixture that is merely note-*shaped* hides the behaviour under test.** The `updated:`
  collision test first failed because the fixture lacked `id`/`type`/`created`, so all three sides
  failed to parse and the engine fell through to a line-based 3-way. It then failed again because
  the two edits were on **adjacent lines**, which diff3 folds into one conflicting hunk. Both are
  fixture artifacts that look exactly like a real regression.

## Declared asymmetries (in `vcs.rs`, not hidden)

- `branch_diff`'s **patch text** is not byte-identical — libgit2 renders its own. The file list is
  exact; the tests assert identical **trees**, not identical patch bytes.
- `merge_proposal_branch` uses a **single merge base** where git's `ort` recurses over all of them.
  In a criss-cross history the native accept can refuse where the desktop merges — fail-closed,
  never corrupting, and the edit-and-save escape hatch still works.

## The test that could not exist, and the blind spot it revealed

`collaboration_pipeline.rs` already ran two users through the real commands "on both backends" —
and its cross-user proposal test was **green throughout the period when proposals were completely
broken on a phone**. The reason: its `backends()` loop swaps only the `pull` function. Every other
call goes through `vcs`, which prefers a `git` binary whenever one exists, so both passes ran the
subprocess backend for committing and for the entire proposal lifecycle. The `[native]` label was
measuring nothing.

The gap is structural: `native()` means *"there is no git binary"*, which a developer machine can
never be — so the phone's real code path was **unreachable from any test that drives the app
commands**. `vcs::force_native()` (feature-gated, inert in production, since Android is already
native) closes that, and `mixed_device_collaboration.rs` uses it to pin the **device** rather than
just the merge function: a laptop with the real `fm` merge driver and a phone with no binary at
all, sharing one bare remote, driven entirely through `fm_app::commands`.

Eight journeys, all passing: propose on phone → accept on laptop; propose on laptop → accept on
phone (against `origin/proposal/<id>`, with no local branch); accept on the phone while `main`
moved underneath (the cross-device `merge=fm` equivalence — both devices must converge on the same
bytes though one shells out to a driver and the other cannot spawn a process); reject; revise;
offline propose-and-accept with no remote; accepting while mid-edit in an unrelated note; and two
open proposals accepted in sequence.

**One finding, which turned out to be my own test's fault — recorded because the mistake is the
lesson.** The reject scenario appeared to show a cross-device bug: the proposer's clone still
offered a proposal someone else had rejected. It reproduced on both backends, which made it look
like a real pre-existing gap, and it was nearly filed as one. It was not. `reject_proposal` marks
the immortal proposal *note* `declined: true` and that flag rides `main` like any other edit;
`proposal_diff` checks it **before** it consults git at all, so the leftover local branch is inert.
The test had simply not committed the rejection, so the decision never left the device. **A
scenario test that skips the commit the app performs automatically is not testing the app** — it
invents failures that no user can reach.

## Status

`pixi run ci`, `pixi run test-native-git` (19 differential + 8 mixed-device + 4 collaboration + 2 native-merge), and
`pixi run android-check` all green. **Not yet verified on the device** — APK rebuild and a real
phone `/transcribe` → proposal → accept is the remaining step.
