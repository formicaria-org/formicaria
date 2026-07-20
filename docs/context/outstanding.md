# Outstanding — the work this codebase knows it owes

A ranked queue, so the next session picks up the most valuable thing rather than the most
recent one. Everything here was found by reading the code or by an audit, not hypothesised.

**This file is a queue, not a log.** When an entry is fixed, *delete it* and fold the durable
outcome into `known-issues.md` or `decisions.md` — the history lives in `sessions/` and in
git. The first version of this file kept its fixed entries struck through, and within a day it
had become a changelog with nothing to do in it. That is the failure mode to avoid.

_Last reconciled: 2026-07-20 (evening), after four defects were fixed, **re-reviewed
adversarially, and four of the fixes found incomplete and repaired**
(`sessions/2026-07-20-four-bugs-a-review-found.md`) — frontmatter leaking on copy (still leaking
via `status`/`tags`), no CSP (and none on the phone at all), `manifest.json` conflicting as text
(and its first merge rule was wrong for a gitignored, per-machine store), and a conflicted note
silently freezing all commits (fixed everywhere but the auto-commit that actually runs). A
threads backend was built and reverted in the same session; see §2.5. The Android CSP **loads clean on the real phone** (installed and launched, zero
violations), but an image-bearing note and a whiteboard are still unopened — see §1.1. Earlier
the same day: media working on Android end to end (`sessions/2026-07-20-capture-on-a-real-phone.md`);
photos are done, 1.2 is now video, gated on where a phone's blobs survive rather than on the
transport. Prior: 2026-07-18, after the whole compiled queue was worked through
(`sessions/2026-07-18-vault-as-a-repo-and-the-queue.md`)._

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

**Two of those now also gate a security claim** (2026-07-20): the Android CSP was installed and
launches clean, but `img-src` is only really proven by **a note with an image** (Android rewrites
`fmblob://` to `http://fmblob.localhost`) and `worker-src`/fonts by **a whiteboard**. If either
policy is wrong the symptom is a broken image, not a crash — so it will not announce itself.

### 1.2 Video on Android, and the storage question behind it
Photos work end to end now (2026-07-20). Video does not: bytes reach the app only as base64 in a
JSON string — the one binary transport Android leaves open — so `MAX_INGEST` caps an attachment at
48 MB and anything larger is **refused with an explanation** rather than crashing. A phone video
clears that in seconds.

Two pieces, and the second is the one that matters:

1. **Chunked ingest** would lift the ceiling: send the file in bounded slices and append, so peak
   memory is one chunk rather than 1.33× the file.
2. **Where the bytes then live.** A phone vault is app-private storage, wiped on uninstall;
   `blobs/` is gitignored so a push does not carry it; restic is a binary Android does not have.
   Lifting the size cap without answering this means inviting people to put the only copy of a
   50 MB video somewhere one uninstall erases. **Do not do 1 before 2.**

**Done looks like:** a video attaches on a phone, and there is an honest answer to where its bytes
survive.

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

### 2.4 Image → LaTeX (parked 2026-07-20, researched, not started)

Photograph or screenshot a formula; get LaTeX in the note. The owner intends to **train the model
and own the architecture**; this entry exists so the constraints found while surveying it are not
re-derived later.

**The licence constraint decides the design.** The target includes the phone, and Android has no
subprocesses — so the model must run *inside* the WebView, i.e. ship inside an MIT app. That is
distribution, not invocation, so the exemption that lets us shell out to GPL `pdftotext`/`libvips`
does **not** apply (`deny.toml` states the rule in its first line). Consequence: **Texo** — the best
lightweight option at 20M params, ONNX, in-browser — is **AGPL-3.0** and cannot be bundled. Training
our own dissolves the problem, and Texo demonstrates 20M params suffices, so it is a reproduction
rather than a research gamble.

