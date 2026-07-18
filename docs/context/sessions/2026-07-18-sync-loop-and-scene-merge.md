# 2026-07-18 — An explicit sync loop, a whiteboard merge, and three plan items that must not be built as written

Second tranche of Track M's host-side band, same session as
`2026-07-18-dispatch-and-blob-route.md`. `pixi run ci` green.

**Outcome — shipped:**

1. **The sync loop is explicit** (ruling 8). `ui/src/lib/sync.svelte.ts`:
   `commit → push`, and on a rejection `pull → merge → push once more`. Exactly one
   retry, every terminal state nameable, and **never a push after a conflicted pull**.
   12 tests drive every branch through injected fake git ops.
2. **Liveness and reindex are two beats, not one** (`POST /api/alive`). The reindex
   beat went 3 s → 15 s and is now visible-tab-only; the watchdog's idle window went
   10 s → 90 s, which is what actually fixes the documented background-tab kill.
3. **Whiteboards merge element-wise** (`crates/fm-core/src/scene.rs`), before the text
   merge ever sees the JSON. 9 tests.
4. **Two live bugs fixed**: auto-commit committed only the *default* vault (a
   multi-vault install had one repo with a history and the rest with none), and the
   top-bar "get changes" pulled **without committing first**, which git refuses over a
   dirty tree — the backup panel already knew to commit first, so the same rule existed
   in two spellings.

## Why the sequence is not a loop

`push_squashed` is designed to refuse when the remote moved, to tell a human, and to
**never fetch** (fetching advances the tracking ref, and an un-advanced tracking ref is
what its ancestry guard survives on). Automate the push without automating the telling
and a moved remote becomes a rejected push retried forever, silently.

So: one retry. And the rule that makes it safe — **a pull that conflicts is never
followed by a push**, because publishing conflict markers as though they were content is
worse than not publishing. `commit_all` returns `Ok(false)` rather than an error when the
tree is unmerged, so a naive retry would push the same rejected HEAD forever; and `pull`
refuses outright when a merge is already half-finished, so it would fail identically every
time. Both are pinned by tests.

## Why whiteboards needed their own merge

A board's body is an Excalidraw scene: one big pretty-printed `elements` array,
re-serialized whole on every change. A *line* merge is close to the worst tool for it —
two people drawing in opposite corners share no shape but do share the punctuation between
them, so it returns either a spurious conflict or spliced JSON Excalidraw cannot parse.
The whole board, lost, because two people drew at once.

The atom of a scene is the element, so merge elements: higher `version` wins, lower
`versionNonce` breaks ties (deterministic on both machines, which is what stops the next
sync diverging again), fractional `index` decides z-order, file maps unite. And the rule
this exists for: **a deletion is honoured against the base**. That is precisely what
Excalidraw's own `reconcileElements` cannot do — it takes two scenes and no base, so it
cannot tell "you deleted this" from "I added this" and resolves both by keeping it. Wire
that into a git merge and every deleted shape returns on the next sync.

**Naming correction, and the docs were wrong:** there is no `.excalidraw` file in a vault.
`FileStore` writes `<ulid>.md`, and the existing `*.md merge=fm` attribute already routes
board notes into `merge_files`. This is a branch inside `merge.rs`, not a second driver —
`plan.md` and `collaboration-design.md` both implied a driver that would never have fired.

## Three plan items that must NOT be built as written

Audited before building, and each is now a blocked task rather than a silent skip.

**The git2 swap (rulings 3 + 6) rests on three wrong premises.**
- `git2::merge_file` **does not exist**. git2 0.20.4 exposes only
  `Repository::merge_file_from_index`, which needs `IndexEntry`s and would pollute the
  ODB — contradicting merge.rs's own design. The buffer API `git_merge_file` *is* bound in
  `libgit2-sys`, but git2 imports that crate privately, so ruling 3 needs a direct
  `libgit2-sys` dependency plus unsafe FFI. The plan says this was "verified against the
  git2-rs docs"; it was not.
- **libgit2 cannot invoke external merge drivers** — only text/union/binary are
  registered, and there is no process-spawn anywhere in it. Porting `pull()` would
  *silently disable* the `.md` frontmatter merge driver while a collaborator's terminal
  `git pull` still honours it: two merge semantics in one vault. This is the Phase 1
  achievement being undone. There is no workaround.
