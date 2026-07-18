# 2026-07-18 — A vault can be a repo you already have, and the queue that came out of it

Third and last stretch of a long session (after `2026-07-18-dispatch-and-blob-route.md` and
`2026-07-18-sync-loop-and-scene-merge.md`). `pixi run ci` green throughout; five commits.

**Outcome:** Track V's V2 and V3 shipped, both blocked decisions were settled under the
project's own principles, an adversarial doc sweep produced a work queue, and the queue was
then worked to empty.

## Track V — a vault stops needing to be a folder made for it

**V2, three of four** (both silent ones among them). Each was a live bug the moment a vault is
also a repo you own, which is exactly where Track V is heading:

- `.gitattributes`/`.gitignore` were **skipped when the file already existed**. That reads as
  politeness and behaves as sabotage: every real repo has both, so `*.md merge=fm` never
  landed and the merge driver silently never engaged — Phase 1's disaster reintroduced by
  conversion — while `blobs/` and `index.sqlite` were committed and pushed. Both writers
  append what is missing now, and touch nothing else.
- `commit_all` did `git add -A` every five seconds. In a project repo that is a second author:
  it staged your half-written function and whatever you had carefully staged for a commit of
  your own. It now stages **exactly the paths `put`/`delete` recorded**, which also stops it
  catching a note you are hand-editing in Vim. Deliberately not on the `Store` trait — that
  seam carries no paths, and that is why a storage swap is a backend change.
- `push_squashed` collapsed *every* unpushed commit. The 5 s auto-commit justifies squashing
  **ours**; it never justified turning three hand-written manuscript commits into one
  `backup:`. The boundary is the `auto:`/`backup:` message prefix — **never the author**,
  since we commit *as* the user and an author test would classify everything as ours.

**V3 — `vault.json`.** Three fields, bounded by one rule: *every field must be a fact git
cannot supply*. So `name`, `description`, and **where the notes are** — no author, no
collaborators, no remote, no history, because git already knows those. A project whose notes
live in `docs/` is adopted with no import step, and new notes land beside the existing ones.

Two things the tests taught that reasoning had not: name precedence is **caller > descriptor >
directory** (`FileStore::open` was passing the directory name, which made it an *opinion* so
the descriptor could never win — when it is in fact the weakest of the three, being whatever
git called the clone); and `list_vaults` had to fill a blank config name from the store, or
adopting a repo showed a vault called `""`.

## The two blocked decisions, settled

Asked to decide them on the project's own principles, and both came out the same way the
principles point.

**`git2` is rejected; "git is a capability, not a dependency" stands.** Every principle
agreed, which is what made it a decision rather than a preference: linking libgit2 is the
opposite of *shell out, don't link*; `deny.toml` forbids linking GPL and libgit2 clears it
only because `libgit2-sys` under-declares as `MIT OR Apache-2.0` while vendoring ~230k lines
of GPL C; **libgit2 cannot invoke external merge drivers**, so porting `pull()` would silently
disable the `.md` merge while a collaborator's terminal git still honoured it; and ruling 3's
real shape is unsafe FFI around a function `git2` keeps private. Mobile's missing git backend
is left unsolved *on purpose*.

**Whiteboard blobs will be git-tracked; the strip is deferred.** Storage: one `.gitignore`
exception beats a blob mirror, which is a sync framework by another name. The strip waits
because boards already sync un-stripped, so what is left is churn and not correctness, and it
touches the one view whose failure mode is "your drawing is gone".

## The sweep, and the queue it produced

An adversarial pass over the docs — six finders, six refuters, three completeness critics.
Findings were applied only after a refuter confirmed them; two were marked WRONG and dropped,
and several more turned out to be artifacts of my own greps (a regex missing hyphens, `sort`
without `-u`), which is the argument for the refute step existing at all.

The worst was in the canonical spec: MASTERPLAN listed **`git2` 0.21.0** as a dependency,
annotated *"libgit2: GPL-2 with linking exception → safe"* — a crate never added, justified by
exactly the reasoning that later rejected it. It also still called the app "single-user,
local-only" after Track C shipped shared vaults. The manual said the same and never mentioned
collaboration, which is odd for a tool whose *name* is the plural.

That produced `outstanding.md`, and the rest of the session was spent emptying it: unreadable
notes surfaced in the UI instead of on stderr; the inert board column drag reconnected;
restic scoped to the vault's own directories; `fm-serve`'s guards given socket-level tests;
the mock's arms typed; the write-list threaded; and the lost-update token changed from a
timestamp to a content hash.

## Three findings worth keeping

- **A unit test cannot catch a caller that stops calling.** `boardOrder.ts` was a pure, tested
  core that nothing imported any more — the pane rewrite dropped the wiring and kept the
  tests, so dragging a column header did nothing while the drag still started and the cursor
  still said `grab`. The new tests pin the *sequence the caller performs*, not the functions.
- **A test written after the fix agrees with the fix.** Two here were written first and both
  caught something the reasoning missed: `Host` validation split on `:` and so refused
  `[::1]:8765`, a request from ourselves; and the `vault.json` name could never win because
  `open` passed the directory name as an opinion. A third — my first attempt to mutation-test
  the content hash — was *invalid*, passing for the wrong reason, and had to be replaced by
  asserting the crux directly.
- **The queue's own instruction outranked the queue.** It said migrate `fm-cli` onto
  `dispatch`. `dispatch` is a JSON wire surface and a CLI wants typed values, so routing
  `fm show` through it to re-parse would have been worse than the fork. The debt that actually
  cost something was duplicated *logic* — `Cmd::Add` rebuilt the asset note, and its copy had
  already drifted: it never set `obj.vault`, so a file added from the command line was stamped
  with no audience. Shared, not routed.

## Left not-working

- **Nothing here has been seen in a browser** — now the top entry in `outstanding.md`, because
  the volume of unobserved UI is the largest single risk in the project.
- **V4 (adoption)** is the largest unbuilt item; **M0–M8** are blocked on the Android toolchain
  *and* on a git backend that the `git2` rejection deliberately leaves open.
- `.view` accepts `search`/`gallery` but both render as a timeline. Documented, not fixed.
