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

### 2.0 The papers tool — the reader is the part that is still missing
Full direction, with the four adversarial reviews behind it: [`papers-plan.md`](./papers-plan.md).
How it got here: [`sessions/2026-08-29-papers-and-what-three-audits-found.md`](./sessions/2026-08-29-papers-and-what-three-audits-found.md).
**Owner's ruling: it is a separate app**, not a feature inside the notebook.

**Shipped (2026-08-29):** the library. A paper is a `Kind::Note` tagged `paper` with flat metadata;
`create_paper` builds one from a pasted BibTeX entry, a DOI, an arXiv link or a title, all offline
(the core links no HTTP client — `fm-agent-run`'s ruling); the identifier a PDF prints on itself is
read at ingest; `paper_bibtex` copies a citation back out; a `Papers` view is one saved tag filter.

**Not started, and this is the half the owner actually asked for** — highlight a word, a sentence, a
figure, a formula, and have the note be anchored to that place:

- **The reader.** pdf.js with `isEvalSupported: false` (the CSP has no `'unsafe-eval'`, by a
  deliberate ruling) **plus** its `cmaps/` and `standard_fonts/` assets, or CJK and
  non-embedded-font papers render blank. Desktop only — say so; Android cannot render a PDF inline
  at all (`decisions.md#track-m`).
- **The anchor format.** `#page=N` ships and is the human/degradation half. The machine half does
  not exist: a highlight is `rects` — **plural**, and one crossing a line break has several, which
  in a two-column paper is the common case; ink is `paths`; an EPUB position is a CFI. None of that
  belongs in a URL fragment. It needs a structured block in an annotation note, versioned and
  explicit about its coordinate space, **designed before a line of reader code**.
- **Annotations as notes from the start.** A highlight-as-prose-block can never hold multi-rect,
  ink, colour, per-annotation tags or `sortIndex`. They must also be excluded from the planning
  views via `thread::notes_base`, or every board and every search fills with fragments — the volume
  disease the plan diagnoses in every competitor.
- **The Zotero port**, which the owner made a hard requirement. Local API only: **every export
  route drops annotations** (`case 'annotation': return false`, upstream of every translator), and
  area-annotation images often do not exist on disk until the PDF has been opened in Zotero's own
  reader — so the importer must render crops itself and therefore *depends on* the reader. PDF-only:
  EPUB and snapshot annotations have no landing site.
  **Two of its three prerequisites landed on 2026-09-02** with the Logseq/Obsidian importer
  (`decisions.md`, *An import converts; adoption renders*): `source_key`/`source_library` are
  reserved and in use, and an import path that sets `created`/`updated` exists — it builds the
  `Object` whole and `put`s once, so it never meets `apply_property`'s refusal. Still owed: the
  atomic `import_item` **command** itself (title + body + tags-as-list + properties + asset hashes
  in one call), which Zotero needs and a folder-walking importer did not.

**Three obligations the owner's *separate app* ruling incurs, none started, all cheaper now than
later** (`papers-plan.md` Part 5): a dated reversal of `MASTERPLAN.md:341` with a `> SUPERSEDED`
banner — **not** a scoping entry; widening `ci/checks.sh`'s HTML-sink and literal-free-renderer
greps to the second app **in the same commit that creates it**; and an npm licence gate, because
`deny.toml` and `third-party.sh` are keyed on `Cargo.lock` and will never see pdf.js. There is also
no bundle-size gate at all today, and the eager JS payload is ~70 KB gz.

### 2.0b Extracted PDF text still lives in the note body
`asset_note` puts `pdftotext` output in the body, measured at 1.7–2.2 KB per page — so a few
thousand papers is >100 MB of note body, in git, in `objects.content`, and in the FTS shadow table.
The `Kind` pushdown (2026-08-29) stops the *planning views* hydrating it, which was the urgent half.
Moving it to `derived/<hash>/text.txt` would fix git size, launch time, and search returning
`paper.pdf` instead of the paper — **but `Predicate::Text` is a substring scan over the body in
`MemoryStore`, so text only `FileStore` can see breaks the store-equivalence invariant
`decisions.md#seams` protects.** That is why it was not done inside a feature commit; it needs its
own design and its own entry.


### 2.0c Supervision signal — the datasheet is written; nothing is captured yet
The corpus this app could produce — *the AI proposed X, the human made it Y* — is currently
**destroyed on the two paths that matter**, and a datasheet now exists for the corpus that does not
exist yet: [`supervision-datasheet.md`](./supervision-datasheet.md). Read it before adding any
capture; it is written as a contract, including what the corpus may **not** be used for.

**Done 2026-08-30:** ten model-authored `propose:`/`revise:` commits were found **unreachable and 37
days old** against a 14-day prune window, and rescued to `refs/fm/rescue/<sha>` in the owner's vault.
Verifying the rescue also confirmed the design's central invariant on real data: a ref outside
`refs/heads/*` is seen by **neither** `git log HEAD` (so not by `activity` or `newest_foreign`) **nor**
`proposal_load` — every history walk in this repo uses `push_head()` alone.

**What git keeps today, and it is the wrong half:** *accept* is safe (the proposal commit survives as
the merge's second parent, and `accept:` is a squash barrier), but **revise force-orphans the previous
revision** (`write_proposal_branch` commits `-p HEAD` then force-moves the ref, and `commands.rs:777`
force-pushes) and **reject deletes the branch outright**. The human's correction — the most valuable
signal — is the one that does not survive.

**Proved 2026-08-30 with a prototype exporter over the real vault** (23 records: 13 accepted, 9
orphaned, 1 no-op): the corpus git already holds is not merely incomplete, **it is mislabelled**.
All 13 accepts report the model's text landing *verbatim* — but that is an artifact: a human edit
becomes a new proposal commit which becomes the merge's second parent, so a clean merge is always
verbatim relative to the final proposal. **5 of those 13 notes also have an orphaned earlier
proposal**, so up to ~38% of the accepted class claims "right first time" when it was the last of
several attempts. Two exporter rules fell out and are not optional: diff each proposal against
**its own merge-base with `main`** (otherwise unrelated notes ride along), and label a proposal
whose tree equals its base as **`no-op`** rather than emitting it as a pair.

**Also done 2026-08-30 — a live data-loss bug, fixed:** `ProposalReview.svelte`'s Accept sent only
the proposal id, never the draft, so **accepting with unsaved edits merged the model's original and
threw the reviewer's correction away silently.** Accept now persists the draft first and refuses to
merge if that fails (`persistDraft`), and Save is disabled mid-accept. Pinned by a regression test
that was *checked to fail without the fix* (`expected 'the model text' to be 'my correction'`).

**Also done 2026-08-30 — retention refs, both backends:** `retain_proposal_tip` points
`refs/fm/review/<proposal-id>/<sha>` at a proposal's outgoing commit before a revise force-moves the
ref or a reject deletes it (`decisions.md`, *A proposal's outgoing commit is kept*). Pinned by
`a_revised_or_rejected_proposal_keeps_its_outgoing_commit_on_both` in `git_differential.rs`, checked
to fail without it. **`pixi run test-native-git` is the gate** — the route-parity grep cannot see
this, because the helper is internal to each backend.

**Also done 2026-08-30 — the reviewer's reason is captured (item 4):** `create_proposal` and
`reject_proposal` take an optional `why`. On a proposal it becomes the **commit body** (never the
subject — the prefix is the squash barrier); on a reject it lands as `declined_why` on the proposal
note, which is the only place it can survive since reject writes no commit and the rejected text
never reaches `main`. `commands::review_note` sanitises it: collapsed to one line, so a pasted `---`
cannot terminate the message and a `Token: value` tail cannot masquerade as a trailer; an
all-punctuation or empty answer is a **skip**, not a failure. The field is optional and
always-visible rather than a confirm step — a required prompt produces satisficing, and a dismissable
one would tax every typo fix. `ci/checks.sh` now greps labels and placeholders for git vocabulary
(deliberately not button text: four such strings pre-date this and are a separate call).

**Partly done 2026-08-30 — the machine trailers (item 5):** a proposal's commit message is now
`subject` / reviewer's reason / **git trailers as the final paragraph** — `SchemaRev: 1` always, and
`Assisted-by: formicaria-agent:<model>` when an agent proposed (never `Co-authored-by:`, which the
kernel, ASF, LLVM and OpenTelemetry all rejected on the grounds that a model cannot hold
accountability). `SchemaRev` exists because aider's convention changed seven times without recording
its own version, leaving its history machine-uninterpretable. Trailers go last because that is the
only paragraph git parses as trailers — which is also what keeps a reason shaped like `Foo: bar`
prose rather than a field, asserted through git's own parser.

**Item 5 finished 2026-08-30 — the input half:** `fmserve::Origin { tool, query, sources }` travels
from the three agent call sites (`propose` / `research` / `transcribe`) through `dispatch` into
`commands::Record`, and lands as `Tool:` / `Query:` / `Sources:` trailers. `/research` carries the
URLs it cited — the only durable record, since it cites a live web that will not exist at training
time — and `/transcribe` carries the `asset:` references of the audio.

**Two gaps remain, stated rather than papered over.** The **fully rendered prompt** is not captured:
it is multi-KB with newlines and a trailer is single-line by definition, so it needs a home that is
not a commit message. And the model's **revision** is absent — `models.toml` pins every model to a
commit + SHA-256, but only the identity reaches this layer, so a pair cannot be proven on-policy for
the model it would train (plan T6).

**Item 5b done 2026-08-30 — the unlabelled pool, and live defect #2 with it:** `vcs::retire_proposal`
(both backends, routed, differential-tested) releases a settled proposal's guardrail slot while
**keeping its commit** under `refs/fm/review/`. It is local-only by design — retiring is bookkeeping,
not a statement to collaborators, and `delete_branch`'s remote delete is a network round trip that
must not happen in a loop from a write path. `commands::create_proposal` sweeps declined-but-still-
branched proposals at the moment the ceiling is checked, which is where the failure actually bit: a
**peer's** reject leaves our `refs/heads/proposal/<id>` behind (fetch never prunes), so
`proposal_load` counted it forever and a UI-only user eventually met *"the vault would have 2 open
proposals, but it allows at most 1"* with no way out. Pinned by a test checked to fail without the
sweep. The retired proposals become the **unlabelled pool** — the precondition for every selection
method that works at this corpus size, since a log of only-corrections has nothing to select from.

**Item 9 half-done 2026-08-30 — consent, the part that cannot be retrofitted:**
`Descriptor::supervision` (`vault.json`) holds two separate answers — `collect` (default **on**: the
user's own history in their own repo, nothing leaves) and `publish` (default **off**, never
inferred, because publication cannot be recalled). It lives in the vault's file rather than
per-device Settings for exactly `git_assets_max`'s reason: publishing relicenses **shared** content,
so the vault decides, not the loosest machine holding a clone. Each proposal is stamped
`Consent: local` / `Consent: local,publish` **per record** — a flag flipped next year must not
silently relicense what came before — and `collect: false` writes no record at all, not a flag
saying so.

**Item 9 finished 2026-08-30 — the UI half:** two switches per vault in Settings, beside the
attachment rule and for the same reason (`vault.json` decides what may leave, not whichever device
is loosest). `set_supervision` is a `dispatch` command, **host-bound** — added to `REMOTE_DENIED`
*and* to the paired-device test, which uses a hardcoded list rather than the constant, so a new
entry is not covered by merely existing. A guest device does not get to grant publishing rights over
the host's vault.

Whoever builds the external export tool: it **reads** this and refuses records lacking it. It never
grants it — a tool that could would be a route around the gate, which is the failure that stranded
the only comparable open corpus.

**Part of item 7 done 2026-08-30 — `Kind:`, the axis the corpus is split on:** three chips
(`style` / `factual` / `reasoning`), single-select and optional, threaded through `Record` to a
`Kind:` trailer and **validated against a closed set** — an unknown value is dropped, never stored,
because a free-text axis is one nobody can group by and grouping is the entire point. It earns its
UI because alignment and formatting saturate after roughly a thousand examples while factual and
reasoning corrections keep improving with data: unlabelled, the two are indistinguishable and the
corpus is only as useful as its weakest part.

**Deliberately not done in item 7, with reasons:** `Strength` (an ordinal severity) — second-order
next to `Kind`, and the review panel's UI budget is real; `IFD` — needs a model on the write path;
`simhash` and the PII scan — worth doing, no blocker, just not yet.

**Half of item 6 done 2026-08-30 — `shown`:** `thread::SHOWN` stamps an RFC3339 timestamp on the
proposal note the first time the review panel actually renders the proposed text, server-side,
**once**. It is the only thing separating *the reviewer read this and left it alone* from *nobody
ever opened it* — two absences that are identical in the data and opposite facts about the model.
A timestamp rather than a bool because it costs the same and the gap to the decision is how long
someone actually spent. It lives on the note, not in a trailer, because it is a fact that arrives
**after** the commit is written and a commit message is immutable.

**The survival re-check — the other half of item 6 — is deliberately not built**, and not for want of
time. It needs **move- and reformat-tolerant span identity**: in a Markdown note, re-wrapping a
paragraph or moving a section is not a deletion, but a line-based differ scores it as one, and the
one published pipeline that got this right prevented 8.3M false "deaths" across 32.5M line-birth
events. Its censoring is also *informative*, not merely interval — someone who closes a note right
after accepting a bad edit is censored **because** it was bad, which biases naive retention upward.
That is an algorithm and a statistical correction, not a flag; it deserves its own design.

**Item 5c will not be built, and the reason is structural, not effort.** A suppressed-suggestion
holdout — generate 1–5% of outputs and deliberately not show them, keeping what the person wrote
unaided — assumes the AI **offers** things unprompted. This app has none: `watch.rs:123` gates every
turn on `convo::addressed`, so the agent speaks only when `@name`-mentioned. Suppressing an answer
someone explicitly asked for is not a holdout, it is a broken reply. The unbiased eval set and the
anchoring baseline it was meant to buy still need solving; they need a different mechanism here, and
inventing one is its own design question rather than a line of code.

**Composition is now asserted, not assumed (2026-08-30).** Each field was tested alone; a test now
pins that they add up to *one* well-formed message — three paragraphs (what / why / machine fields),
the trailer block last, every key single-line, and git's own parser agreeing. The failure it exists
to catch is the compositional one: two correct features producing a message where the trailers are
no longer the final paragraph, at which point git silently stops parsing them while every unit test
still passes. The key order is fixed and **`SchemaRev` versions it** — changing the shape means
bumping that.

**Item 8 started 2026-08-30 — the pure core of the preference memory:**
`crates/fm-agent/src/preference.rs` holds `Preference`, cosine similarity, `Memory::nearest`
(k = 5, filtered per tool) and `Memory::prompt_block`, plus an `Embed` seam that `OpenAiStep` now
implements against `llama-server`'s `/v1/embeddings`. Pure and model-free, like every other seam in
that crate.

**The load-bearing detail, encoded and tested:** the block emits the inferred **descriptions only,
never the examples** — feeding back retrieved edits measured *worse than no learning at all*
(32,405 vs 31,103 cumulative edit cost, arXiv:2404.15269). It also deduplicates, because one
preference retrieved from five contexts is one preference and repeating it five times is how a small
model starts obeying it instead of the request; and it returns `None` rather than an empty heading,
which a small model will parrot.

**Not yet wired, and the missing link is architectural:** the corpus lives in git, and `fm-agent`
deliberately depends on no store and no git — that purity is what lets it target Android. So
loading preferences belongs in `fm-agent-run` (over the HTTP surface) or `fm-app`. Still missing:
inferring a description from a correction (an ordinary `LlmStep` call), reading the corpus, and
injecting the block into `assemble_prompt`.

**Verified end to end 2026-08-30 — `crates/fm-app/tests/supervision_roundtrip.rs`.** Five tests that
write through the **real command door** (`fm_app::dispatch`, exactly as `fm-serve` frames it) and
read back with the **`git` binary alone**, never through the app — because the consumer is a separate
program that will not link `fm-app`, and asking an implementation to grade itself proves nothing:

- a whole review event round-trips: model text, human correction, reason, labels, provenance;
- a **rejected** proposal keeps both its text and the reason it was refused;
- `git gc --prune=now` takes **nothing** — the durability claim, which is exactly what Step 0 had to
  rescue by hand because nothing referenced those commits;
- **a push does not collapse the record**: `push_squashed` rewrites local history with `reset
  --soft`, and this proves the accept survives as a squash barrier and the retention refs are
  untouched;
- **a complete dataset row is derivable from git alone** — the executable contract for the external
  exporter. If that test changes, the tool changes, and `SchemaRev` is what says so.

**It found a real bug, which is what it was for.** `reject_proposal` and `proposal_shown` wrote
`declined` / `declined_why` / `shown` to the working tree and **never committed them** — so an
outside tool reading history saw none of it, and the record depended on the debounced auto-commit,
a browser `setTimeout` that dies with the tab (and with `FM_AUTO_SHUTDOWN`, closing the tab *is* how
the app is quit). A rejection's reason is the only record of *why*, since the rejected text never
reaches the branch. Both arms now commit their own record, as `create_proposal` already did.

**Verified against the real model, end to end, 2026-08-30.** Not simulated: the projector was
fetched, `agent-serve` restarted with `--mmproj` (3562/4096 MiB on the RTX 3050), five typeset
fixtures attached to a note in a scratch vault, and `@qwen3-vl-4b /transcribe` run through the
running app. Then the human half — corrected, said why, accepted; a second accepted unchanged; a
third rejected with a reason. **Rebuilding the dataset from `git` alone gave 3 rows across all three
outcomes, 13/13 field checks passing**, including the pair that matters: the model's `LiFeP04` and
the human's `LiFePO4` both recoverable, and the rejected plot transcription still readable although
it never reached the branch. Fixtures and the measured results: `agents/bench/vision/`.

**Two extraction lessons for whoever writes the external tool** — both found by running it:
- **Strip the trailer block before reading `%b` as the reason.** With no reason given, `%b` *starts*
  with the machine fields, so a naive read reports the record's own trailers as the human's words.
- **A row is a join, not a read.** The input half (`Tool`, `Query`, `Sources`, `Assisted-by`) rides
  on the **agent's** commit; the reason and `Kind` on the **human's** revision. Reading trailers from
  one end only silently loses half the record.

**Done looks like**, in order: a **retention ref namespace** (`refs/fm/review/…`) written *before* the
existing code moves or deletes the branch, on **both** git backends; an exporter built first, against
the ten rescued examples, so the corpus has a consumer before it has volume; and a *"what did you
change?"* prompt whose answer becomes the commit body — **with no git vocabulary anywhere in the UI**,
per the owner's ruling.

**Three live defects were found while designing this and are not part of it** — worth their own
commits: `accept_proposal` never checks `declined` while `pull` fetches without `--prune`, so
accepting a peer's *rejected* proposal **merges the rejected content into `main`**; `proposal_load`
leaks one of 25 guardrail slots per peer-rejected proposal, with no way for a UI-only user to clear
it; and the two backends order the squash walk differently (`git.rs` by date, `git_native.rs`
topologically) with no test over a merge.

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

**The Logseq/Obsidian importer (2026-09-02) did *not* do this**, and the distinction is the whole
point of both: an import *converts* files you are leaving behind and writes fresh `<ULID>.md` into a
vault; V4 *renders* files you keep owning elsewhere, in place, with a transient id and nothing
written to your repo. The importer does reuse V4's one durable insight — "use git for what only git
knows" — to date an imported note from the source repo's history rather than from the day it ran.
The recursive walk it needed lives in `import.rs` and is **not** the one V4 wants; V4's has to be
inside `FileStore::reindex`, with the path recorded in the index, which is still unbuilt.

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
- ~~**Minor:** a `perf.rs` budget for `thread()`~~ — **done 2026-08-31**, and the comments panel it
  was meant to precede landed with it (`crates/fm-app/tests/perf.rs`). Measured: 56.7 µs/note at
  2k, 51.5 µs/note at 8k — **linear in the corpus, growth 0.91x**. The budget asserts that shape
  rather than a duration, because the honest finding is that `thread()` is corpus-linear *by
  design* (a bare `NoteRef` with no `Kind` conjunct cannot push down), which is precisely why the
  feed reads counts from `thread_roots` and calls `thread()` only when a reader opens one.

**Not doing (v1):** cloud LLM agents reviewing notes — `MASTERPLAN.md:63`. A *local, contained*
study-assistant agent is planned but **gated** (`ai-agents-plan.md` Part III); its `propose_branch`
lands on exactly the create-seam above, so building that seam serves both.

---

### 2.5b Aggregating notes by tag — the next real want, not started
**Named by the owner 2026-08-30**, while looking at the finished side panel: *"I am not sure people
will want all that level of personalization at this stage. What for sure they will want is to
aggregate and visualize notes based on other tags or options, that's for later."*

Two instructions in one sentence. **Stop deepening view personalization** — the panel, the rail and
the saved-view surface are enough for now, and the appetite for more is unproven. And the thing that
*is* wanted is **gathering notes by tag** (and by other properties) and seeing them that way.

What already exists to build on, so this does not start from nothing: the `.view` filter grammar
already expresses `tag`, `tags_any`, `tags_all`, `prop`, `date` and `not` (`crates/fm-app/src/views.rs`);
`save_view` can already write exactly one bare tag from the UI and **refuses rather than flattens**
anything richer; and `describe_pred` already turns a filter into plain English for the "filtered:"
chip. What is missing is a way to *ask* for a grouping without writing YAML — and the standing
ruling is that a UI over the nine-predicate grammar is a query builder, rejected twice. So the design
question this has to answer is: what is the small, sentence-shaped question a person actually asks
("show me everything tagged X, grouped by Y"), and can that be offered without becoming the builder?

Not started. Do not treat the rejected query builder as the only shape this could take.

### 2.6 The welcome screen — done
**Closed on 2026-08-28:** the in-app manual route this entry used to ask for is done — the book is
baked into the binary, `/manual/` serves it under its own CSP, and a Help button sits beside the
gear (`decisions.md#toolchain`). What is left is the other half of that session's decision.

The archive now ships a **ready vault with one note in it** (`packaging/welcome/`, staged by
`release.yml` and dated with the build), so the app opens straight into somewhere real and the
`vaults.length === 0` gate in `App.svelte` never fires. The owner's ruling was that a **short
welcome screen** should come first: name, email, and optionally a remote — then into the app.

**The note is the other half, added 2026-08-30.** The screen answers the question git forces; the
note answers *what is this and where do I click*, on the Timeline the screen dismisses into. It is
~360 words and says it can be deleted. Its risk is not that it is wrong today but that it goes
stale silently, so `crates/fm-app/tests/welcome_note.rs` parses the shipped file, opens a vault
staged exactly as the release does (**including the date rewrite**), and fails if the note names a
control that is gone — three of its claims were already wrong when checked against the code.

**Landed already (the backend half):** `set_identity` is its own `dispatch` command, because
`set_git_remote` sets an identity only alongside a URL and refuses an empty one — so "just my name,
no remote" wrote the identity to disk *and reported failure*. It is in `REMOTE_DENIED`: a paired
tablet does not get to name the host's committer. `ipc.ts` exposes `setIdentity`.

**Built 2026-08-29** (`decisions.md#ui`), to all three conditions: `identity` rides on
`VaultInfo` from `list_vaults` — one local `git config` read beside the `remote_label` spawn that
already happens, never `backup_status`; **Skip for now** saves nothing and is remembered per
browser; the screen never renders where git is absent or unknown; and the identity is saved
first and on its own, so a typo'd remote leaves the name standing and says both halves.
`Welcome.svelte.test.ts` pins the component's promises, `App.welcome.test.ts` the gate.

### 2.6b Features a user asks for are still not *delivered* — only declared
**Closed 2026-08-28** (`decisions.md#agent`): nothing now claims a capability it lacks, and saved
views are writable from the app. What the owner asked for goes further — *"a user picks features and
never thinks about dependencies, and asking for a feature must deliver it working, like any app"* —
and that half is deliberately not built yet.

**The installer.** Fetch what a feature needs into `<app>/program/tools/` — portable, no admin
rights, travels when the folder is copied. Every piece exists and none is wired together:
- `crates/fm-agent-run/src/fetch.rs` — resumable, SHA-256 verified, atomic, progress callback,
  hermetic tests already in `pixi run ci`, **desktop-excluded by one Cargo feature** — and the
  feature is `fm-agent-run/download`, *not* `fm-serve/agent`, which is on in the shipped binary
  (corrected 2026-09-02). `pixi run build` also never builds `fm-agent-run` at all, so wiring the
  fetch means building that crate for the desktop as well as enabling the feature.
- `pixi.lock` already pins poppler, libvips and restic for all four platforms with checksums, so
  installing means fetching *the exact artifact pixi would*.
- `packaging/formicaria.sh` records the proof they are relocatable: run "with PATH alone, resolving
  every shared library (conda binaries carry their own RPATH)".
- The lookup is the missing link: three `available()` functions all use bare `Command::new`, with no
  indirection at all. `fm_core::git`'s `merge_command()` already argues the fix — *never a bare name
  hoping PATH will answer* — for our own `fm` binary; nothing applies it to a third-party tool.

**Delivering a feature is usually three steps, not one.** Encrypted backup needed restic **plus** a
repo location **plus** a password, and only the first of those was ever a capability the app could
see. **Done 2026-08-29** (`decisions.md#vault`): the backup panel takes a repo per vault and one
password per machine. `vaults::save`'s append-only rule was *met, not worked around* — a narrow
`vaults::set_restic` rewrites that one key of that one entry and carries every other byte through,
so a hand-edited list is still safe. The password is `0600` beside the vault list and never in it;
`RESTIC_PASSWORD` still wins, so an edited launcher keeps working.

**The assistant, delivered — closed 2026-09-02** (`decisions.md#agent`). It fetches its own
runtime and model on first enable, on Linux, macOS and Windows, having asked which model and stated
the size and licence; audio transcription does the same on Linux and Windows. The installer this
section wanted therefore exists for one feature — and **not** at `<app>/program/tools/` as proposed
above: that folder is unwritable under `/opt` or `C:\Program Files`, and every update would
re-download gigabytes, so tools live in `<config>/formicaria/tools`. The accepted cost is that a
model does not travel on a USB stick with the app folder. **What remains of this section** is the
other half: `pdftotext`, `vipsthumbnail` and `restic` are still bare `Command::new` lookups with no
installer behind them.

**Smaller, same theme — closed 2026-08-29** (`decisions.md#ui`, *One tag is an arrangement*): the
"save this view" naming step is a real dialog now, not a `window.prompt()`, and it asks the two
questions a prompt could not ask at once. Two further §2.6b failures went with it: `newView` had an
empty default key in an app whose palette was removed, so **"New view" was a labelled capability
with no way to invoke it** — there is a *Save view* button in the pane header now; and a filtered
view could not be produced from the app at all, which is what the optional tag fixes.

**Also closed 2026-08-29** (`decisions.md#data`): the note's property form showed six fixed fields,
so any other frontmatter key was written to the file and then **invisible in the app** — a paper's
`authors`/`year`/`doi` were editable only in a text editor. Every key is now shown and editable,
structural ones read-only, and a hand-typed value is stored with the type the *file* would have
given it (lossless-only inference), so the app and an editor can no longer disagree about whether
`year: 2017` is a number. *(The backup panel's missing token
field is **done 2026-08-29**: an HTTPS remote with no stored credential now asks for one, in the
same words and with the same scope advice as the clone form, and asks for nothing where the remote
is SSH or the helper already holds it.)*

**A thumbnail consumer.** Generation works; nothing requests it (`render.ts`: *"There is no thumbnail
path on any platform"*). Until one exists, previews cannot be offered as a feature at all.

### 2.7 The manual reads for a beginner now — it still has no pictures
**Closed 2026-08-28** (`decisions.md#toolchain`): setup is per-OS and assembled by `ci/docs.sh`, so
each archive carries instructions for its own platform only; `Start here` is two short chapters and
ends by telling the reader they are done; `Going further`/`Reference`/`Developer guide` hold the
depth; and the introduction routes beginners at beginners.

What the 2026-08-28 audit found and this did **not** fix (three usability defects found on
2026-08-29 *were* fixed — see the note at the end of this section):

- **Zero screenshots** in a manual for a GUI. Now the largest remaining gap by some distance, and
  the slowest to close. `docs/context/shots/` proves the capture path exists.
- **`user/notes.md` is 2,245 words unsplit** — it is now the first chapter under *Going further*,
  so it is the first wall a curious beginner hits.
- **No glossary.** *frontmatter*, *ULID*, *content-addressed*, *blob*, *merge driver*, *remote* and
  *FTS* all appear in user chapters undefined.
- ~~**`user/assistant.md` is 100% checkout/pixi/Android-SDK** — unusable from a release, and nothing
  on the page says so.~~ **Said, 2026-08-29:** the page now opens with a block quote stating that
  every step below needs the source, that the release archive carries no assistant, and that it runs
  on Linux and Android only. The content is still checkout-only; the reader is no longer misled
  about who it is for.
- **`user/views.md` documents saved views only as hand-written YAML**, plus a 9-row filter DSL.
- **The sidebar still shows every part at once.** `fold` in `book.toml` is a no-op because no
  chapter has children, so a beginner still sees `Architecture` and `Design system` in the same
  column as their own two chapters — further down now, but present.

**Fixed 2026-08-29, found by an adversarial review of the packaging plan** — all three were
actively misleading a first-time reader, and none was in the audit above:

- **`README.md`'s download link pointed at `singhbal-baljinder/formicaria`** after the move to
  `formicaria-org` — a 404 on the front page's own download button, and step one of the journey. It
  also still told the reader to run `./fm-serve`, which moved into `program/` in August, and
  mentioned an `xattr` command that exists nowhere else. `ci/checks.sh` now guards all three.
- **Every shipped page told macOS users to right-click → Open.** Apple removed that in Sequoia, and
  the dialog a blocked app now shows offers only **Done** and **Move to Trash** — so our own
  instructions steered a first-time Mac user toward deleting the app while looking for the "Open" we
  promised. Rewritten around System Settings → Privacy & Security → *Open Anyway*, with an explicit
  warning, and `ci/checks.sh` fails if the old advice reappears anywhere we ship.
- **`introduction.md` promised PDF search on page one, unconditionally** — false on any machine
  without poppler, which is most fresh Windows and macOS installs.

Also added: what to click for the Windows browser warning (*there is no visible button — click the
grey "More info" text*), **how to start it a second time** (there was a "TO STOP IT" and no
counterpart anywhere in the product), and a real GitHub release body — until now the first prose
every downloader met was a list of commit titles, on the near side of the download where everything
we had written was unreachable.

### 2.8 Windows gets libgit2 — one cost accepted and deferred
**Taken 2026-08-28** (`decisions.md#git`): the libgit2 exception is now scoped to *any device with
no git binary*, so `fm-serve` pulls it through a `[target.'cfg(windows)'.dependencies]` entry.
Windows users get history, backup and collaboration without installing anything; machines that have
git are unchanged, because `vcs.rs` chooses at runtime.

**The deferred cost — paid 2026-08-29, and it was not what this entry predicted.** The v0.2.1
release job failed on Windows exactly as forecast, but the cause was not vendored OpenSSL needing
Perl and NASM. It was that `openssl-sys` **never got the `vendored` feature on Windows at all**:
`git2`'s `vendored-openssl` propagates through `libgit2-sys`, and on Windows that edge does not
exist, because libgit2 there speaks WinHTTP. Meanwhile `fm-core` declared `openssl-sys` and
`libgit2-sys` **unconditionally** — purely for Android's in-memory trust store — so on Windows they
entered the graph as *direct* dependencies with no features, and `openssl-sys`'s build script went
hunting for a system OpenSSL: *"Could not find directory of OpenSSL installation."*

Measured, before and after: `openssl-sys v0.9.117` bare on `x86_64-pc-windows-msvc` against
`openssl-sys v0.9.117 openssl-src,vendored` on `aarch64-linux-android`. Both raw `-sys` crates are
now under `[target.'cfg(not(windows))'.dependencies]`, `add_certs_from_pem` is `#[cfg]`-split with a
Windows arm that returns `Ok(0)` (WinHTTP uses the machine's own store, so there is nothing to add),
and the differential test is gated to match. **`openssl-sys` is now absent from the Windows graph
entirely** — so the Windows build is smaller than planned, and needs neither Perl nor NASM on the
runner. Android's graph is byte-identical.

**Still unverified:** whether the Windows job now *completes*. This fixes the failure that was
observed; it cannot prove the next one does not exist.

### 2.9 A backup with no git reports success and records nothing
**Closed 2026-08-29** (`decisions.md#git`). Root-caused, and it was neither half of what the report
guessed: `git::unrecorded` answers "nothing" for a directory that is **not yet a repository** — it
has to, git cannot say otherwise — while `commit_all` only creates the repository once it is already
running. So the first backup of a new vault swept for orphans *before* there was a repo to be
missing from, found none, then init'd, staged the `.gitignore`/`.gitattributes` it had just written,
and answered `committed: true` with not one note in history. Pinned by
`a_first_backup_on_a_device_with_no_git_binary_creates_the_repository`, on both backends.

The *other* claim did not reproduce and was measured rather than argued: on a Linux build with no
git binary, `ping.git` is `false`, `backup_status.git` is `false`, and `commit` returns
`io error: could not run git (is it installed?)`. Those surfaces were honest already.

### 2.10 Backup says the right things now; three of them it still cannot say
Opened 2026-09-02 out of the review that produced `decisions.md`'s *A backup surface names what its
tier carries*. The wording is fixed; these are the gaps behind it, in the order they are worth
doing.

**A "last backed up at", per vault.** `fm_core::backup::latest()` already returns the newest
`fm`-tagged `Snapshot { id, time, paths }` and **no dispatch command exposes it**, so the one fact a
person actually wants from a backup panel — *when did this last work* — cannot be shown at any
price. The same shape of blindness one level down: `run_backup` returns unit, so even a snapshot
taken thirty seconds ago tells the app nothing about what it contained. A `backup_latest` read
command beside `backup_status` is the small version; folding the timestamp into `VaultStatus` costs
a restic spawn per vault on a call that already polls every 45 s, so it should not go there.

**A restic-only vault cannot run a backup.** `canRun` requires `vaults.some(v => !!v.remote)`, so a
machine with restic, a repo and a password but no git remote has an enabled tick box and a Back up
button that never enables. The panel now says why instead of sitting mute, which is honesty, not a
fix. The fix is to let the snapshot tier run alone — worth doing, and it needs the verdict
sentences to stop assuming a git tier ran at all.

**The dispatch layer for both tiers is untested.** `crates/fm-app/src/dispatch.rs` contains **zero**
`#[test]`, and specifically: nothing asserts `backup_status`'s shape (not `restic_ready`'s three
conditions, not that the password is only ever a bool), nothing drives the `backup` arm's two error
paths (*no repo configured*, *no password*), and nothing covers `set_restic_repo` /
`set_restic_password` / `clear_restic_password` / `restore_vault`. The `fm-core` layer beneath them
is well covered by six real-`restic` tests, and the UI above them by mocks — the seam between is
where nothing looks. `fm backup` / `fm restore` / `fm check` have no CLI test either. Related and
cheaper: **no UI test has ever pressed the Back up button**, so not one step line or verdict
sentence in `BackupPanel.run()` is asserted anywhere.

**And the MASTERPLAN's own acceptance for S6 is not met.** `:429` asks for `restic backup` →
`restic restore` to a scratch dir → **diff against the vault** → `fm verify --scrub` clean.
`fm-core/tests/backup.rs` restores and compares one note's bytes and runs `check --read-data`;
there is no whole-vault diff and no `--scrub` in the restore path. *"Test the restore in month
one"* is the plan's own rule and the suite does not keep it.

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
- **The `.view` renderer set is four, and the fifth says so.** Decided 2026-08-30, having sat here
  as "accepted, do not fix without deciding": `search` now draws — it is the flat, un-bucketed list,
  and `Search.svelte` takes no `query` when a view supplies it. `gallery` is reported as an **error**
  by `list_views` (its renderer was deliberately removed) instead of silently drawing a timeline,
  and `save_view` refuses to author one.