- `deny.toml` forbids linking GPL code ("GPL tools like pdftotext/libvips are invoked as
  subprocesses and never appear in this graph"). libgit2 is GPL-2.0-with-linking-exception
  and `cargo deny` passes it only because `libgit2-sys` **under-declares** as
  `MIT OR Apache-2.0` while vendoring ~230k lines of GPL C. `ci/third-party.sh` reads the
  same field, so we would ship binaries omitting a notice the exception requires.

**The Excalidraw image-strip (Ruling B)** would stop shared boards showing images:
`blobs/` is gitignored, so after the strip a scene carries only a hash and the bytes never
travel. `mobile-design.md` names two options and decides neither.

**"Swap in pragmatic-DnD's pointer adapter"** is not an option:
`@atlaskit/pragmatic-drag-and-drop` 2.0.1 ships three adapters (element, external,
text-selection) and none of its 12 companion packages provides a pointer one. So the
tap→move-to-column fallback was the only path — **and it was built** (below).

## And then the data-loss bug it exposed, fixed

The audit found that an open `NotePanel` never refetches after a pull, so its next debounced
save writes the pre-merge draft back over the merged file — a collaborator's text gone,
silently, with a clean history saying you wrote it. The sync work made it easier to reach,
so it was fixed in the same session.

**Why the guard that already existed did not cover it.** `FileStore::put` refuses a write
whose file moved on disk since we indexed it (`refuse_if_stale`, mtime-based). But `pull`
merges and then **reindexes** — it has to, because the views are served from SQLite and a
merge is otherwise invisible — and that reindex records the post-merge mtime. From `put`'s
point of view everything is in sync while the open pane still holds pre-merge text.
*Noticing the change is what disarms the protection against it.*

mtime cannot see this because the staleness is in the client, not on disk. So the client
declares it: `update_body` takes the `updated` stamp the caller last saw and returns the new
one, and a mismatch is `StoreError::Conflict`. On rejection `NotePanel` reloads and puts the
unsaved draft back **below** the merged text with markers — the `.md` driver's own stance,
one level up: both versions where a human can see them, never a silent choice. `mock.ts`
mirrors the guard deliberately, so the rejection path is exercised in dev.

Written as a **failing test first** (`crates/fm-app/tests/lost_update.rs`), and the test
earned its keep immediately: the first version of its helper rewrote the body while leaving
`updated` alone, which a real merge never does — so it was modelling a merge that cannot
happen and would have "proved" a guard that never fires. Verified against a live server too:
the stale save returns 500 and the collaborator's paragraph is still there.

**Residual gap, recorded:** the token is a timestamp, so a writer that changes a body
without bumping `updated` — hand-editing in Vim — is still invisible to it. Closing that
needs a content hash.

## The board works without a pointer now, and the shell fits a phone

Dragging a card is HTML5 drag, which never fires on touch, and there is no adapter to swap
in — so on a phone the board was simply inert. `Card` now offers a tap→move menu built from
real `<button>`s, and `Board` hands it `columns` + an `onmoveto` that calls the *same*
`onmove` a drop does (appending: a tap expresses a column, not a position). One write path,
not two.

Real buttons rather than touch handlers is the load-bearing choice: a jsdom click exercises
exactly what a tap does, so this is **testable without a phone**
(`ui/src/renderers/Card.touch.test.ts`, 5 tests — including that using the menu does not
also *open* the note, since the card itself is a button). What CI still cannot answer
shrinks to "is the target big enough for a finger".

It stays literal-free: the menu's labels are `col.label`, i.e. whatever the grouped
property's values happen to be. The CI grep over `ui/src/renderers` (case-insensitive,
whole-word `todo|doing|done`) would have failed on a "Done" label — worth knowing before
writing one.

The shell reflow is media queries only, no new stateful layout: the pane workspace collapses
to one column, the top bar wraps, the board snap-scrolls a column at a time, and
`pointer: coarse` bumps the 3px-padding targets to ~44px. `--cols` is **overridden rather
than read** — it is a desktop preference, and a workspace saved on a laptop must not arrive
on a phone as four 4rem columns.

Verified through to disk: the move writes `status:` into the note's own file.

## Three documented bugs closed on the way out

All three were found by audit during this session, recorded honestly, then fixed rather than
left as entries someone else would inherit.

- **A duplicated `id:` made the poll flap forever.** `objects.id` is the primary key and
  `index_object` is INSERT OR REPLACE, so two files claiming one id collapsed to a single row
  whose `path` alternated — every beat re-indexed whichever path the row was *not* pointing
  at, reported `updated: 1`, and the UI refreshed indefinitely on a vault nobody was
  touching. Now the first path wins and the other is **named** in `skipped`: serving one
  file's content under another's id is worse than serving neither, and that is the discipline
  the unreadable-note skip already set. The test asserts three consecutive quiet polls report
  nothing — a count, not a snapshot, because the bug *was* the repetition.
- **No `Host` validation.** Binding to 127.0.0.1 keeps other machines out but does not decide
  which *name* a browser used to arrive: a hostname an attacker controls, resolved to
  127.0.0.1, is same-origin with itself, so the CSRF guard waved it through and the page
  could read every response. Requests must now carry a Host we actually serve. **A missing
  Host still passes** — that is HTTP/1.0 or curl, not a browser, so not this vector.
  Verified live: `localhost` 200, `evil.attacker.test` 403, no-Host 200.
- **A surfaced error was not durable.** `refresh()` cleared `error` unconditionally and runs
  on every pane change and `changed` beat, so a sync failure could be wiped before it was
  read. It now clears only what it raised; anything about the user's data goes through
  `report()` and stays until dismissed.

## Left not-working
- **A surfaced error is not durable** — `refresh()` clears `error`, and refresh runs on any
  pane change.
- **Auto-push is still not automatic.** `syncVault` is wired to the backup panel's button
  and nothing fires it on a timer. Whether writing notes should publish them without being
  asked is an outward-facing default, and the shipped design says backup is "a
  conversation, not a fire-and-forget" — so it wants an owner's decision, not a default.
- **Not seen in a browser** (the extension is not connected): the sync states, the 15 s
  visible-only beat, and the scene merge are verified by tests and by `svelte-check`, not
  by eye. **The phone shell has never run on a phone** — and Chrome's touch emulation is
  actively misleading here, since it synthesises PointerEvents without reproducing Android's
  `dragstart` suppression, so emulation can hide the very bug the menu works around.
