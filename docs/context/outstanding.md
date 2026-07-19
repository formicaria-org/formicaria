# Outstanding — the work this codebase knows it owes

A ranked queue, so the next session picks up the most valuable thing rather than the most
recent one. Everything here was found by reading the code or by an audit, not hypothesised.

**This file is a queue, not a log.** When an entry is fixed, *delete it* and fold the durable
outcome into `known-issues.md` or `decisions.md` — the history lives in `sessions/` and in
git. The first version of this file kept its fixed entries struck through, and within a day it
had become a changelog with nothing to do in it. That is the failure mode to avoid.

_Last reconciled: 2026-07-20, after capture ran on a real phone
(`sessions/2026-07-20-capture-on-a-real-phone.md`) — 1.2 is the open thread and the next action is
to read a placeholder, not to write code. Prior: 2026-07-18, after the whole compiled queue was
worked through (`sessions/2026-07-18-vault-as-a-repo-and-the-queue.md`)._

---

## 1. The one that gates everything else being trustworthy

### 1.1 None of this has been seen in a browser
The extension is not connected, so every UI change in the last stretch — the sync states, the
skipped-notes banner, the tap-to-move menu, the phone reflow, the reconnected column drag, the
conflict-marker reinsert on a refused save — is verified by tests, by `svelte-check`, and by
driving the HTTP API. **Not by eye.** The pane workspace has never been visually checked
either, and `decisions.md` records that its layout is deliberately not CI-verifiable.

**Done looks like:** an hour with the app open, exercising each of those paths once. **This is
the highest-value item in the file** — not because anything is known to be broken, but because
the volume of unobserved UI is now the largest single risk in the project, and everything
below is cheaper to judge once it is gone.

**◐ Partly retired 2026-07-19** (`sessions/2026-07-19-the-phone.md`). The app has now been seen
by eye **on a real phone**, over `adb reverse` against the real backend, and the first finding is
already load-bearing: *the current GUI is not good for small screens* — enough to move the
single-column touch work **ahead of** packaging. Still unobserved, so this item stays open: the
pane workspace on a desktop, and on the phone each specific path it was opened for — Board
scroll-snap vs finger drag, the tap→move menu, a `<video>` seeking mid-file (the only exercise of
`blob.rs`'s `Range` on real hardware), Excalidraw under a finger, thumb reach on the 2.75rem
targets.

### 1.2 A captured photo does not render on the phone — **start here**
Capture, ingest and the blob write are verified on real hardware (2026-07-20); display is not.
The note shows the filename as text where the image should be. This is the last step of the
feature that the owner said *"determines the usability of the mobile app"*, so nothing else in
this file outranks it.

**The next action is to read, not to build.** A build that renders the failure reason in the
placeholder is already installed on the phone (MD5 `d0796a29b271338e796e36b089b36a4e`, verified).
Open the note and read the text after the dash:

- `no bytes in vault "…"` → ingest returned a hash but nothing landed; follow the POST arm in
  `mobile/src-tauri/src/lib.rs` and which vault `dispatch("ingest", …)` resolves `""` to.
- `the vault has this blob but it read back empty` → the write is fine, the read is not.
- anything else → `asset_status` threw and that text is the error.

**Two dead ends already paid for, do not repeat them:** it is *not* a vault mismatch
(`dispatch.rs`'s `asset_status` searches every registered vault) and *not* the reference format
(`parse_ref` accepts `sha256-`). Both were asserted before being checked, and both were wrong.
Nor is `console.warn` a way to find out — that device logs no JS at all.

**Done looks like:** a photo taken on the phone appears in the note, and a test covers whichever
seam turns out to be broken.

---

## 2. Not started

### 2.1 Track V4 — adoption
Any `.md` reads for free with a **transient, index-only id**; the first time you cite or edit
it, it is stamped with a real ULID. This is what makes "point formicaria at every repo you own"
actually free — ten commits putting ULIDs into READMEs your collaborators read is a bad trade.
It also retires the objection to derived ids (*they break links when a file moves*), which only
bites if you can link to them, and you cannot until promotion stamps a real one.

The trap it must solve, from `plan.md`: `path_for(id) = notes.join("{id}.md")` — the filename
*is* the id, so `docs/installation.md` reads perfectly and the first edit writes
`docs/01KX….md`, orphaning the original. The index has to record the real path.

**The largest unbuilt item in the plan**, and the natural next feature now V1–V3 are in.

### 2.2 The whiteboard image-strip
Decided and deferred, both on purpose — see `decisions.md`. Storage is settled (git-track the
whiteboard blobs as a scoped exception, over a blob mirror). The strip itself waits because
boards sync un-stripped today, so what remains is churn rather than correctness; the cost is
felt on a phone that does not exist yet; and it touches the one view whose failure mode is
"your drawing is gone", which cannot be verified without eyes on a canvas — see 1.1.

### 2.3 Track M — M0–M8
Blocked on the Android toolchain, **and now also on choosing a git backend**: `git2` is
rejected (`decisions.md`) and a phone has no `git` binary to shell out to. The options are
recorded and deliberately not chosen, because pre-committing a backend for a platform that
does not exist is how the wrong one gets built.

---

## 3. Known and accepted — do not "fix" without deciding

Recorded so nobody spends a session on these thinking they are bugs.

- **Auto-commit dies with the tab.** A browser `setTimeout`, and with `FM_AUTO_SHUTDOWN`
  closing the tab *is* how you quit — so "edit, then close" can skip that commit. Files are
  never at risk (atomic temp+rename); our commits lag. Fixing it means `beforeunload`, which is
  unreliable by design.
- **A note edited outside the app is never committed by the app.** The deliberate flip side of
  staging only what `put`/`delete` wrote: it is your edit, in your repo, and yours to commit.
- **No per-view object cache.** Board/Agenda/Timeline each parse the corpus per request. Fine
  at this scale; gate any work on a perf budget test, as the poll fix was.
- **`fm-cli` does not route through `dispatch`,** and should not: that is a JSON wire surface
  and a CLI wants typed values to print. The duplicated *logic* was the debt, and it is shared
  now. Five of its commands are CLI-only by design, and `merge-md` runs before a store is
  opened because git invokes it as the merge driver.
- **The `.view` renderer set is three, not five.** `search` and `gallery` still parse but fall
  through to the timeline; said plainly in the manual rather than silently tolerated.
