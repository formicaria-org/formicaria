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

### 1.1 The auto-commit can still commit a note you are typing in Vim
`git.rs::commit_all` stages the **vault's own directories** (`notes`/`views`/`manifest.json`/
`.gitattributes`/`.gitignore`) — which stops it sweeping a surrounding project's half-written
code and stops it destroying a curated index. But the plan's actual requirement was narrower:
*only the files `put()` just wrote*. Right now, hand-editing a note in Vim while the app is
open means the debounced 5 s commit can catch it mid-sentence.

**Done looks like:** `FileStore` accumulates the paths it wrote, `commit_all` takes that list,
and a hand-edit is never staged by us. **Cost:** threading a write-list from `put` through
`dispatch` to `git::commit_all`. *(Track V2.4, half-fixed.)*

### 1.2 restic snapshots the whole vault path
`backup.rs` snapshots the vault root, so a vault that is also a project repo puts your
`data/`, your `.env` and your `.git` into whatever restic repo the vault is pointed at — which
for a lab vault may not be yours. It excludes only `index.sqlite` and `derived/`.

**Done looks like:** snapshot the notes dir + blobs dir, not the root. **Cost:** small, but it
changes what an existing restic repo contains, so it wants a line in the backup panel.
*(Track V2.4, second half — never started.)*

### 1.3 The lost-update guard is a timestamp, so Vim is invisible to it
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

### 3.1 Board column reorder is inert
`Pane.svelte:271` passes `onreorder={() => {}}` to `Board`, so dragging a column header in the
pane workspace does nothing at all. The drag still starts, the cursor still says `grab`, and
`localStorage['fm-board-order']` is still read on the way in — so the affordance is fully
present and fully dead.

**Done looks like:** either wire it (persist per group-by, as the card order already does) or
remove the affordance. Leaving a live-looking drag that does nothing is the worse of the two.

---

## 4. Structural debt, in the order it will hurt

### 4.1 `fm-cli` never migrated onto `fm_app::dispatch`
`dispatch` exists and `fm-serve` is a shell over it, but `fm-cli` still calls `fm-core`
directly and re-implements the command flows (`Cmd::Add` rebuilds the ingest path rather than
calling `commands::ingest`). So "one command library" is true of one frontend out of two, and
a change to a command's behaviour still has to be made twice.

**Done looks like:** `fm-cli` builds an `App` and calls `dispatch`. **Cost:** medium; its
tests (`cli.rs`, `merge.rs`) are the safety net, and `merge.rs` must keep driving the real
driver.

### 4.2 `fm-serve` is only partly tested
The blob route has real socket-level tests and the query-args split has unit tests. The CSRF
guard, the new `Host` guard, static file serving and the auto-shutdown watchdog have none —
and every UI test runs against `mock.ts`, so the real HTTP path is otherwise only exercised by
hand.

**Done looks like:** the guards get tests in the same shape as `blob.rs`'s (bind port 0, drive
a real socket, assert on raw bytes). The harness already exists, which makes this cheap.

### 4.3 The mock can drift from the real contract silently
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
