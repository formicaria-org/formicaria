# Outstanding — the work this codebase knows it owes

A single ranked list of what is **known to be wrong or missing**, so the next session picks
up the most valuable thing rather than the most recent one. Everything here was found by
reading the code or by an audit, not hypothesised — each entry names the file and says what
"done" looks like.

This is deliberately separate from [known-issues.md](./known-issues.md), which is the honest
*current-state* description of rough edges (including ones we have decided to live with).
**This file is a work queue**: when an entry is fixed, delete it here and, if the outcome is
durable, fold a line into `known-issues.md` or `decisions.md`.

_Compiled 2026-07-18 from the multi-agent doc sweep + the Track V/M code audits._

---

## 1. Data integrity — fix these first

### 1.1 ~~The auto-commit can still commit a note you are typing in Vim~~ — FIXED 2026-07-18
`FileStore::put`/`delete` record every path they touch; `MultiStore` exposes them per vault;
`commit_all` takes that list and stages exactly it. Cleared only after a commit actually
lands, so a failed commit does not forget what it owed.

**Deliberately not on the `Store` trait** — that seam carries no paths, no mtimes and no
directory handles, and it is the reason a storage swap stays a backend change. `Vaults.store`
is a concrete `MultiStore`, so it reaches them without widening the seam.

**The trade, stated:** a note edited outside the app is now never committed *by* the app.
That is intended — it is your edit, in your repo, and yours to commit — and it makes true
what `known-issues.md` already claimed about `fm-cli`/Vim writes. Pinned by a test that
half-writes a note "in Vim" and asserts it is absent from the commit.

The original entry:
`git.rs::commit_all` stages the **vault's own directories** (`notes`/`views`/`manifest.json`/
`.gitattributes`/`.gitignore`) — which stops it sweeping a surrounding project's half-written
code and stops it destroying a curated index. But the plan's actual requirement was narrower:
*only the files `put()` just wrote*. Right now, hand-editing a note in Vim while the app is
open means the debounced 5 s commit can catch it mid-sentence.

**Done looks like:** `FileStore` accumulates the paths it wrote, `commit_all` takes that list,
and a hand-edit is never staged by us. **Cost:** threading a write-list from `put` through
`dispatch` to `git::commit_all`. *(Track V2.4, half-fixed.)*

### 1.2 ~~restic snapshots the whole vault path~~ — FIXED 2026-07-18
`backup` now takes the vault's **own** directories — its notes dir (wherever `vault.json` puts
it) and `blobs/` — instead of the root. Naming what is ours beats excluding what is not,
because the set to exclude has no end. The notes dir is derived inside `backup()` rather than
passed in, so its three callers cannot drift on which directories count. Pinned by a test that
puts a `.env` and a `src/lib.rs` beside the notes and asserts they are absent from the
snapshot while the notes and blobs are present.

### 1.3 ~~The lost-update guard is a timestamp, so Vim is invisible to it~~ — FIXED 2026-07-18
The base token is the **sha256 of the body** now, carried on `NoteDetail.version` and returned
by `update_body`. `updated` only moves for writers that bump it — the app does, the `.md`
merge driver does, Vim does not — and `put`'s mtime guard is disarmed by the poll's own
reindex seconds later. Content cannot lie about whether the body moved.

**Measured before committing to it, as this entry asked:** 1.6 ms in release, ~41 ms in debug,
for a 2.8 MB whiteboard body — against a 600 ms save debounce that then writes and fsyncs that
same body, so the hash is comparable to the write it precedes rather than a new cost. Pinned
by a perf budget, and the test asserts the crux directly: after a Vim-style edit `updated` is
unchanged while `version` has moved.

The original entry:
`update_body` refuses a write whose `base` (`updated`) has been superseded, which closes the
pull-merged-under-an-open-editor hole. But a writer that changes a body **without** bumping
`updated` — hand-editing in Vim — is invisible to it, and `FileStore::put`'s mtime guard is
disarmed by the poll's own reindex.

**Done looks like:** the base token is a content hash rather than a timestamp. **Cost:**
hashing the body on read and on write; measure it against the 600 ms whiteboard save path
before committing to it.

---

## 2. The app knows something is wrong and does not say so

### 2.1 ~~An unreadable note is stderr-only~~ — FIXED 2026-07-18
`ping` now carries `skipped`, and the UI states it: *"N note(s) could not be read and are
missing from every view — usually a conflicted merge"*, naming each one. Keyed on the set
rather than a count, so a persistent conflict is not a notification every fifteen seconds and
a *new* one is not swallowed because an older one is already showing. Pinned by a test that a
note breaking **mid-session** is reported — a conflict arrives when a pull lands, not at
startup, which is exactly what stderr-at-startup could never say.

The original entry:
`FileStore::skipped()` names every note that could not be parsed — a conflicted merge is the
usual cause — and `fm-serve` prints it to **stderr at startup**. In the browser, which is the
product, those notes are simply absent from every view with no explanation. This is the one
place the app knows a note is missing and tells nobody who is looking.

**Done looks like:** `skipped` reaches the UI (it is already on the store; `ping` could carry
a count) and the app says "3 notes could not be read" with their names. **Cost:** small — a
field on an existing DTO plus a banner. **This is the highest-value item in the file**: it is
cheap, and it converts a silent absence into a fixable one.

---