**Cost of shipping a model.** 4 bytes/param at fp32, 1 at int8 → **20M params ≈ 20 MB int8**
(UniMERNet-tiny's 441 MB is ~110M at fp32). **PyTorch is a training dependency only**: export ONNX,
quantise, run under ONNX Runtime Web / transformers.js — no Python in the shipped app and no new
runtime in `pixi.lock`. The eager-JS budget is unaffected (lazy chunk, like KaTeX/Mermaid). **The
sharp decision when it lands: bundling doubles the APK, 26 → 46 MB, and not bundling means the
feature needs the network once, against a stated offline-first principle.**

**Datasets** — UniMER-1M (1,061,791 pairs, HF `wanderkid/UniMER_Dataset`) for training;
UniMER-Test (23,757, split SPE 6,762 / CPE 5,921 / SCE 4,742 / HWE 6,332) for evaluation, where
**SCE and HWE are our real use case** and CPE is where the field separates (.678–.949).
im2latex-100k (103,556, Zenodo 56198) as a legacy baseline; MathWriting (230k human + 400k
synthetic, CC) is **digital ink, not images**, so strokes must be rendered.
**Open question: UniMER-1M's own licence is not stated separately from the repo's Apache-2.0, and it
determines whether a model trained on it can ship in an MIT app. Settle it first.**

**Evaluate with CDM, not BLEU** (arXiv 2409.03643). `(x+y)+z` vs `\left(x+y\right)+z` render
identically yet score BLEU 0.449 / ExpRate 0; a visibly wrong formula scored 0.907. Text metrics
reward matching the training set's LaTeX *style*, which no user cares about.

**Where it lands:** `crates/fm-core/src/ingest.rs`'s `extract_text` already dispatches on MIME
(`Some("application/pdf") => pdftotext(blob)?`). An image arm makes formulas searchable for free via
`Ingested.text`; inserting `$$…$$` at the cursor is a small change in `NotePanel.svelte`'s
`ingestAll`, and KaTeX already renders it.

---

### 2.5 Discussions as notes (built 2026-07-20, reviewed, **reverted before commit**)

`plan.md:279-287` and `collaboration-design.md:205` already specify it: **one message = one
file**, ULID-named, `thread:`/`reply_to:` frontmatter. Maildir, and "the atom is the file"
paying off. An adversarial review costed it, and the costs are the part worth not re-deriving:

- **`capture` cannot set a custom property** (`commands.rs:105`) — body and vault only. A reply
  through the existing path is `capture` + 2 × `set_property` = three whole-file rewrites and
  three index updates for one message. It wants its own command.
- **`Renderer::Thread` is impossible.** `Renderer` is the `.view` file's vocabulary and `base()`
  returns a *static* query (`views.rs:130`); a thread is parameterised by a runtime ULID. It has
  to be a new `dispatch` command, like `search`.
- **Contamination breaks a daily gesture.** A message is `Kind::Note` in the strongest sense.
  At 10k messages: Board puts all of them in `(none)`; Timeline/`recent` bury real notes below
  the fold forever; and **the `/` note-picker calls `recent()`** (`NotePanel.svelte:723`), so
  linking one note to another stops working. Agenda and Gallery survive.
- **Use the property filter, not a separate directory** — reversing an earlier note here. A
  directory sounded free because `Descriptor` carries `notes_dir`, but that is *one* directory:
  `FileStore` scans exactly one and `path_for(id) = notes.join("{id}.md")`, so a second means
  teaching `put`/`delete`/`path_for`/`reindex`/`verify`/`backup`/`written` which one a note is
  in. Meanwhile the filter is four call sites behind **one shared constructor** (`views::base`
  already had them behind a local closure), which is what the design doc specified all along.
  And the performance argument for a directory is void: `candidates()` pushes nothing but
  full-text down to SQL, so `Kind(Note)` is *already* evaluated in memory over the whole corpus
  — board and `recent` parse every file on every request today.
- **Must land with it, not after:** orphan/cycle handling (`reply_to` is unvalidated and hand-
  editable; `a → b → a` is a blank pane, a deleted parent silently drops a subtree), vault badges
  per message row (`decisions.md:481`), and `verify` reporting dangling `thread:` targets.
- **Conflict-freedom is half true.** File-level, yes. But `push_squashed` refuses outright when
  anyone else has replied since your last push (`git.rs:1078`), and a forum is *defined* by
  concurrent writes — so the retry path is the normal path and must be exercised by a test.

**What the 2026-07-20 build got wrong — fix these before rebuilding:**

1. **Do not call the property `thread`.** It is an unnamespaced English word, `set_property` is
   public, and `board` groups by *any* key — so grouping a board by `thread` and dragging one
   card writes `thread: <column>` and the note disappears from every view: one gesture, silent,
   no undo affordance, and `verify` will not even warn (its check only matched `Text`, and a
   dragged value may be anything). Prefer `thread_of`. Whatever the name, the exclusion should
   also require the value to *look like a ULID* before hiding a note.
2. **Spell id-valued properties `note:<ulid>`, or teach `refs` about them.** A bare ULID is
   invisible to `refs::strip_cross_vault`, which keys off `note:`/`asset:`/`sha256:` prefixes.
   So `copy_note` carried a `thread:` pointer into another vault — a cross-vault reference in
   permanent git history, a copy hidden from every view in its own vault, and a vault-B message
   showing up inside vault A's thread. The leak fix and the feature were each correct alone.
3. **`reply()` must re-root.** Replying to a *message* set `thread` to the message, so the reply
   was in no view and no thread — and `REPLY_TO` exists precisely to express that action, making
   the broken path the natural one. Re-root `thread` to the discussion root, set `reply_to`
   automatically, and reject non-`Note` targets.
4. **Compare ULIDs canonically, not as strings.** The argument normalises; a hand-written
   lowercase `thread:` value does not, and orphans the message.
5. **`activity()` needs the exclusion too** — it was missed. Every reply is a git touch, so a
   40-message thread floods the recent-edits feed: the same pollution `recent()` was fixed for.
6. **Budget the read.** `thread()` measured 170 ms at 30k notes — as much as listing the entire
   vault — because a `Prop` filter has no index behind it. Add a `perf.rs` case before a
   comments panel makes it hot.
7. **Ship it with its UI.** The 2026-07-20 build was backend-only, so `ipc.ts`/`mock.ts` were
   never updated (`docs/src/dev/adding-features.md` names four places; it touched two) and none
   of the UI-side claims — the vault badge, indentation from a flat list — were exercised.

**Then, and only then:** the **read half** of git review — branch list and diff, which are
"collaboration is git, *exposed*" (`decisions.md:460`). Merge and delete are corruption-path
writes needing their own decision and both backends (`git.rs` *and* `git_native.rs`, or the
phone regresses to the bug that seam exists to prevent).

**Not doing:** cloud LLM agents reviewing notes. Barred by `MASTERPLAN.md:63` (*no AI features
in v1*), and the premise is gone — GitHub Models retires 30 July 2026, we are already inside the
brownout window, and there is no free hosted successor. If any form returns it is the Lua hatch
(`MASTERPLAN.md:353`): user-run, local, off by default.

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
