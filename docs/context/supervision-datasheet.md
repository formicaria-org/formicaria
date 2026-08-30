# Datasheet — the formicaria supervision corpus

_Status: **written before collection**, 2026-08-30. Schema revision **1**. Records collected under
this schema: **0**. Records rescued from before it: **10** (see the plan's Step 0)._

This follows [Datasheets for Datasets](https://arxiv.org/abs/1803.09010)'s seven sections, plus the
[Data Statements](https://aclanthology.org/Q18-1041/) elements that a single-annotator corpus makes
load-bearing. It exists **before** collection deliberately: data statements *"should be developed in
tandem with the datasets themselves and may even inform the curation of datasets"*, and the measured
outcome of the documentation cascade — 20.8% of practitioners, in a study where 92% hit at least one
cascade — is *discarding part or all of the dataset*.

**Treat this as a contract.** If collection drifts from what is written here, this file is the bug
report, not the thing to quietly update.

---

## 1. Motivation

**Purpose.** To record what a human does with an AI proposal — accept it, change it, refuse it, or
ignore it — so that (a) formicaria's own small per-tool models can be improved, and (b) the result
can be published as an open corpus that anyone can train on.

**Created by / for.** The vault owner, for themselves. No funder, no institution, no third party.

**Explicitly not created for** general instruction tuning, factual-knowledge training, or
benchmarking. See §5.

## 2. Composition

**An instance is one review event**: a model proposal, the human's disposition, and the resulting
text — carried as git commits plus message trailers, not as new files.

| | |
|---|---|
| Instances collected under schema 1 | 0 |
| Instances rescued from before it | 10 (2026-07-23/24) |
| Expected rate | **~240/year total**, across all tools |
| Per-model split, observed | 23 `qwen3-4b-2507` · 4 `qwen3-vl-4b` · 3 `lfm2.5-1.2b` |
| The phone model's share | **~24/year** |

**Each instance carries** the outcome (`accepted` / `edited` / `rejected` / `unreviewed`), the tool
and its revision, the model and its pinned revision, the decoding parameters, the rendered prompt,
retrieved sources, an edit distance, and — when the person chose to give one — a sentence in their
own words plus short `@tags`.

**Labels.** `Outcome` comes from the person's own act. `Kind` (style / factual / reasoning) and
`Strength` (ordinal, including an explicit *unsure*) are theirs to set and are optional.

**Information that is missing, and will stay missing unless fixed.**
- The **input** for the research and search tools: queries and retrieved URLs are recorded nowhere
  today, so those records are not trainable as instruction pairs until they are.
- **Audio.** `blobs/` is gitignored, so a speech pair's audio does not travel with the corpus. An
  ASR record without its audio is not a training example.

**Errors, noise, redundancy — stated up front because they are structural, not incidental.**
- **One annotator.** Repeated-item self-inconsistency of roughly a quarter to a third is the
  published expectation, not a defect.
- **Survival measurement is interval-censored and *informatively* censored** — a person who closes
  the note right after accepting a bad edit is censored *because* it was bad, biasing retention
  upward.
- **Acceptance is partly a measurement of the UI**, because accepting is one click and rejecting is
  a two-click confirm.
- **Selection bias**: only outputs that were produced *and reviewed* generate a labelled record.

**Self-contained?** No. It references blobs that git does not carry, and note text that may quote
third parties.

**Confidential or sensitive content.** Yes, by construction — these are personal research notes. A
PII scan and its redaction record are part of every instance; export without it is a defect.

**Identifiability.** The corpus identifies exactly one person: the vault owner. Their writing style
*is* the signal. Anyone reading it can characterise how they write and what they work on.

## 3. Collection process

**Mechanism.** The app records each review event as git commits plus trailers, in the ordinary
course of use. There is no separate pipeline, no daemon, and no upload path.

**Sampling.** Not a sample — a census of reviewed proposals, plus an unlabelled pool of unreviewed
ones, plus a **1–5% suppressed-suggestion holdout** where a proposal is generated but deliberately
not shown, keeping what the person wrote unaided.

**Who, when, compensated how.** One person, their own data, unpaid. Collection begins with the first
release carrying schema 1; the ten rescued records predate it and are tagged accordingly.

**Consent.** Two consents, asked separately: *collect and use locally*, and *publish openly*. The
first does not imply the second. Consent is revocable up to the moment of export by deleting the
retention refs; **after export it is not**, because publication cannot be recalled.

**Ethical review.** None, and none is applicable: a single subject collecting their own data. This
changes the moment a vault is shared — see §7.

## 4. Preprocessing, cleaning, labelling

**The raw is always kept.** Both the model's output and the human's version are stored verbatim, not
as a diff. The corpus is **append-only**: records are deprecated with a reason and dropped from
export, never deleted. Deletion is what produced the Step-0 emergency.

**Computed at write time** (all impossible to backfill honestly later): a near-duplicate signature, a
difficulty score, the PII scan result, and the licence class of the underlying note.

**Every record is stamped with the schema revision and the guideline version in force**, so a later
change to either is a migration rather than a silent mixing of rubrics.

## 5. Uses

**Used for so far:** nothing.

**Intended use:** a per-tool **retrieval** preference memory — a natural-language description of
what this person prefers, retrieved by context. That works at N≈20. A fine-tune is a *hypothesis
with a written threshold*, not a plan.

**This corpus should NOT be used for:**
- **Factual or reasoning training.** It is style-weighted by construction. Style saturates around a
  thousand examples; reasoning shows a monotonically increasing curve this volume cannot feed.
- **Anything requiring annotator diversity.** There is exactly one annotator, and annotator identity
  is worth up to 23 accuracy points on held-out data. Models trained here will fit one person.
- **Benchmarking or evaluating third-party models.** It is derived from a live personal vault;
  contamination status is unknown and unknowable.
- **Inferring anything about anyone other than the vault owner** — including people mentioned in the
  notes, who did not consent to anything.

**Composition effects on future uses.** Every bias in §2 travels with the data. A consumer who reads
only the JSONL and not this file will over-trust the acceptance labels.

## 6. Distribution

**Default: it does not leave the machine.** Retention refs live outside `refs/heads/*`, so a normal
clone or fetch does not carry them. This is deliberate, not an oversight.

**Export is explicit, user-initiated, and produces a file** — JSONL plus a Croissant manifest plus a
CAWG `cawg.training-mining` consent block stating which of training, generative training, inference
and data mining are permitted. There is no automatic egress of any kind, which keeps
`MASTERPLAN.md:63` (*"No telemetry, ever — not even opt-in"*) intact.

**Licence.** Chosen at export. The underlying note text may carry third-party constraints, so each
record records Permitted Use / Attribution / Share-Alike rather than a single licence string —
aggregator licence fields are correct only 35–54% of the time in practice, and skew permissive.

**Irreversibility.** Publication cannot be undone. This is the failure that froze the only
comparable open corpus, and it is why the two consents are separate.

## 7. Maintenance

**Maintainer:** the vault owner. **Erratum:** appended to this file.

**Updates:** continuous, as the app is used. Older records stay valid; they carry their schema
revision and guideline version, so a consumer can filter rather than guess.

**Deprecation:** mark with a reason and stop exporting. Never delete. If real deletion ever becomes
necessary it is an explicit, logged operation.

**Re-anchoring when the model changes.** Base models move — one published case moved 33 points on a
task in three months, and stale labels are *not* repaired by a newer model. On a model change,
re-run the stored prompt: if the new output already satisfies the old correction, retire that record
to the evaluation set; if it fails the same way, keep it; if it fails a new way, keep the corrected
output as a supervised target and drop the preference pair.

**Contributions.** None — this is a single vault. **A shared vault would make the corpus
multi-annotator and would invalidate the single-annotator assumptions throughout this document.**
If that ever happens, this datasheet must be rewritten before the mixed data is exported.

---

## Data Statements elements that a one-person corpus makes load-bearing

**Language variety.** English as written by one person in Singapore (`en-SG`, with `en-GB`
conventions). Note bodies may contain other languages verbatim.

**Annotator demographic.** One person: a domain expert in their own notes, with no training in
linguistics or annotation. **This is a *prescriptive* dataset** — the goal is to capture one
person's intended preference, so disagreeing with oneself is a **bug in the written guideline**, not
a signal to model.

**Speech situation.** Written, asynchronous, self-directed, composed with no audience in mind at the
time of capture — which is precisely why the publication consent is asked separately.

## The quality apparatus this document commits us to

Not aspirations; the corpus is defective without them.

- A **time-based** train/eval split, carved at capture time and frozen. There is no
  disjoint-annotator split available, and k-fold at this scale reports badly inflated numbers.
- **20% of each batch re-reviewed blind after two weeks**, agreement computed **per batch with
  confidence intervals, never cumulatively** (a running average hides late collapse), alerting below
  **α = 0.6**, against a rotating calibration set.
- **20–30 fixed behavioural probes**, versioned, that never change.
- **Three demonstrations at three time points, alternating within a week** — not before-and-after
  across months, because the annotator drifts across months.
- **Every reported number carries its minimum detectable effect.** At N≈200 that is ~10 percentage
  points; anything smaller cannot be claimed from this corpus, and claiming it anyway is the most
  likely way this work becomes untrue.
