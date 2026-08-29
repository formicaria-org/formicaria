# Outstanding — the work this codebase knows it owes

A ranked queue, so the next session picks up the most valuable thing rather than the most
recent one. Everything here was found by reading the code or by an audit, not hypothesised.

**This file is a queue, not a log.** When an entry is fixed, *delete it* and fold the durable
outcome into `known-issues.md` or `decisions.md` — the history lives in `sessions/` and in
git. The first version of this file kept its fixed entries struck through, and within a day it
had become a changelog with nothing to do in it. That is the failure mode to avoid.

_Last reconciled: 2026-07-21 — **§2.5 corrected**: the discussion/proposal backend + its UI were
rebuilt and committed (green under `cargo test --workspace`), so §2.5 no longer says "reverted"; only
the **proposal create/diff seam** remains (which is also the AI-agent plan's Phase-2 dependency). A
local study-assistant agent was researched and planned (`ai-agents-plan.md` Part III) — **plan-only,
gated behind MASTERPLAN's "core boring & stable".** Prior: 2026-07-20 (evening), after four defects were fixed, **re-reviewed
adversarially, and four of the fixes found incomplete and repaired**
(`sessions/2026-07-20-four-bugs-a-review-found.md`) — frontmatter leaking on copy (still leaking
via `status`/`tags`), no CSP (and none on the phone at all), `manifest.json` conflicting as text
(and its first merge rule was wrong for a gitignored, per-machine store), and a conflicted note
silently freezing all commits (fixed everywhere but the auto-commit that actually runs). A
threads backend was built and reverted in the same session; see §2.5. The Android CSP **loads clean on the real phone** (installed and launched, zero
violations), but an image-bearing note and a whiteboard are still unopened — see §1.1. Earlier
the same day: media working on Android end to end (`sessions/2026-07-20-capture-on-a-real-phone.md`);
photos are done and **video too (owner-confirmed 2026-07-20 evening)**, so 1.2 is no longer about
the transport at all — it is the storage question underneath: a phone-captured video currently has
one copy, in app-private storage an uninstall erases. Prior: 2026-07-18, after the whole compiled queue was worked through
(`sessions/2026-07-18-vault-as-a-repo-and-the-queue.md`)._

---

## 1. The one that gates everything else being trustworthy

### 1.0 One person, several name spellings, in existing history
The **identity** half is done (2026-07-31, `decisions.md#ui` — "A contributor is an email,
everywhere"): the chips and the filter now key on the email, so one human is one contributor
whatever the commits are signed, and all three vaults sign `singhbal-baljinder` going forward.

What remains is **display of the history already written**: the owner's vault holds 534 commits as
`singhbal-baljinder` and 77 as `Baljinder`, one email, and the Activity pane shows each commit's raw
`%an`. Deferred by the owner ("keep it in future features"), so it is a queue item and not a gap.

**Done looks like:** a `.mailmap` at the vault root maps every spelling of one email to one name, and
**both git backends honour it** — `git log --use-mailmap` on the subprocess side (`log.mailmap`
defaults to true since git 2.26, but relying on a default is how the two backends drift), and
`Repository::mailmap()` + `Commit::author_with_mailmap()` on the libgit2 side (both exist in git2
0.21 — checked). Plus a parity test in `crates/fm-cli/tests/`, because this is exactly the shape that
bit twice on 2026-07-31: **a behaviour present on the laptop and absent on the phone.** The file must
also be committed to travel, which means adding `.mailmap` beside `.gitattributes`/`.gitignore` in
both `commit_all`s. **No history rewrite** — `.mailmap` changes display only.


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

**◐ Narrowed again 2026-08-20**, and the reason is worth stating precisely, because it changes what
"seen by eye" is still buying. The suite can now *drive* the phone's own branches — `asPhone()` in
`ui/src/lib/harness.ts` installs a fake `__TAURI_INTERNALS__` so `ipc.ts` takes the Tauri transport,
and `test-setup.ts` supplies a settable `matchMedia` so `(pointer: coarse)` branches execute. So
"the touch shell is untested" is no longer true of the *logic*.
What it does **not** buy, and what still needs the device:
- **Layout, reach and legibility.** jsdom applies no CSS at all. Thumb reach on the 2.75rem targets,
  the single-column reflow, and anything behind a `@media (pointer: coarse)` block are unchanged by
  any of this.
- **The synchronous JNI bridge**, which is the mechanism behind the freezes that prompted the
  2026-08-20 work. Nothing host-side reproduces it; the `(async)` fix is verified by a `checks.sh`
  grep and by compiling for `aarch64-linux-android`, **not by running**.
- **The CSP claims above**, unchanged — still a real note with a real image, and a whiteboard.
**Done now looks like:** install the next APK, open a big note and type a paragraph, add a photo,
leave the app and come back. Those are the three reported symptoms and none is provable from CI.

### 1.3 Chunked ingest — the real fix for the photo path
`MAX_INGEST` dropped 48 → 16 MB on 2026-08-20 (`decisions.md#track-m`), which keeps every phone
photo and drops the transient peak from ~600 MB to ~200 MB. It is an interim: a file still crosses
the bridge as **one JSON string**, copied roughly ten times between the page and Rust, so the
ceiling is a memory limit wearing a size limit's clothes and **video is still refused**.

**Done looks like:** `fm_ingest_chunk(session, seq, data)` + `fm_ingest_finish(session, name, vault)`
over `BlobStore::put_file`, which already streams and hashes in 64 KB chunks; `commands::asset_note`
is already factored out for exactly this second byte-arrival path. Bounds the transient at one chunk
regardless of file size, and lifts the video refusal. Content addressing is safe — the hash is taken
once over the assembled file, chunks being pure transport. **The one new hazard** is an orphan
session after a `SIGKILL` mid-upload; sweep the session directory at boot.

Deliberately *not* done alongside the 2026-08-20 fixes: it is a transport design change, and
smuggling one in beside a bug fix is how a transport ends up with two shapes nobody chose.

### 1.2 Where a phone's media actually survives
**Video attaches and plays on a real phone (owner-confirmed, 2026-07-20)** — this entry used to
say it did not, which was overstated. Photos and video both go through the same base64 `fm_ingest`
IPC path, the one binary transport Android leaves open, and `MAX_INGEST` caps an attachment at
48 MB; over that it is **refused with an explanation** rather than crashing. So the transport is
not the open question. Two are:

1. **Where the bytes live — the one that matters.** A phone vault is app-private storage, wiped
   on uninstall; `blobs/` is gitignored so a push does not carry it; restic is a binary Android
   does not have. Today a video attached on a phone has **exactly one copy, in the place a single
   uninstall erases**. That is true right now, for media that already works — it is not gated on
   anything below.
2. **Chunked ingest**, only if the 48 MB cap turns out to bite: send the file in bounded slices
   and append, so peak memory is one chunk rather than 1.33× the file. **Do not do 2 before 1** —
   lifting the ceiling without an answer to 1 invites people to trust it with more.

**Done looks like:** an honest, stated answer to where a phone-captured video survives — and the
app saying it, rather than the user discovering it.

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

### 2.5 Proposals & discussions — only the create/diff seam is left (backend + UI landed, green 2026-07-21)

**Discussions-as-notes and proposals-as-notes shipped and pass `cargo test --workspace`.** The
corrected rebuild is committed: `crate::thread` holds the *one* definition of the hidden note-classes
(`thread_of`/`reply_to`/`proposes`, each parse-guarded so a board drop can't erase a note);
`commands::{reply,thread,create_discussion,discussions,proposals}` are routed in `dispatch.rs`;
`activity()` excludes messages **and** proposals (line ~905); and it shipped **with** its UI
(`ipc.ts`/`mock.ts` + `thread.svelte.test.ts`), covered by `tests/{thread,discussions,proposals,
stale}.rs`. All seven findings that sank the first attempt — don't-call-it-`thread` (→ `thread_of`),
`note:<ulid>` spelling, re-root on reply, canonical ULID compare, the `activity()` exclusion,
ship-with-UI — are addressed.

**What actually remains — the create/diff seam** (and this *is* the agent plan's Phase-2 dependency,
`ai-agents-plan.md` §17):
- **Create a proposal in-app.** Today a proposal is created **out of band** — a note carrying
  `proposes: branch:<name>` dropped in the vault by hand (or by an agent). There is no
  `create_proposal`/`propose_branch` command that makes the branch + note from a note or whiteboard.
  `docs/src/user/collaboration.md` already advertises this as "being built next".
- **The read half of git review** — branch list + diff for a proposal ("collaboration is git,
  *exposed*", `decisions.md:460`).
- **Merge/delete** — corruption-path writes; their own decision and **both** backends (`git.rs` *and*
  `git_native.rs`, or the phone regresses to the bug that seam exists to prevent).
- **Minor:** a `perf.rs` budget for `thread()` (measured ~170 ms at 30k notes) — the one 7-item finding
  not yet covered by a test; add it before a comments panel makes it hot.

**Not doing (v1):** cloud LLM agents reviewing notes — `MASTERPLAN.md:63`. A *local, contained*
study-assistant agent is planned but **gated** (`ai-agents-plan.md` Part III); its `propose_branch`
lands on exactly the create-seam above, so building that seam serves both.

---

### 2.6 The welcome screen — the last piece of the friendly release
**Closed on 2026-08-28:** the in-app manual route this entry used to ask for is done — the book is
baked into the binary, `/manual/` serves it under its own CSP, and a Help button sits beside the
gear (`decisions.md#toolchain`). What is left is the other half of that session's decision.

The archive now ships a **ready empty vault**, so the app opens straight into somewhere real and
the `vaults.length === 0` gate in `App.svelte` never fires. The owner's ruling was that a **short
welcome screen** should come first: name, email, and optionally a remote — then into the app.

**Landed already (the backend half):** `set_identity` is its own `dispatch` command, because
`set_git_remote` sets an identity only alongside a URL and refuses an empty one — so "just my name,
no remote" wrote the identity to disk *and reported failure*. It is in `REMOTE_DENIED`: a paired
tablet does not get to name the host's committer. `ipc.ts` exposes `setIdentity`.

**Still to build:** `ui/src/lib/Welcome.svelte` and its gate. Three things decide whether it is
right:
- **The trigger must be cheap.** Gate on identity, read from `list_vaults` (add `identity` to
  `VaultInfo`, beside the `remote_label` call that already spawns git locally) — **never**
  `backup_status`, which shells out `git ls-remote` per vault. The 2026-07-17 ruling already
  refused to make a first-run screen wait on the slowest git command.
- **It must be skippable, and skipping must cost nothing.** The notebook needs no git at all;
  `ensure_repo` commits under a placeholder identity, so a user who skips keeps full history and is
  asked again by `BackupPanel` at the moment a remote makes a name matter. Hide it entirely when
  git is absent, or a git-less user is trapped on a form that can never save.
- **Save identity first, and separately.** A bad remote URL must lose the remote and keep the name.

### 2.6b Features a user asks for are still not *delivered* — only declared
**Closed 2026-08-28** (`decisions.md#agent`): nothing now claims a capability it lacks, and saved
views are writable from the app. What the owner asked for goes further — *"a user picks features and
never thinks about dependencies, and asking for a feature must deliver it working, like any app"* —
and that half is deliberately not built yet.

**The installer.** Fetch what a feature needs into `<app>/program/tools/` — portable, no admin
rights, travels when the folder is copied. Every piece exists and none is wired together:
- `crates/fm-agent-run/src/fetch.rs` — resumable, SHA-256 verified, atomic, progress callback,
  hermetic tests already in `pixi run ci`, **desktop-excluded by one Cargo feature**.
- `pixi.lock` already pins poppler, libvips and restic for all four platforms with checksums, so
  installing means fetching *the exact artifact pixi would*.
- `packaging/formicaria.sh` records the proof they are relocatable: run "with PATH alone, resolving
  every shared library (conda binaries carry their own RPATH)".
- The lookup is the missing link: three `available()` functions all use bare `Command::new`, with no
  indirection at all. `fm_core::git`'s `merge_command()` already argues the fix — *never a bare name
  hoping PATH will answer* — for our own `fm` binary; nothing applies it to a third-party tool.

**Delivering a feature is usually three steps, not one.** Encrypted backup needs restic **plus** a
repo location (hand-edited `vaults.json`; Settings says verbatim *"there is no UI for it"*) **plus**
`RESTIC_PASSWORD` (an env var whose documented answer is *"a launcher you have edited yourself"*).
Note `vaults::save` is append-only by design and never rewrites an entry — that constraint has to be
met, not worked around.

**The assistant, delivered.** Fetch the model in-app with progress as Android already does on first
enable, ship the manifest in the archive, start the runtime. **The blocker is cleared** (2026-08-29,
`decisions.md#agent`): `models.toml` now pins every model to a Hugging Face **commit + SHA-256**, and
`fetch.rs` gained the read timeout, the identity encoding, the completeness check, the fatal/transient
split and the cancel flag it needed before being armed. What remains is the *desktop* wiring — the
Cargo feature is still off there, and the runtime is still not in the archive.

**Smaller, same theme:** the backup panel has no token field (one exists only in the clone flow), so
a user who created a vault locally and later adds an HTTPS remote has nowhere to put a token. And the
"save this view" naming step is a `window.prompt()` — genuinely in-app, but not the app-quality
affordance the owner asked for.

**A thumbnail consumer.** Generation works; nothing requests it (`render.ts`: *"There is no thumbnail
path on any platform"*). Until one exists, previews cannot be offered as a feature at all.

### 2.7 The manual reads for a beginner now — it still has no pictures
**Closed 2026-08-28** (`decisions.md#toolchain`): setup is per-OS and assembled by `ci/docs.sh`, so
each archive carries instructions for its own platform only; `Start here` is two short chapters and
ends by telling the reader they are done; `Going further`/`Reference`/`Developer guide` hold the
depth; and the introduction routes beginners at beginners.

What the 2026-08-28 audit found and this did **not** fix:

- **Zero screenshots** in a manual for a GUI. Now the largest remaining gap by some distance, and
  the slowest to close. `docs/context/shots/` proves the capture path exists.
- **`user/notes.md` is 2,245 words unsplit** — it is now the first chapter under *Going further*,
  so it is the first wall a curious beginner hits.
- **No glossary.** *frontmatter*, *ULID*, *content-addressed*, *blob*, *merge driver*, *remote* and
  *FTS* all appear in user chapters undefined.
- **`user/assistant.md` is 100% checkout/pixi/Android-SDK** — unusable from a release, and nothing
  on the page says so.
- **`user/views.md` documents saved views only as hand-written YAML**, plus a 9-row filter DSL.
- **The sidebar still shows every part at once.** `fold` in `book.toml` is a no-op because no
  chapter has children, so a beginner still sees `Architecture` and `Design system` in the same
  column as their own two chapters — further down now, but present.

### 2.8 Windows gets libgit2 — one cost accepted and deferred
**Taken 2026-08-28** (`decisions.md#git`): the libgit2 exception is now scoped to *any device with
no git binary*, so `fm-serve` pulls it through a `[target.'cfg(windows)'.dependencies]` entry.
Windows users get history, backup and collaboration without installing anything; machines that have
git are unchanged, because `vcs.rs` chooses at runtime.

**The deferred cost:** `crates/fm-core/Cargo.toml` pins `git2` to `vendored-openssl` for Android's
sake — Android has no system OpenSSL — and that feature is **not split per target**, so the Windows
build compiles OpenSSL it does not need. libgit2 on Windows can use the OS's own TLS. Splitting it
means separate `[target.…]` blocks for `git2` **and** for `openssl-sys` (which `native-git` also
pulls, purely for Android's in-memory trust store), so the Android path must not break.
**Done looks like** a smaller, faster Windows build with the Android trust-store path untouched and
`pixi run android-check` still green.

**Unverified, and it is the thing most likely to bite:** no Windows build of this has been run.
`cargo tree` resolves correctly per target from here, but compiling vendored OpenSSL on the Windows
runner needs Perl and NASM on the image. If the release's Windows job fails, this is the first
suspect.

### 2.9 A backup with no git reports success and records nothing
Separately, and worse than the report above: on the build that ships today, with no git binary,
`commit` returns `{"committed":true}` and creates **no repository at all**. `ping.git` likewise
reports `true` when git is provably absent (`env` cannot find it). A user can be told their notes
are backed up when nothing was recorded — the failure class this repo names repeatedly, *a surface
that will not say what it knows*. Diagnosed but not root-caused; independent of whether libgit2 ever
ships on the desktop.

### 2.8 The Windows and macOS launchers have never been executed
`packaging/launcher/formicaria.sh` is verified end-to-end: unpacked from a real archive, launched
from an unrelated working directory, from a path containing a space, and the note landed beside the
app with `~/.config/formicaria/vaults.json` untouched. `Formicaria.command` is byte-identical below
its header, so its *logic* is covered — its Finder behaviour is not. `formicaria.vbs` and
`formicaria.bat` have been executed by nobody.

**Known risks, unmeasured:** Windows Script Host is disabled by policy in many managed
environments, and `.vbs` launchers are a malware idiom that AV heuristics flag — on an unsigned
binary that is two strikes. `formicaria.bat` ships beside it as the visible-console fallback for
exactly that reason. **Done looks like** one run of each on a real machine of each kind.

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