## 3. Things that silently do nothing

### 3.1 ~~Board column reorder is inert~~ — FIXED 2026-07-18
`Pane.svelte` passed `onreorder={() => {}}`, so dragging a column header did nothing while the
drag still started and the cursor still said `grab`. `boardOrder.ts` — a pure, *tested* core —
had simply stopped being imported when the pane rewrite landed, which is the sharp lesson
here: **a unit test cannot catch a caller that stops calling.** Reconnected, keyed by each
pane's own `groupBy`, and the tests now pin the *sequence* the caller performs (drag, persist
the whole permutation, re-apply on render, compose a second drag against what is on screen)
rather than only the functions.

---

## 4. Structural debt, in the order it will hurt

### 4.1 ~~`fm-cli` never migrated onto `fm_app::dispatch`~~ — RESOLVED 2026-07-18, differently
**Not by routing it through `dispatch`.** That is a *wire* surface — JSON in, JSON out — and a
CLI wants typed values to print. Sending `fm show` through it to re-parse the JSON back would
have been worse than the fork it was meant to remove. Only 7 of the CLI's 13 commands even
have an arm; `verify`/`manifest`/`restore`/`check`/`reindex` are CLI-only by design and
`merge-md` runs *before* a store is opened, because git invokes it as the merge driver.

**What actually cost anything was duplicated logic**, and that is fixed: `Cmd::Add` rebuilt
the asset note — title, blob hash, MIME, put, thumbnail — in five lines that already existed
in `commands::ingest`. The copies had diverged: the CLI's never set `obj.vault`, so a file
added from the command line was stamped with no audience. Both now call
`commands::asset_note`. The CLI still streams from a path (a large file is never held whole)
while `ingest` takes bytes — that difference is real and kept; the note is not.

The original entry:
`dispatch` exists and `fm-serve` is a shell over it, but `fm-cli` still calls `fm-core`
directly and re-implements the command flows (`Cmd::Add` rebuilds the ingest path rather than
calling `commands::ingest`). So "one command library" is true of one frontend out of two, and
a change to a command's behaviour still has to be made twice.

**Done looks like:** `fm-cli` builds an `App` and calls `dispatch`. **Cost:** medium; its
tests (`cli.rs`, `merge.rs`) are the safety net, and `merge.rs` must keep driving the real
driver.

### 4.2 ~~`fm-serve` is only partly tested~~ — FIXED 2026-07-18
The CSRF guard, the `Host` guard, path traversal, method rejection and `/api/alive` now have
socket-level tests in the same shape as `blob.rs`'s. **They caught a bug on the first run:**
the `Host` guard split on `:` to strip the port, so a bracketed IPv6 literal (`[::1]:8765`)
yielded `[` and a request from ourselves was refused. The original entry:
The blob route has real socket-level tests and the query-args split has unit tests. The CSRF
guard, the new `Host` guard, static file serving and the auto-shutdown watchdog have none —
and every UI test runs against `mock.ts`, so the real HTTP path is otherwise only exercised by
hand.

**Done looks like:** the guards get tests in the same shape as `blob.rs`'s (bind port 0, drive
a real socket, assert on raw bytes). The harness already exists, which makes this cheap.

### 4.3 ~~The mock can drift from the real contract silently~~ — FIXED 2026-07-18
Every structured arm binds its value to the real DTO before the `as T`. The cast is forced by
the generic signature and cannot go away, but binding first is what makes a wrong shape fail
`check-ui`. Verified by reintroducing the exact historical drift — a top-level `restic_repo` —
and watching it error. The original entry:
`mock.ts` returns `… as T`, casting the type check away. It kept a top-level `restic_repo`
long after restic became per-vault and `tsc` said nothing. `backup_status` and now
`update_body` are typed properly; the other arms are still bare casts.

**Done looks like:** each arm builds its typed DTO first. **Cost:** mechanical, and it is what
stops a UI test passing against a shape the Rust never sends.

---

## 5. Known and accepted (do not "fix" without deciding)

These are recorded so nobody spends a session on them thinking they are bugs.

- **Auto-commit dies with the tab.** It is a browser `setTimeout`, and with `FM_AUTO_SHUTDOWN`
  closing the tab *is* how you quit — so "edit, then close" can skip that commit. The next
  commit sweeps it up (`add` is `-A` within our paths), so files are never at risk; commits
  lag. Fixing it properly means committing on `beforeunload`, which is unreliable by design.
- **No per-view object cache.** Board/Agenda/Timeline each parse the corpus per request.
  Fine at this scale; gate any work on a perf budget test, as the poll fix was.
- **The phone shell has never run on a phone**, and Chrome's touch emulation is actively
  misleading for the one bug it exists to work around.
- **The whiteboard image-strip is deferred**, and the storage question is decided
  (git-track the blobs). See `decisions.md`.
- **`git2` is rejected.** Mobile therefore has no git backend, deliberately unsolved.

---

## 6. Not started

- **Track V4 — adoption.** Any `.md` reads for free with a transient, index-only id; the
  first time you cite or edit it, it is stamped with a real ULID. This is what makes "point
  formicaria at every repo you own" actually free, and it is the largest unbuilt item in the
  plan. See `plan.md`.
- **Track M M0–M8** — blocked on the Android toolchain, and now also on choosing a git
  backend for a platform with no `git` binary.
